// crates/nikaia/src/check/mod.rs
//
// The type checker.
//
// It answers one question per construct - "are these two types the same?" - and
// it answers it only where both sides are written down. That is the whole
// design, and `contracts::ty::Ty::Unknown` is what makes it honest: Stage 0 has
// no signatures for the Rust half of `std`, and a checker that guessed at
// `push_str`, `entry` or `chars` would report errors that are not there. So
// every rule below has the same shape - infer both sides, and report only when
// **both are known and they disagree**.
//
// What it therefore promises, exactly:
//
//   * it never rejects a program that is correct;
//   * what it catches grows as the ledger grows, without this file changing.
//
// The second is the point of building it on the ledger (ADR-020) rather than
// beside it. A program's own functions are checked because `Ledger::infer` read
// their signatures out of the source; `std`'s are checked because
// `std.contracts` writes them down; a package's will be because a package ships
// its ledger (Part III, 13.5). One mechanism, three sources.
//
// Spans are the enclosing statement's, as they are for the `sync` check:
// expression-level spans are open work in the parser, and a caret on the right
// line is worth more than none at all.

pub(crate) mod attributes;
mod core;
mod host;
use crate::contracts::LedgerOps;
use std::collections::{BTreeMap, BTreeSet};

use crate::assets::Reads;
use crate::ast::{self, BinaryOp, Block, Expr, Item, Span, Stmt, UnaryOp};
use crate::build_time;
use crate::contracts::ty::TyOps;
use crate::contracts::{FieldContract, Ledger, ty, ty::Ty};
use crate::fold::Constant;
use crate::parser::Parsed;
use nikaia_std::tools::check_consts;
use nikaia_std::tools::check_core_comptime;
use nikaia_std::tools::check_core_expr;
use nikaia_std::tools::check_core_leaves;
use nikaia_std::tools::check_core_stmt;
use nikaia_std::tools::check_crossings;
use nikaia_std::tools::check_flow::{
    CheckConstant, CheckFlow, CheckUnrolling, CheckUnsignedSlot as UnsignedSlot,
};
use nikaia_std::tools::check_items;
use nikaia_std::tools::check_overflow;
use nikaia_std::tools::check_reports::CheckWall;
use nikaia_std::tools::check_shapes::is_growable;
pub use nikaia_std::tools::check_state::{
    CheckFinding as Finding, CheckLabel as Label, CheckSeverity as Severity,
};
use nikaia_std::tools::check_state::{
    CheckImmutable as Immutable, CheckKind as Kind, CheckLocal as Local, CheckPromises as Promises,
    CheckState, CheckWorld, a_body_never_ends,
};
use nikaia_std::tools::check_texts::{self};
use nikaia_std::tools::check_widen::{self};
use nikaia_std::tools::parsed_view::ParsedView;

/// A finding's code as the `&'static str` the compiler's tables and its tests
/// compare: each code is kept once for the life of the process.
pub trait CodeOps {
    fn code_str(&self) -> &'static str;
}

impl CodeOps for Finding {
    fn code_str(&self) -> &'static str {
        static CODES: std::sync::Mutex<BTreeSet<&'static str>> =
            std::sync::Mutex::new(BTreeSet::new());
        let mut codes = CODES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(kept) = codes.get(self.code.as_str()) {
            return kept;
        }
        let kept: &'static str = Box::leak(self.code.clone().into_boxed_str());
        codes.insert(kept);
        kept
    }
}

/// What one variant declares it carries, by position.
fn payload_of(
    parsed_here: &Parsed,
    beside: &[&Parsed],
    ty: &str,
    variant: &str,
) -> Option<Vec<Ty>> {
    std::iter::once(parsed_here)
        .chain(beside.iter().copied())
        .find_map(|parsed| {
            nikaia_std::tools::check_items::check_payload_in(&parsed_view_of(parsed), ty, variant)
        })
}

use winnow_grammar::Symbol as Ident;

/// What one pass of the checker learned: its findings, the facts the emitter
/// reads (`tools/check_facts.nika`), and what each function calls.
#[derive(Debug, Clone)]
pub struct Checked {
    /// Every mistake it is sure about.
    pub findings: Vec<Finding>,
    /// What the emitter reads, by the id of the node each is about.
    pub facts: CheckFacts,
    /// What each function calls through a method (ADR-288 D2).
    pub methods: BTreeMap<String, MethodCalls>,
    /// What code stored in a function field reaches (ADR-230 D1).
    pub stored_code: BTreeMap<String, StoredCode>,
}

impl Default for Checked {
    fn default() -> Checked {
        Checked {
            findings: Vec::new(),
            facts: CheckFacts::empty(),
            methods: BTreeMap::new(),
            stored_code: BTreeMap::new(),
        }
    }
}

pub use nikaia_std::tools::check_facts::{
    CheckFacts, FactClaim as Claim, FactKeyForm as KeyForm, FactNarrowing as Narrowing,
    FactPausing as PausingEntry, FactViewed as Viewed, FactWrap as Wrap,
};
pub use nikaia_std::tools::check_state::{
    CheckCallsInATask as CallsInATask, CheckMethodCalls as MethodCalls,
    CheckStoredCode as StoredCode,
};

/// Every type mistake the ledgers are enough to see, and every loop that can
/// fail.
///
/// For one file. A program of several (Part I, 9.1) uses [`check_program`],
/// which additionally knows which names are *modules* - and therefore which
/// qualified calls cross a file boundary.
pub fn check(parsed: &Parsed, own: &Ledger, library: &Ledger) -> Checked {
    check_program(parsed, own, library, &BTreeSet::new())
}

/// **What moved in a contract since the committed ledger**, keyed by the
/// callee's name: the errors it newly throws
/// ([ADR-308](../../docs/specification/adr/adr-308.md) D11), and the parameters
/// whose `keeps` changed on a type whose teardown does something
/// ([ADR-297](../../docs/specification/adr/adr-297.md) D12).
///
/// Empty for every build that has nothing to compare — a first build, a loose
/// file, a package whose ledger is not committed yet — which is the honest
/// answer: nothing was written against the old contract, so nothing has
/// changed for anybody.
#[derive(Debug, Clone, Default)]
pub struct Newly {
    /// The errors each callee throws now and did not.
    pub throws: BTreeMap<String, Vec<String>>,
    /// The parameters each callee keeps now and did not, or keeps no longer.
    pub keeps: BTreeMap<String, Vec<Kept>>,
}

/// **A parameter whose `keeps` moved** (ADR-297 D12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    /// The parameter, as the callee's signature names it.
    pub param: String,
    /// `true` where the callee keeps it now and did not; `false` where it kept
    /// it and only reads it now.
    pub now: bool,
    /// The parameter's type, whose teardown does something.
    pub ty: String,
}

impl Newly {
    /// Nothing moved.
    pub fn new() -> Newly {
        Newly::default()
    }
}

/// A map of newly thrown errors is a `Newly` about errors alone.
impl FromIterator<(String, Vec<String>)> for Newly {
    fn from_iter<I: IntoIterator<Item = (String, Vec<String>)>>(items: I) -> Newly {
        Newly {
            throws: items.into_iter().collect(),
            keeps: BTreeMap::new(),
        }
    }
}

/// The same, for one file of a program made of several.
///
/// `modules` is what the program is made of, and it is the whole difference: a
/// qualified call is either into another **module**, where Part I 9.2 says
/// `pub` decides, or into a **type** (`Stats::new`), where it does not. Without
/// the set there is no telling those apart, and a private constructor called in
/// its own file would be reported as a privacy violation.
pub fn check_program(
    parsed: &Parsed,
    own: &Ledger,
    library: &Ledger,
    modules: &BTreeSet<String>,
) -> Checked {
    check_against(
        parsed,
        &[],
        own,
        library,
        modules,
        &Newly::new(),
        &Reads::none(),
    )
}

/// The same, told what a callee has newly gained since the committed ledger
/// ([ADR-308](../../docs/specification/adr/adr-308.md) D11).
///
/// **A second entry point rather than a parameter on the first**, because the
/// answer is a fact about the *build* — two ledgers, one of them on disk — and
/// every other caller of `check_program` is handed one program and no history.
/// An empty map is what they all pass, and an empty map says nothing.
pub fn check_against<'a>(
    parsed: &'a Parsed,
    // **The program's other files**, for the one question that needs a body
    // rather than a contract: a `comptime` calling across a file boundary
    // ([ADR-287](../../docs/specification/adr/adr-287.md) D6). Empty for a
    // caller that has one file, which is every test and every `nikaia lower`
    // outside a project.
    beside: &'a [&'a Parsed],
    own: &'a Ledger,
    library: &'a Ledger,
    modules: &BTreeSet<String>,
    newly: &'a Newly,
    // **What this build may read while it builds**
    // ([ADR-310](../../docs/specification/adr/adr-310.md)). The same shape as
    // `beside`, and for the same reason: it is a fact about the *build* that no
    // ledger can carry, and a caller with nothing to say passes
    // [`assets::Reads::none`], which is D1 — a build given no list reads
    // nothing.
    reads: &'a Reads,
) -> Checked {
    walked(
        parsed, beside, own, library, modules, newly, reads, false, true,
    )
}

/// **The pass the ledger's inference reads**: the same walk, asked for what a
/// call resolves to and not for what a `comptime` comes to
/// ([ADR-321](../../docs/specification/adr/adr-321.md) D1). A value is computed
/// once, by the check that reports it, and compiled there; this pass has no
/// workshop to compile it in, and nothing it hands on reads one.
pub(crate) fn check_for_the_ledger<'a>(
    parsed: &'a Parsed,
    beside: &'a [&'a Parsed],
    own: &'a Ledger,
    library: &'a Ledger,
    reads: &'a Reads,
) -> Checked {
    walked(
        parsed,
        beside,
        own,
        library,
        &BTreeSet::new(),
        &Newly::default(),
        reads,
        false,
        false,
    )
}

/// **Which types the program's other files called this unit's shape walks
/// with** ([ADR-304](../../docs/specification/adr/adr-304.md) D6).
///
/// A walk of `parsed` that stops as soon as the calls have been typed: nothing
/// past the item walk is read, so none of the separate passes below run and no
/// finding is kept. What comes back is one file's contribution to a question
/// about the whole program.
fn instantiations_in<'a>(
    parsed: &'a Parsed,
    beside: &'a [&'a Parsed],
    own: &'a Ledger,
    library: &'a Ledger,
    // **The same module names the asking unit was checked with**, because a
    // name this walk cannot resolve is a call whose argument it cannot type -
    // and an instantiation it misses is a copy nobody writes.
    modules: &BTreeSet<String>,
    reads: &'a Reads,
) -> BTreeMap<(String, String), Vec<FieldContract>> {
    walked(
        parsed,
        beside,
        own,
        library,
        modules,
        &Newly::new(),
        reads,
        true,
        true,
    )
    .facts
    .unrolled
}

/// The walk both entry points above share.
#[allow(clippy::too_many_arguments)]
fn walked<'a>(
    parsed: &'a Parsed,
    beside: &'a [&'a Parsed],
    own: &'a Ledger,
    library: &'a Ledger,
    modules: &BTreeSet<String>,
    newly: &'a Newly,
    reads: &'a Reads,
    harvesting: bool,
    computes: bool,
) -> Checked {
    let world = CheckWorld {
        words: &parsed.interner,
        own,
        library,
        here: parsed_view_of(parsed),
        beside: beside.iter().map(|other| parsed_view_of(other)).collect(),
        here_at: beside
            .iter()
            .position(|other| std::ptr::eq(*other, parsed))
            .map_or(-1, |at| at as i64),
        fields_by_base: check_items::fields_by_base(own, library),
        newly_throws: newly.throws.clone(),
        newly_keeps: newly
            .keeps
            .iter()
            .map(|(callee, all)| {
                let moved = all
                    .iter()
                    .map(|kept| (kept.param.clone(), kept.now, kept.ty.clone()))
                    .collect();
                (callee.clone(), moved)
            })
            .collect(),
    };
    let mut s = core::hollow();
    s.flow = CheckFlow::start(computes, harvesting);
    s.grammars = parsed
        .program
        .items
        .iter()
        .filter_map(|item| match &item.node {
            Item::Grammar(def) => Some((
                parsed.text(def.name).to_string(),
                def.rules
                    .iter()
                    .filter(|r| r.is_entry)
                    .map(|r| parsed.text(r.name).to_string())
                    .collect(),
            )),
            _ => None,
        })
        .collect();
    s.std_in_scope = parsed
        .program
        .items
        .iter()
        .filter_map(|item| match &item.node {
            Item::Import { path, .. } if path.len() == 2 => {
                (parsed.text(path[0]) == "std").then(|| parsed.text(path[1]).to_string())
            }
            _ => None,
        })
        .collect();
    // The distinct prefixes first and the tests after, because there are a
    // few dozen modules and some thousand keys: the test per key built a
    // `format!` and an owned prefix for every one of them.
    s.std_modules = library
        .functions
        .keys()
        .chain(library.types.keys())
        .filter_map(|key| key.split_once("::").map(|(prefix, _)| prefix))
        .collect::<BTreeSet<&str>>()
        .into_iter()
        .filter(|prefix| {
            !library.types.contains_key(*prefix)
                && !library.functions.contains_key(&format!("{prefix}::new"))
        })
        .map(str::to_string)
        .collect();
    s.modules = modules.clone();
    s.opaque_handles = parsed
        .program
        .items
        .iter()
        .filter_map(|item| match &item.node {
            Item::Extern { opaque, .. } => Some(opaque),
            _ => None,
        })
        .flatten()
        .map(|h| parsed.text(h.node.name).to_string())
        .collect();
    s.foreign_names = parsed
        .program
        .items
        .iter()
        .filter_map(|item| match &item.node {
            Item::Extern { declarations, .. } => Some(declarations),
            _ => None,
        })
        .flatten()
        .map(|d| parsed.text(d.node.name).to_string())
        .collect();
    s.dsl_drivers = crate::dsl::drivers(parsed).into_iter().collect();
    let mut checker = Checker {
        parsed,
        beside,
        reads,
        here: beside.iter().position(|other| std::ptr::eq(*other, parsed)),
        world: &world,
        s: core::Baton::new(s),
        spares: Vec::new(),
    };
    checker.collect_types();
    checker.program();
    // **Only the call sites were wanted**, and everything below this line
    // answers a different question at a cost the asking unit has already paid
    // for itself (ADR-304 D6).
    if harvesting {
        return finished(checker);
    }
    // **The calls that stand in the program's other files**, before the turns
    // are walked, because a copy is written by the unit that declares the
    // function and decided at the call - which may be a file away (Part I 9.1).
    checker.instantiations_beside();
    // **And once more per unrolled turn** (ADR-304 D7, ADR-304 D6), which the
    // walk above is what found: a call may stand above the function it names.
    checker.unroll();
    checker.unsigned_slots_proved(parsed);
    checker.s.walls_a_refusal_explains();
    checker.s.facts.walks_fields = checker.s.walks_fields.clone();
    // ADR-296 D5: the DSL parameters a call forgot, and the ones it invented.
    // A separate walk because it answers a question about a *statement's
    // holes* rather than about a type, and it needs no ledger to answer it.
    checker.s.findings.extend(crate::dsl::check(parsed));
    // **Every `assert` outside a test is proved where it can be**
    // ([ADR-269](../../docs/specification/adr/adr-269.md) D4), and checked
    // when the program runs where it cannot. After the walk,
    // because the checker is what decided which calls are the prelude's.
    let keys: BTreeSet<(usize, usize)> = checker
        .s
        .facts
        .claims
        .keys()
        .map(|(a, b)| (*a as usize, *b as usize))
        .collect();
    let comptimes = crate::prove::Comptimes::of(parsed, &checker.s.facts.comptime_values);
    // **A call an open claim reads is run as a `comptime` is**
    // ([ADR-359](../../docs/specification/adr/adr-359.md) D11-D13), where this
    // build has a place to compile it.
    let host = checker.host();
    let runner = |call: &Expr, callee: &str, args: &[i64]| {
        host.run_for_a_claim(call, callee, args, &comptimes.by_name)
    };
    let run: Option<crate::prove::ClaimRunner<'_>> = match reads.workshop().somewhere() {
        true => Some(&runner),
        false => None,
    };
    let proved = crate::prove::prove(parsed, own, library, &keys, &comptimes, run);
    checker.s.findings.extend(proved.findings);
    checker.s.facts.entries = proved.entries;
    checker.s.facts.reaches = proved.reaches;
    checker.s.facts.published = proved.published;
    for (key, held) in proved.held {
        if let Some(claim) = checker
            .s
            .facts
            .claims
            .get_mut(&(key.0 as i64, key.1 as i64))
        {
            claim.held = Some(held);
        }
    }
    // A naked view parameter that is kept past the call (`NK2302`). Also a
    // separate walk, and for the same reason: it asks where a *value* goes
    // rather than what a type is. It reads both ledgers, because **a call says
    // which of its arguments its result may point into** and a parameter that
    // is none of them does not escape through it.
    checker
        .s
        .findings
        .extend(crate::views::check(parsed, own, library));
    // A change in place of data a view still to be used may point into
    // (`NK2305`, ADR-353 D4). A walk of its own, like `views`: it asks where a
    // view points and when it is used, not what a type is.
    let (changed, _) =
        crate::view_changes::analyse(parsed, own, library, &checker.s.facts.lent_lets);
    checker.s.findings.extend(changed);
    // A view handed back that points into a buffer the body owns (`NK2303`).
    // The tether's own refusal
    // ([ADR-283](../../docs/specification/adr/adr-283.md) D9), and a walk of
    // its own for the same reason `views` is: it asks where a *value* points
    // rather than what a type is. It reads both ledgers, because which calls
    // make a buffer is what they say.
    checker
        .s
        .findings
        .extend(crate::contracts::tether::check(parsed, own, library));
    // Kap 4.7: what an `impl` owes the `trait` it names. Separate for a reason
    // of its own - it reads the ledger's **finished** `sync` column, and the
    // type walk runs before `sync::infer` fills that in
    // ([ADR-080](../../docs/specification/adr/adr-080.md)).
    checker.s.findings.extend(crate::traits::check(parsed, own));
    checker.nikaia(|state, world, _| check_crossings::check_crossings_language_impls(state, world));
    // Part I 2.2: a **type** nothing declares, which had nothing where a value
    // has had `NK1117` since ADR-298
    // ([ADR-096](../../docs/specification/adr/adr-096.md)). Separate for the
    // same reason as the three above: it asks about a written *name* rather
    // than about a value's type, so it needs the item tree and neither the
    // scope stack nor the inference.
    checker
        .s
        .findings
        .extend(crate::types::check(parsed, own, library));
    // **What no function body held**, typed the same way (ADR-285).
    checker.numbers_typed_by_their_uses();
    // **Arms that disagree, where no position asked them** (#596).
    let disagreeing = std::mem::take(&mut checker.s.arm_disagreements);
    checker.s.findings.extend(disagreeing.into_values());
    // **`NK1245` comes after every other refusal** (ADR-286 D41): a type
    // nothing can tell is most often what another refusal left behind - a
    // name never declared, a member a type does not have, a body with a jump
    // that goes nowhere - and the value it would have made is then unknown
    // wherever it is used, lines away. Where the file holds another error,
    // that error is the one said; what is still unknown once it is fixed is
    // said then.
    if checker
        .s
        .findings
        .iter()
        .any(|f| f.severity == Severity::Error && f.code != "NK1245")
    {
        checker.s.findings.retain(|f| f.code != "NK1245");
    }
    checker.s.findings.sort_by_key(|f| f.span.at());
    // **`a` or `an` by the word that follows** (#511): forty messages write
    // `a `{ty}`` by hand, and the type is known only when they are said - `a
    // `io::IoError``. Corrected once, here, by the rule `an` follows.
    for finding in &mut checker.s.findings {
        finding.message = articles(&finding.message);
        for note in &mut finding.notes {
            *note = articles(note);
        }
        if let Some(help) = &mut finding.help {
            *help = articles(help);
        }
        for label in &mut finding.labels {
            label.text = articles(&label.text);
        }
    }
    // Only the calls that provably fail, and only where the name is not also a
    // call that does not: the emitter writes a `?` for each of these, and a `?`
    // on something that is not a failure is a `rustc` error about a file the
    // author never wrote.
    checker.s.facts.fallible_methods = checker
        .s
        .flow
        .fallible_methods
        .difference(&checker.s.flow.opaque_methods)
        .cloned()
        .collect();
    // The same subtraction one column over: an `.await` on something that is not
    // a future is the same kind of message about the same file nobody wrote.
    checker.s.facts.pausing_methods = checker
        .s
        .flow
        .pausing_methods
        .difference(&checker.s.flow.settled_methods)
        .cloned()
        .collect();
    // The same subtraction, for the same reason: a checked conversion written on
    // a widening one does not compile.
    let state = &mut *checker.s;
    state
        .facts
        .narrowing_casts
        .retain(|at, _| !state.flow.widening_casts.contains(at));
    finished(checker)
}

/// What the walk found: the state's findings go into the table the emitter reads.
fn finished(checker: Checker<'_>) -> Checked {
    let state = checker.s.into_state();
    Checked {
        findings: state.findings,
        facts: state.facts,
        methods: state.methods,
        stored_code: state.stored_code,
    }
}

/// Everything the emitter needs that only a type checker can answer: the loops
/// whose step can fail, the method calls that can fail, and the places where a
/// declared `Shared[T]` makes the first handle.
///
/// All three together because all three come out of one pass, and a second pass
/// Which wrap a value standing in `want` needs, and `None` where it needs none.
///
/// One function for all four of Part I 2.3's positions, so the rule is stated
/// once: the annotated `let`, the assignment, the `return`, and a struct
/// literal's field — and a call's argument, which is the fourth written a fifth
/// way. Before this they were four copies of the same three conditions, and the
/// copies had already drifted in what they did with an unknown type.
/// **Which of a statement's calls an argument belongs to**, as text both the
/// checker and the emitter can produce from the same tree.
///
/// A key of statement, callee and position names a *parameter*; a statement may
/// call one function more than once, and the argument itself is what tells the
/// two calls apart. The alternatives were weighed and each fails on something
/// this does not:
///
///   * **the argument's span** - an argument has none. One node type does:
///     [ADR-081](../../docs/specification/adr/adr-081.md) D1 gave `Expr::Binary`
///     a span of its own, for a key of exactly this kind. It is one node and an
///     argument is rarely that one, so there is nothing general to key by -
///     which is the reason this whole side table exists;
///   * **the address of the `Expr`** - stable for a statement walked twice, and
///     not for an `f"…"` hole, which is *text* until each pass parses it into a
///     tree of its own ([`crate::emit::literal_expressions`]);
///   * **the order the calls are walked in** - which would make the two passes
///     agree by assumption rather than by construction.
///
/// So it is structural: the same expression, from either pass, renders the
/// same. The interner is shared, so an identifier renders as the same `Symbol`
/// on both sides - including in a re-parsed hole, which is parsed with that
/// interner.
///
/// **Two textually identical arguments share an entry, and want the same
/// answer**: the wrap is a function of the argument's type and the parameter's,
/// and within one statement the same expression in the same position has the
/// same type.
///
/// It is built only where a wrap was recorded for this statement, callee and
/// position - which is rare - so the cost is not on the path every argument
/// takes.
/// Whether an expression **can only** be a value: a name, a literal, an
/// operator, a field, an index.
///
/// Whether this expression **leaves** rather than coming to a value
/// ([ADR-276](../../docs/specification/adr/adr-276.md) D20).
///
/// Its type is *never*, which is not a type this checker's `Ty` spells: what
/// never means here is *this is not one of the answers that have to agree*, and
/// that is a question about the expression rather than about a type. A block
/// counts where its last statement is one of the four, which is the shape an
/// arm written `=> { throw NotFound }` still has.
impl Checker<'_> {}

/// What a computed build-time value stands for, which its refusal names.
#[derive(Clone, Copy)]
pub(crate) enum Computing {
    Option,
    Field,
    Attribute,
}

/// An integer type's name as a name that lives for the program.
fn a_static_integer(name: &str) -> &'static str {
    INTEGERS
        .iter()
        .copied()
        .find(|n| *n == name)
        .unwrap_or("i64")
}

/// **The integer types a program writes** (Part I 2.2), as the type names
/// they are.
const INTEGERS: [&str; 5] = ["u8", "u32", "u64", "i32", "i64"];

/// **The lossless common type of two integer types** (`tools/check_widen.nika`).
pub fn common_integer(a: &str, b: &str) -> Option<&'static str> {
    check_widen::common_integer(a, b).map(|common| a_static_integer(&common))
}

/// The loops whose step can fail, for a caller that wants only those.
///
/// The emitter's entry point: it builds the ledgers a program is compiled
/// against and asks this, rather than carrying the checker's findings around.
pub fn fallible_loops(parsed: &Parsed) -> BTreeSet<i64> {
    fallible_loops_against(parsed, &Ledger::infer(parsed))
}

/// The same, against contracts the caller already has - which for a program of
/// several files is the **program's** ledger and not this file's (Part I, 9.1).
pub fn fallible_loops_against(parsed: &Parsed, own: &Ledger) -> BTreeSet<i64> {
    propagation_against(parsed, &[], own, &Ledger::empty(), &Reads::none()).fallible_loops
}

/// **What a `T::fields` loop was unrolled to, for the types actually used**
/// ([ADR-304](../../docs/specification/adr/adr-304.md) D9, built as
/// [ADR-304](../../docs/specification/adr/adr-304.md) D10's `--comptime`).
///
/// Every build-time system shares one readability problem: you cannot see what
/// a function becomes for a given type without unrolling it in your head. The
/// usual answer is to invent syntax; this project already has the other one,
/// and `--overlaps`, `--sharing`, `--tethers` and `--trust` are it. So this is
/// the **same information** [ADR-304](../../docs/specification/adr/adr-304.md)
/// D3's diagnostic carries, offered on demand instead of on failure.
///
/// **A function that walks a shape and was never called is printed too**, with
/// its own line. It is the one thing a reader could not otherwise find out: no
/// copy is emitted for it at all, so nothing in the generated file says it
/// exists.
///
/// The whole program at once, because an instantiation is a fact about a
/// **call** and a call may stand in a different file from the function it names.
pub fn unrolling_report(beside: &[&Parsed], own: &Ledger, reads: &Reads) -> String {
    // **Every unit and not just the first**, because the two halves of one
    // line may stand in three different files: the function is declared in
    // one, called from a second, and the report is asked for by a build that
    // has a third. A report built from one file would say *nothing calls it*
    // about a function called twice.
    let mut walks_fields: BTreeMap<String, String> = BTreeMap::new();
    let mut unrolled: BTreeMap<(String, String), Vec<FieldContract>> = BTreeMap::new();
    for parsed in beside {
        let found = propagation_against(parsed, beside, own, &Ledger::empty(), reads);
        walks_fields.extend(found.walks_fields);
        unrolled.extend(found.unrolled);
    }
    let mut found = CheckFacts::empty();
    found.walks_fields = walks_fields;
    found.unrolled = unrolled;
    let mut out = String::new();
    for (function, parameter) in &found.walks_fields {
        let copies: Vec<(&String, &Vec<FieldContract>)> = found
            .unrolled
            .iter()
            .filter(|((written, _), _)| written == function)
            .map(|((_, on), fields)| (on, fields))
            .collect();
        if copies.is_empty() {
            out.push_str(&format!(
                "comptime: `{function}` walks `{parameter}::fields` and nothing calls it, \
                 so no copy is written\n"
            ));
            continue;
        }
        for (on, fields) in copies {
            out.push_str(&format!(
                "comptime: `{function}` unrolled over `{on}` as `{}`\n",
                specialised(function, on)
            ));
            for field in fields {
                out.push_str(&format!("    {}: {}\n", field.name, field.ty.text()));
            }
        }
    }
    out
}

/// Both halves of ADR-023 D8's propagation, against contracts the caller
/// already has.
pub fn propagation_against(
    parsed: &Parsed,
    beside: &[&Parsed],
    own: &Ledger,
    described: &Ledger,
    reads: &Reads,
) -> CheckFacts {
    propagation(parsed, beside, own, described, reads, true)
}

/// **The same, with build-time values left alone**: for a caller that asks what
/// the program uses and not what it computes (`emit::Needs::of`).
pub(crate) fn propagation_leaving_values(
    parsed: &Parsed,
    beside: &[&Parsed],
    own: &Ledger,
    described: &Ledger,
    reads: &Reads,
) -> CheckFacts {
    propagation(parsed, beside, own, described, reads, false)
}

fn propagation(
    parsed: &Parsed,
    beside: &[&Parsed],
    own: &Ledger,
    described: &Ledger,
    reads: &Reads,
    computes: bool,
) -> CheckFacts {
    let mut library = crate::contracts::std_library();
    // **And the described crates' boundary**, as the check that refused had it
    // (`project::Foreign::library`, `std` winning a collision): what a call
    // into `hyper_shim` keeps decides how its argument is written, and a walk
    // that could not resolve the callee lowered a literal it had been told to
    // build as text of its own (0.0.230, issue #155 before it).
    for (name, contract) in &described.functions {
        library
            .functions
            .entry(name.clone())
            .or_insert_with(|| contract.clone());
    }
    for (name, contract) in &described.types {
        library
            .types
            .entry(name.clone())
            .or_insert_with(|| contract.clone());
    }
    // **The same walk the refusal ran**, `beside` included: a `comptime` that
    // calls across a file boundary is answered by the checker, and what the
    // emitter writes is that answer. Handing it fewer files than the check had
    // would make the emitter refuse an item the checker accepted, which is the
    // two halves disagreeing about one program.
    // **And the same reads**, for the reason `beside` is here: a `comptime`
    // that reads a file is answered by the checker and what the emitter writes
    // is that answer, so a walk handed a different allowlist would refuse an
    // item the check accepted ([ADR-310](../../docs/specification/adr/adr-310.md)).
    let checked = walked(
        parsed,
        beside,
        own,
        &library,
        &BTreeSet::new(),
        &Newly::new(),
        reads,
        false,
        computes,
    );
    checked.facts
}

/// **Whether this body writes `P::fields` anywhere in it**
/// ([ADR-304](../../docs/specification/adr/adr-304.md) D4).
///
/// The **body** and not the bound, because a bound says only that a shape may
/// be asked for: a `[T: Struct]` function that never asks is an ordinary
/// generic one and stays generic in the language below.
///
/// `crate::emit::visit_block` rather than a walk of this file's own, for the
/// reason that function is `pub(crate)` at all: a second walk over one shape is
/// a second thing to keep in step with the AST.
fn walks_the_fields_of(
    parsed: &Parsed,
    body: &crate::ast::Block,
    parameter: &str,
    shape: &str,
) -> bool {
    let mut found = false;
    crate::emit::visit_block(body, &mut |expr| {
        if let Expr::Path(segments, _) = expr {
            let names: Vec<&str> = segments.iter().map(|s| parsed.text(*s)).collect();
            // **`T::attribute(X)` asks for the type too** (ADR-331 D6): its
            // value is the type's own, so the function is copied per type as
            // a walk is.
            if matches!(names.as_slice(), [ty, member]
                if *ty == parameter
                    && (*member == shape || *member == "attribute" || *member == "attributes"))
            {
                found = true;
            }
        }
    });
    found
}

/// **The name of one specialised copy** — `describe__User`
/// ([ADR-304](../../docs/specification/adr/adr-304.md) D6).
///
/// `pub(crate)` because the emitter writes the same name at the call and at the
/// definition, and two spellings of one name is the defect that arrangement
/// exists to prevent. A double underscore because a `.nika` name may hold one
/// and this is not a name a program can collide with by accident: `describe__User`
/// would have to be **written** to clash, and `NK1148` catches that.
///
/// **A dependency's type is named by its path**, `lib::User`, and `::` has no
/// place in a function's name below: `describe__lib__User`.
pub(crate) fn specialised(function: &str, on: &str) -> String {
    nikaia_std::tools::check_core_pilot::check_specialised(function, on)
}

/// The value as the author wrote it, where this compiler can say so: a literal,
/// a name, a negation, and an ellipsis for the rest. `CheckHost::written`.
pub(super) fn written_by(parsed: &Parsed, value: &Expr) -> String {
    match value {
        Expr::LitInt {
            value, negative, ..
        } => crate::ast::int_value(*value, *negative).to_string(),
        Expr::LitBool(yes, _) => yes.to_string(),
        Expr::LitStr { text, .. } => format!("{text:?}"),
        Expr::Variable(name, _) => parsed.text(*name).to_string(),
        Expr::Unary {
            op: UnaryOp::Neg,
            expr,
            ..
        } => format!("-{}", written_by(parsed, expr)),
        _ => "…".to_string(),
    }
}

/// The members Part II 10.3 writes: a `struct`'s fields under `[T: Struct]`,
/// an `enum`'s variants under `[T: Enum]`.
const FIELDS: &str = "fields";
const VARIANTS: &str = "variants";

/// The member a shape bound makes reachable, and the type of one element of it.
fn shape_member(bound: &str) -> Option<(&'static str, &'static str)> {
    match bound {
        "Struct" => Some((FIELDS, ty::FIELD)),
        "Enum" => Some((VARIANTS, ty::VARIANT)),
        _ => None,
    }
}

impl From<Constant> for CheckConstant {
    fn from(c: Constant) -> CheckConstant {
        CheckConstant {
            value: c.value,
            beyond: c.beyond,
            pinned: c.pinned,
        }
    }
}

impl From<CheckConstant> for Constant {
    fn from(c: CheckConstant) -> Constant {
        Constant {
            value: c.value,
            beyond: c.beyond,
            pinned: c.pinned,
        }
    }
}

struct Checker<'a> {
    /// **The checker's state that is Nikaia** (`tools/check_state.nika`, #558).
    s: core::Baton,
    /// What it looks at and does not own: the names, and the two ledgers. A
    /// reference, so the moved code can be handed it while the checker is lent.
    world: &'a CheckWorld<'a, 'a, 'a, 'a, 'a>,
    /// States with nothing in them, for the crossings into Nikaia (`core.rs`).
    spares: Vec<Box<CheckState>>,
    parsed: &'a Parsed,
    /// **The program's other files**, for the one question that needs a body
    /// rather than a contract — a `comptime` calling across a file boundary.
    ///
    /// Empty for a caller that has one file, which is every test and every
    /// `nikaia lower` outside a project. It carries `Parsed` and not just items,
    /// because each one owns the interner its symbols resolve in.
    beside: &'a [&'a Parsed],
    /// **What this build may read while it builds** (ADR-310), carried beside
    /// the files for the same reason: no ledger can say it.
    reads: &'a Reads,
    /// Where this file stands among `beside`, where it does.
    here: Option<usize>,
}

impl<'a> Checker<'a> {
    // --- the shape of a program ---------------------------------------------

    /// What the Nikaia half asks of the compiler (`tools/check_host.nika`).
    fn host(&self) -> host::Host<'a> {
        host::Host {
            parsed: self.parsed,
            beside: self.beside,
            here: self.here,
            reads: self.reads,
            own: self.world.own,
            library: self.world.library,
        }
    }

    fn collect_types(&mut self) {
        self.s.collect_types(self.world);
        self.collect_field_walks();
        self.collect_comparisons();
        self.collect_copies();
    }

    /// The other files of the program: `beside` without this one.
    fn others(&self) -> impl Iterator<Item = &'a Parsed> + use<'a> {
        let (beside, here) = (self.beside, self.here);
        beside
            .iter()
            .enumerate()
            .filter(move |(at, _)| Some(*at) != here)
            .map(|(_, other)| *other)
    }

    /// **What each type another file of the package declares is made of**
    /// (ADR-286 D1), read off its declaration: the fields of a struct, every
    /// payload of an enum's variants.
    fn parts_beside(&self) -> BTreeMap<String, Vec<Ty>> {
        nikaia_std::tools::check_copies::check_parts_beside(self.world)
    }

    /// **Which declared types derive `Copy`**
    /// ([ADR-294](../../docs/specification/adr/adr-294.md) D9.1,
    /// [`Checked::copies`]).
    ///
    /// The least answer that holds: a type joins once every part of it is a
    /// copy, asked again until nothing more joins. So a type that holds itself
    /// never does - it holds itself through a box the compiler wrote
    /// ([ADR-246](../../docs/specification/adr/adr-246.md)), and a box is not
    /// a copy - where [`Comparable`]'s walk, which answers *yes* on a cycle,
    /// would have said it was.
    fn collect_copies(&mut self) {
        self.s.collect_copies(self.world);
    }

    /// **Which declared types compare**
    /// ([ADR-204](../../docs/specification/adr/adr-204.md) D1), for the derive
    /// the emitter writes and the refusal this file raises.
    ///
    /// After every type is collected, because the answer is structural: a
    /// `struct` compares when its fields do, and a field may name a type
    /// declared further down the file.
    fn collect_comparisons(&mut self) {
        // **And what another file of the package declares** (ADR-286 D1).
        let beside = self.parts_beside();
        let asking = Comparable {
            beside: &beside,
            structs: &self.s.structs,
            enums: &self.s.enums,
            payloads: &self.s.enum_payloads,
            parameters: &self.s.struct_parameters,
            own: self.world.own,
            library: self.world.library,
        };
        let found = nikaia_std::tools::check_compare::compare_declared(&asking, &self.world.here);
        self.s.facts.compares.extend(found.compares);
        self.s.facts.compares_totally.extend(found.totally);
    }

    /// **Which functions walk a type's fields, and which parameter each walks**
    /// ([ADR-304](../../docs/specification/adr/adr-304.md) D4).
    ///
    /// Read off the **body** rather than off the bound, because the bound says
    /// only that a shape *may* be asked for: `fn tell[T: Struct](v: T)` that
    /// never writes `T::fields` is an ordinary generic function and stays one,
    /// generic in the language below and emitted once.
    ///
    /// Collected before any body is walked, because a call may stand **above**
    /// the function it names — items are order-independent here — and what a
    /// call has to do about one of these is decided at the call.
    ///
    /// **Over the program's other files too**, for the same reason one file
    /// over: the files of a package share one namespace (Part I 9.1), so
    /// `describe` may be declared in `shapes.nika` and called from `main.nika`
    /// — and a call that did not know it names a shape walk would be left
    /// pointing at a generic original nobody emits.
    fn collect_field_walks(&mut self) {
        self.field_walks_in(self.parsed);
        for other in self.beside {
            self.field_walks_in(other);
        }
    }

    /// One file's shape walks, under the interner its symbols resolve in.
    fn field_walks_in(&mut self, parsed: &Parsed) {
        for item in &parsed.program.items {
            let Item::Fn {
                name: Some(name),
                generics,
                body,
                ..
            } = &item.node
            else {
                continue;
            };
            let walked = generics.iter().find_map(|g| {
                let parameter = parsed.text(g.name).to_string();
                let asked = g.bounds.iter().any(|b| {
                    shape_member(parsed.text(*b)).is_some_and(|(member, _)| {
                        walks_the_fields_of(parsed, body, &parameter, member)
                    })
                });
                asked.then_some(parameter)
            });
            if let Some(parameter) = walked {
                self.s
                    .walks_fields
                    .insert(parsed.text(*name).to_string(), parameter);
            }
        }
    }

    fn program(&mut self) {
        // **A frame under every body, filled before any of them is walked**
        // ([ADR-287](../../../docs/specification/adr/adr-287.md)). A `comptime`
        // at item level is in scope for the whole file, and this checker's
        // scope is a stack pushed per function — so the name has to be there
        // before the first `fn` is entered, and it has to be there for a
        // function declared *above* the constant as much as below it. That is
        // the whole of why this is a pass and not an arm.
        self.item_constants();
        self.types_that_contain_themselves();
        // **Attributes** (ADR-331), before the bodies: they are about the
        // declarations.
        self.attributes();
        for item in &self.parsed.program.items {
            match &item.node {
                // Walked by `item_constants` above, in a frame that stays.
                Item::Comptime { .. } => {}
                // **`NK1227`: the convention is not written**
                // ([ADR-324](../../docs/specification/adr/adr-324.md) D6).
                Item::Extern { abi: Some(abi), .. } => {
                    self.s.findings.push(Finding {
                        severity: Severity::Error,
                        span: item.span,
                        code: "NK1227".to_string(),
                        message: format!("`extern` takes no convention, and this names `{abi:?}`."),
                        notes: vec![
                            "`extern` is the C calling convention, the only one on every target, \
                             so it is not written."
                                .to_string(),
                        ],
                        help: Some(
                            "Write `extern { … }`, or `extern(library: \"name\") { … }` for a \
                             block whose functions come from a system library."
                                .to_string(),
                        ),
                        labels: Vec::new(),
                    });
                }
                Item::Fn { .. } => {
                    self.an_entry_point(&item.node, &item.span, false);
                    // **`main`'s parameters are the command line**
                    // ([ADR-360](../../docs/specification/adr/adr-360.md) D2).
                    self.s.main_parameters(self.world, &item.node, &item.span);
                    self.function(&item.node, None)
                }
                Item::Struct { fields, .. } => {
                    for field in fields {
                        self.a_field_default(field);
                    }
                    self.an_extern_struct(&item.node, &item.span);
                }
                Item::Enum { .. } => self.an_extern_enum(&item.node, &item.span),
                Item::Impl {
                    target,
                    methods,
                    trait_name,
                } => {
                    for method in methods {
                        self.an_entry_point(&method.node, &method.span, trait_name.is_some());
                    }
                    if trait_name.is_some_and(|t| self.parsed.text(t) == "Cleanup") {
                        self.s.a_cleanup_shaped(
                            self.world,
                            self.parsed.text(target.name),
                            methods,
                            &item.span,
                        );
                    }
                    if trait_name.is_some_and(|t| self.parsed.text(t) == "Fatal") {
                        self.s.a_fatal_type_that_is_no_error(
                            self.world,
                            self.parsed.text(target.name),
                            &item.span,
                        );
                    }
                    if let Some(trait_name) = trait_name {
                        let written = self.parsed.text(*trait_name).to_string();
                        self.a_trait_nothing_declares(&written, &item.span);
                    }
                    // `impl Stack[T]` puts `T` in scope for every method in it,
                    // exactly as the ledger reads it (`Ledger::of`), so a method
                    // body sees the same names its signature was recorded with -
                    // one rule, called from both (`contracts::impl_parameters`).
                    let declared = crate::contracts::declared_types(self.parsed);
                    // An `impl`'s own parameters carry no bounds: the head
                    // writes `impl Stack[T]` and there is nowhere in it for a
                    // `: Summarize` to stand
                    // ([ADR-295](../../docs/specification/adr/adr-295.md)).
                    let outer: BTreeMap<String, Vec<String>> =
                        crate::contracts::impl_parameters(self.parsed, target, &declared)
                            .into_iter()
                            .map(|name| (name, Vec::new()))
                            .collect();
                    // **The head's own brackets, as types.** `impl Holder[T]`
                    // writes a parameter and `impl Holder[i64]` writes a type,
                    // and `Ty::from_ast` needs to know nothing about which: a
                    // parameter's in-body representation *is* its name as a
                    // type (`Ty::Named { name: "T" }`), which is what
                    // `type_parameters` above is keyed by and what `NK1126`
                    // reads. So one line answers both, and the receiver below
                    // binds the struct's parameters the way any other value of
                    // that type does (ADR-295 D2).
                    let arguments: Vec<Ty> = target
                        .generics
                        .iter()
                        .map(|g| Ty::from_ast(self.parsed, g))
                        .collect();
                    let target = self.parsed.text(target.name).to_string();
                    for method in methods {
                        self.s.enclosing = outer.clone();
                        self.s.subject_arguments = arguments.clone();
                        self.function(&method.node, Some(&target));
                    }
                    self.s.enclosing = BTreeMap::new();
                    self.s.subject_arguments = Vec::new();
                }
                // A test and a bench are code, and nothing about them is
                // exempt from the language's rules (Part III, 14.1 and 13.4).
                // Nothing produces these yet - the items are in the AST and the
                // grammar has no rule for either - so this is what stops their
                // bodies from arriving unchecked on the day it does.
                Item::Test { body, .. } | Item::Bench { body, .. } => {
                    let outer = self.s.expected.take();
                    self.s.scope.push(Vec::new());
                    self.block(body);
                    self.s.scope.pop();
                    self.s.expected = outer;
                }
                Item::Grammar(grammar) => self.grammar(grammar),
                // **A `use` brings no name in, for `std` as for a package**
                // ([ADR-140](../../docs/specification/adr/adr-140.md) D5).
                Item::Import { path, .. } => self
                    .s
                    .an_import_that_brings_a_name_in(self.world, path, &item.span),
                _ => {}
            }
        }
    }

    fn grammar(&mut self, grammar: &ast::GrammarDef) {
        let named = self.parsed.text(grammar.name).to_string();
        self.s.rule_results = grammar
            .rules
            .iter()
            .filter_map(|rule| {
                let ty = Ty::from_ast(self.parsed, rule.ret_type.as_ref()?);
                Some((self.parsed.text(rule.name).to_string(), ty))
            })
            .collect();
        for rule in &grammar.rules {
            for alt in &rule.alts {
                self.s
                    .a_name_the_page_does_not_have(self.world, &alt.pattern);
            }
        }
        for rule in &grammar.rules {
            let expected = rule.ret_type.as_ref().map(|t| Ty::from_ast(self.parsed, t));
            // **The rule's own key, so its answers land in its own entry.**
            // A `entry` rule *is* a ledger entry
            // ([ADR-296](../../docs/specification/adr/adr-296.md) D24), and the
            // walks that derive `touches` and `locks` read the method answers
            // this checker files under the caller's key — which was `None` for
            // an action, so a grammar's entry inherited nothing and every
            // caller inherited that
            // ([ADR-186](../../docs/specification/adr/adr-186.md)). What the key
            // must **not** do is make an action a *function*, and `NK2605` is
            // where that is said instead: an action's failure leaves the parser
            // rather than travelling to a caller.
            let outer_current = self
                .s
                .current
                .replace(format!("{named}::{}", self.parsed.text(rule.name)));
            for alt in &rule.alts {
                let rules: BTreeSet<String> = grammar
                    .rules
                    .iter()
                    .map(|r| self.parsed.text(r.name).to_string())
                    .collect();
                let frame = self.s.bindings_of(self.world, &alt.pattern.node, &rules);
                // **A fold's lambdas first, and in the pattern's own frame**
                // ([ADR-296](../../../docs/specification/adr/adr-296.md)),
                // because a fold may stand beside a binding in a sequence:
                // `head:N rest:fold(N, zero, fn(acc, m) { acc + m + head })`
                // parses, and without this `head` is refused. And outside
                // `expected`, which is the rule's declared type - what a fold's
                // `init` and `step` build is the parser backend's arithmetic on
                // the way to that type, not the type itself.
                // **An action may not pause**
                // ([ADR-296](../../docs/specification/adr/adr-296.md) D35), and
                // a fold's `init`, `step` and `merge` are action code too
                // ([ADR-296](../../docs/specification/adr/adr-296.md)) - so the
                // flag is set around both rather than around the block alone.
                let named = self.parsed.text(rule.name).to_string();
                let outer_action = self.s.inside_an_action.replace(named);
                self.s.scope.push(frame.iter().map(Local::clone).collect());
                self.folds_in(grammar, &alt.pattern.node, &alt.pattern.span);
                self.s.scope.pop();
                let Some(action) = &alt.action else {
                    self.s.inside_an_action = outer_action;
                    continue;
                };
                let outer = std::mem::replace(&mut self.s.expected, expected.clone());
                self.s.scope.push(frame);
                let tail_span = action.stmts.last().map(|s| s.span);
                let tail = self.block(action);
                self.s.scope.pop();
                self.s.inside_an_action = outer_action;
                if let (Some(expected), Some(span)) = (&expected, tail_span) {
                    self.expect(&tail, expected, span, "returns", |found, want| {
                        format!("This action builds `{found}`, but its rule returns `{want}`.")
                    });
                }
                self.s.expected = outer;
            }
            self.s.current = outer_current;
        }
    }

    /// **A `fold`'s `init`, `step` and `merge`, walked like any other code**
    /// ([ADR-296](../../../docs/specification/adr/adr-296.md)).
    ///
    /// They are expressions inside a **pattern**, and the walk above takes a
    /// rule's *action block* and nothing else - so
    /// `fn(acc, m) { nothing_declares_this }` lowered without a word and the
    /// message was `rustc`'s about a file nobody wrote (Part III, C.1).
    ///
    /// [ADR-276](../../../docs/specification/adr/adr-276.md) D16 closed the half
    /// a **jump** can reach and deliberately no more, with its own walk of
    /// these same expressions. This subsumes that walk rather than standing
    /// beside it: [`Expr::Closure`]'s arm already crosses a boundary and counts
    /// loops from zero, so a jump here meets `NK1132` through the path every
    /// other lambda's does - and gets the better of the two messages, naming
    /// the lambda instead of saying there is no loop.
    fn folds_in(&mut self, grammar: &ast::GrammarDef, pattern: &ast::Pattern, span: &Span) {
        match pattern {
            ast::Pattern::Fold(spec) => self.fold(grammar, spec, span),
            ast::Pattern::Seq(parts) | ast::Pattern::Choice(parts) => {
                for part in parts {
                    self.folds_in(grammar, &part.node, &part.span);
                }
            }
            ast::Pattern::Bind { pat, .. }
            | ast::Pattern::Repeat { pat, .. }
            | ast::Pattern::Group(pat) => self.folds_in(grammar, &pat.node, &pat.span),
            ast::Pattern::Ref { args, .. } => {
                for arg in args {
                    self.folds_in(grammar, &arg.node, &arg.span);
                }
            }
            ast::Pattern::Literal(_) | ast::Pattern::Cut => {}
        }
    }

    /// **A fold's `init`, `step` and `merge`, typed** (0.0.230, issue #160
    /// before it).
    ///
    /// The step is handed the accumulator `init` builds and one item of the
    /// rule it folds, so its parameters are typed from those two: without that,
    /// `fn(acc, m) { acc.record(m) }` resolved `record` to nothing and a
    /// pausing one compiled - into a future the fold dropped, so the parse
    /// counted nothing. Typed, the call meets `NK2209` like any action's.
    ///
    /// A `merge` is handed two accumulators. An `init` or a `merge` written as
    /// a function's name is a call the fold makes, so it is asked the same
    /// question of the function it names.
    fn fold(&mut self, grammar: &ast::GrammarDef, spec: &ast::FoldSpec, span: &Span) {
        let item = grammar
            .rules
            .iter()
            .find(|rule| rule.name == spec.rule)
            .and_then(|rule| rule.ret_type.as_ref())
            .map(|t| Ty::from_ast(self.parsed, t))
            .unwrap_or(Ty::Unknown);
        let named = self.a_named_fold_part_that_may_pause(&spec.init, span);
        let built = self.expr(&spec.init, span);
        // A function's name - `Summary` is its constructor, `Summary::new` -
        // builds what its signature hands back; anything else is the value.
        let acc = match (named, built) {
            (Some(result), _) => result,
            (None, Ty::Fn { result, .. }) => (*result).unwrap_or(Ty::Unknown),
            (None, other) => other,
        };
        // **`NK2209` is the rule here and not `NK2206`**: no parameter type
        // says `sync` about this lambda, the language does, and the action
        // rule inside it names the call that breaks it.
        let promised = Promises {
            may_pause: true,
            may_fail: true,
        };
        for (part, given) in [
            (Some(&spec.step), vec![acc.clone(), item]),
            (spec.merge.as_ref(), vec![acc.clone(), acc.clone()]),
        ] {
            let Some(part) = part else { continue };
            match part {
                Expr::Closure {
                    params,
                    mutable,
                    body,
                    ..
                } => {
                    self.lambda(params, mutable, body, &given, promised, span);
                }
                _ => {
                    let _ = self.a_named_fold_part_that_may_pause(part, span);
                    self.expr(part, span);
                }
            }
        }
    }

    /// `Summary::merge` or `Summary` in a fold: a function the fold calls, so
    /// a pausing one is refused as a call in an action is. What the function
    /// hands back, where its signature says.
    fn a_named_fold_part_that_may_pause(&mut self, part: &Expr, span: &Span) -> Option<Ty> {
        self.s
            .a_named_fold_part_that_may_pause(self.world, part, span)
    }

    fn function(&mut self, item: &Item, target: Option<&str>) {
        let Item::Fn {
            name,
            generics,
            receiver,
            args,
            config,
            spread,
            ret_type,
            body,
            can_throw: throws,
            is_sync,
            sync_by,
            ..
        } = item
        else {
            return;
        };

        // The same key the ledger uses, arrived at the same way - the anonymous
        // constructor of Kap 4.2 included, which a caller reaches as
        // `Type::new`. Two spellings of one name would silently drop every
        // method call in a constructor.
        let own_name = match name {
            Some(name) => self.parsed.text(*name).to_string(),
            None => "new".to_string(),
        };
        // A function's own name, for the reason the struct's is asked: `fn self`
        // cannot parse and `fn crate` can (ADR-298 D11). The span is the body's
        // first statement where there is one, which is the nearest this walk has
        // - a declaration with a span of its own is `FnArg`'s and not the item's.
        // **What `sync(f)` names is code the call runs**
        // ([ADR-288](../../docs/specification/adr/adr-288.md) D31).
        if !sync_by.is_empty() {
            let key = match target {
                Some(target) => format!("{target}::{own_name}"),
                None => own_name.clone(),
            };
            let at = body.stmts.first().map_or(Span::nowhere(), |s| s.span);
            for named in sync_by {
                let named = self.parsed.text(*named).to_string();
                self.s
                    .a_lambda_the_promise_names(self.world, &key, &named, args, &at);
            }
        }
        if let Some(span) = body.stmts.first().map(|s| s.span) {
            self.s.nameable(&own_name.clone(), &span, "a function");
        }
        let key = match target {
            Some(target) => format!("{target}::{own_name}"),
            None => own_name,
        };
        let outer_current = self.s.current.replace(key);
        let current_name = self.s.current.clone().unwrap_or_default();
        let outer_sync = match is_sync {
            true => self.s.inside_a_sync_function.replace(current_name),
            false => self.s.inside_a_sync_function.take(),
        };

        // **Inside its own body a type parameter is a type**
        // ([ADR-295](../../docs/specification/adr/adr-295.md) D1). `T` is not a
        // hole here: the caller picked it, this body did not, and a body that
        // put an `i64` where its caller's `T` is wanted would be wrong. So the
        // name stays a name and `fits` compares it, which is what lets `NK1126`
        // below say that nothing describes what a `T` can do. Only at a *call*
        // is it a variable (`bindings`), and that is the whole of the split.
        let mut declared: BTreeMap<String, Vec<String>> = self.s.enclosing.clone();
        declared.extend(generics.iter().map(|g| {
            (
                self.parsed.text(g.name).to_string(),
                g.bounds
                    .iter()
                    .map(|b| self.parsed.text(*b).to_string())
                    .collect(),
            )
        }));
        // `Self` stands for the type the `impl` is on, and nothing here
        // resolves it. Unlike `T` it is bound by no call site either, so it
        // stays erased: a comparison against it could only be a false positive.
        let parameters: BTreeSet<String> = ["Self".to_string()].into_iter().collect();
        let outer_declared = std::mem::replace(&mut self.s.type_parameters, declared);

        let outer_borrowing = std::mem::replace(
            &mut self.s.borrowing_self,
            receiver.as_ref().is_some_and(|r| r.is_ref),
        );

        let mut frame: Vec<Local> = Vec::new();
        if let Some(receiver) = receiver {
            // With the `impl` head's arguments (`subject_arguments`): `self`
            // inside `impl Holder[T]` is a `Holder[T]` and not a bare `Holder`,
            // so `self.value` binds the declaration's `T` and reaches the body
            // as a type rather than as `?`.
            let ty = match target {
                Some(target) => Ty::Named {
                    name: target.to_string(),
                    args: self.s.subject_arguments.clone(),
                    view: receiver.is_ref,
                },
                None => Ty::Unknown,
            };
            frame.push(self.local_free("self".to_string(), ty));
        }
        for arg in args {
            let name = self.parsed.text(arg.name).to_string();
            self.s.nameable(&name, &arg.span, "a parameter");
            frame.push(Local {
                written_mut: false,
                id: self.a_new_binding(),
                literal: None,
                name,
                ty: self.declared(&arg.ty, &arg.span).erase(&parameters),
                constant: None,
                // A parameter is not a `for` binding: what a call lends is the
                // caller's business, and a cast over one needs no deref.
                lent: false,
                built: None,
                // **D3**: without the word, a body that changes this parameter
                // is `NK1138`.
                empty_list: None,
                open_number: None,
                immutable: (!arg.mutable).then_some(Immutable {
                    at: arg.span,
                    kind: Kind::Parameter,
                }),
                // **D3's third state, on the binding**: this name is a `&mut T`
                // already, so a call that lends it writes nothing.
                changing: arg.mutable,
            });
        }
        // **An option is a parameter** (Part I 5.1): it stands after the `;`,
        // it is named at the call rather than passed by position, and it has a
        // default - and none of that changes that the body may use it. It was
        // missing from this frame, which nothing noticed while an undeclared
        // name was only refused in statement position: measured on
        // `examples/tally/src/main.nika`, whose `f"{lines}{separator}{blank}"` names one.
        // **A default that is not a literal is computed where the function is
        // declared** (ADR-318 D1, D7): refused here, with a `comptime`'s codes
        // and the option's name, where it cannot be.
        for option in config {
            self.a_computed_default(option);
        }
        for option in config {
            frame.push(self.local_free(
                self.parsed.text(option.name).to_string(),
                // Not `declared`: an option has no span of its own, and a
                // caret on the wrong line is worse than no message. Its default
                // is a literal (Part I 5.1), so a hull cannot stand here anyway.
                Ty::from_ast(self.parsed, &option.ty).erase(&parameters),
            ));
        }
        // **So is a spread** ([ADR-296](../../docs/specification/adr/adr-296.md)
        // D5): `...args: Self::dsl` is the driver's one value holding what the
        // statement left open, and its body hands it on. Missing from this
        // frame, `return args` was `NK1117` in every driver. Its type is the
        // statement's, which this checker does not see, so it is not guessed.
        if let Some(spread) = spread {
            let local = self.local_free(self.parsed.text(*spread).to_string(), Ty::Unknown);
            self.s.flow.unknown_bound.insert(local.id as usize as i64);
            frame.push(local);
        }

        let expected = ret_type
            .as_ref()
            .map(|t| Ty::from_ast(self.parsed, t).erase(&parameters));
        let outer = std::mem::replace(&mut self.s.expected, expected.clone());
        let outer_either = std::mem::replace(
            &mut self.s.flow.expected_either,
            ret_type.as_ref().is_some_and(|t| t.either),
        );
        let outer_throwing = std::mem::replace(&mut self.s.flow.throwing, *throws);

        let keeps: Vec<String> = self
            .s
            .current
            .as_ref()
            .and_then(|key| self.world.own.functions.get(key))
            .map(|contract| contract.keeps.clone())
            .unwrap_or_default();
        // **A parameter the function keeps dies in it**, unless it is handed
        // on, as a value bound by a `let` does (ADR-297 D2).
        for arg in args {
            let name = self.parsed.text(arg.name).to_string();
            let kept = keeps.contains(&name).then(|| {
                frame
                    .iter()
                    .find(|local| local.name == name)
                    .map(|local| local.ty.clone())
            });
            if let Some(Some(ty)) = kept {
                self.a_value_with_a_cleanup_dying(&name, &ty, &arg.span);
            }
        }
        let run_code: BTreeSet<String> = frame
            .iter()
            .filter(|local| matches!(local.ty, Ty::Fn { .. }) && !keeps.contains(&local.name))
            .map(|local| local.name.clone())
            .collect();
        let outer_run_code = std::mem::replace(&mut self.s.run_code, run_code);
        self.s.scope.push(frame);
        let tail_span = body.stmts.last().map(|s| s.span);
        let outer_lists = std::mem::take(&mut self.s.flow.empty_lists);
        let outer_numbers = std::mem::take(&mut self.s.flow.open_numbers);
        let outer_grown =
            std::mem::replace(&mut self.s.flow.grown_text, grown_text(self.parsed, body));
        let tail = self.block(body);
        self.s.flow.grown_text = outer_grown;
        // **The tail hands back as a `return` does**, and a field of a lent
        // place is refused there too (`NK1131`, #576). Read against the scope
        // still standing, as the numbers below are.
        if expected.is_some()
            && !a_body_never_ends(body)
            && let Some(Stmt::Expr(value)) = body.stmts.last().map(|s| &s.node)
            && let Some(at) = tail_span
            && !self.s.hands_back_a_view_of_the_subject(self.world, value)
        {
            self.a_field_of_a_borrowed_subject(value, &at, "handed back");
            self.a_field_of_a_lent_place_handed_back(value, &at);
        }
        // **A body's numbers, typed by their uses** once all of them have been
        // seen ([ADR-285](../../docs/specification/adr/adr-285.md) D24) - the
        // tail's included, which is read against the scope still standing.
        if let (Some(expected), Some(Stmt::Expr(value))) =
            (&expected, body.stmts.last().map(|s| &s.node))
        {
            let at = body.stmts.last().map_or(Span::nowhere(), |s| s.span);
            self.number_asked(value, expected, &at, true);
        }
        self.numbers_typed_by_their_uses();
        self.s.flow.open_numbers = outer_numbers;
        self.s.scope.pop();

        // **`NK1153`, once the whole body has been seen**
        // ([ADR-135](../../docs/specification/adr/adr-135.md) D2), for
        // `NK2101`'s reason one page down: the use that gives an empty list its
        // element type stands *after* the `let`, and a single pass reaches a
        // later statement later.
        for (name, at) in std::mem::replace(&mut self.s.flow.empty_lists, outer_lists).into_values()
        {
            self.s.an_empty_list_with_no_element_type(&name, &at);
        }

        // The last expression of a body is what the function hands back, so it
        // answers to the declared type exactly as a `return` does - unless the
        // body **cannot get there**
        // ([ADR-276](../../../docs/specification/adr/adr-276.md)).
        let ends = !a_body_never_ends(body);
        if let (Some(expected), Some(span)) = (&expected, tail_span.filter(|_| ends)) {
            // **A tail is a `return` written without the word**, so it owes the
            // same `&`: `fn text(ref self) -> ref String { self.text }` and the
            // `return` form are one program, and one of the two answering
            // differently would be a spelling that decides a refusal.
            let lending = matches!(body.stmts.last().map(|s| &s.node), Some(Stmt::Expr(value))
                if self.s.hands_back_a_view_of_the_subject(self.world, value));
            if lending {
                self.s.facts.lent_returns.insert(span.at() as i64);
            }
            let tail = match lending {
                true => view_of(&tail),
                false => tail,
            };
            // **And the tail is a use**, as `return` is (ADR-282 D4).
            let tail = match body.stmts.last().map(|s| &s.node) {
                Some(Stmt::Expr(value)) if !lending => self
                    .text_literal(expected, value, true)
                    .or_else(|| self.tuple_literal(&tail, expected, value))
                    .unwrap_or(tail),
                _ => tail,
            };
            let value = match body.stmts.last().map(|s| &s.node) {
                Some(Stmt::Expr(value)) => Some(value),
                _ => None,
            };
            // **A tail widens as a `return` does** (ADR-285 D32).
            // **And into an unsigned type where the walk shows it is not
            // negative** (D32): the slot's own question, asked here too.
            // **A plain tail stands where a `T?` is returned**, as a plain
            // `return` value does (ADR-278): it is put into `Some`.
            // **An `if` or a `match` is put into `Some` at each end**, as an
            // arm beside a `null` is: the tail is lowered as a statement, so a
            // wrap around the whole would not be written. Numbers its uses
            // have not typed yet (`?`) are plain too.
            let branches = matches!(
                value,
                Some(Expr::If { .. } | Expr::Match { .. } | Expr::Block(..))
            ) && value.is_some_and(every_end_is_a_value);
            let how = value.and_then(|v| self.widened_slot(&tail, expected, Some(v), &span));
            if let Some(value) = value {
                match how {
                    Some(Wrap::Constructor | Wrap::Conversion)
                        if branches && matches!(expected, Ty::Nullable(_)) =>
                    {
                        self.s.some_at_the_tails(value)
                    }
                    Some(how @ (Wrap::Widen(_) | Wrap::Unsigned(_) | Wrap::Constructor)) => {
                        self.s.facts.wrapped.insert(value_node(value), how);
                    }
                    _ => {}
                }
            }
            self.expect_kept(
                &tail,
                expected,
                value,
                "the function returns it, and the caller keeps it after the call ends",
                span,
                "returns",
                |found, want| {
                    format!(
                        "This function returns `{found}`, but it's declared to return `{want}`."
                    )
                },
            );
        }

        // **`NK2101`, once the whole body has been seen.** The question is what
        // comes *after* a `spawn`, and a single pass reaches a later statement
        // later - so it is asked here rather than at the `spawn`.
        // **`NK2702` first, because it borrows what the next one takes**
        // ([ADR-105](../../docs/specification/adr/adr-105.md) D2). A second walk
        // is a later statement and a single pass reaches one later, so both
        // questions are asked here; `a_task_took_what_is_used_again` empties
        // `read_at`, and this one reads it.
        self.s.a_sequence_was_walked_twice();
        self.s.a_part_left_out();
        self.s.data_handed_over_is_used_again();
        self.s.reads_on_paths.clear();
        self.s.flow.read_index.clear();
        self.s.lent_lets.clear();
        self.s.a_task_took_what_is_used_again();

        self.s.expected = outer;
        self.s.flow.expected_either = outer_either;
        self.s.flow.throwing = outer_throwing;
        self.s.type_parameters = outer_declared;
        self.s.borrowing_self = outer_borrowing;
        self.s.current = outer_current;
        self.s.inside_a_sync_function = outer_sync;
        self.s.run_code = outer_run_code;
    }

    /// **A type that holds itself holds itself through a box the compiler
    /// writes** ([ADR-246](../../docs/specification/adr/adr-246.md) D1).
    ///
    /// `enum Expr { Add(Expr, Expr) }` is a program as written: a value that
    /// holds a whole value of its own type inline has no size, so the fields on
    /// such a ring are the ones that go behind a `Box` below, and this is the
    /// walk that finds them. It was `NK1192`'s refusal at 0.0.234, until the
    /// owner decided the question it named.
    ///
    /// **A field is boxed where what it holds inline reaches back to the type
    /// that declares it** - the type itself, or one on the same ring. Only an
    /// edge that is certainly inline counts: the type, nullable or not, a
    /// tuple's parts and an array's element. A view, a function, a list or
    /// anything this compiler cannot see through is an indirection already.
    ///
    /// Recorded for **every** type of the program, not only this file's: a
    /// literal in this file may build a type declared beside it.
    fn types_that_contain_themselves(&mut self) {
        self.s
            .types_that_contain_themselves(&self.world.here, &self.world.beside)
    }

    /// **An `impl` of a trait nothing declares** (0.0.231).
    ///
    /// `impl db::Connection for Fake` lowered as it was written, and the
    /// language below answered *unresolved import `db`* about a file nobody
    /// wrote ([Part III C.1](../../docs/specification/30-nikaia-tooling.md));
    /// a bare `impl Connection for Fake` was *cannot find trait* the same way.
    /// Nothing looked at the trait an `impl` names: a **bound** fails open on a
    /// trait it does not know ([`Checker::answers_for`]), and that is right for
    /// a bound, which only asks. An `impl` *writes* the name into the program.
    ///
    /// **A path is [`Checker::a_head_nothing_declares`]'s question**, asked of
    /// its head: a package, a module, a described crate or a `std` module may
    /// hold the trait, and past a head this compiler knows it does not claim
    /// to know the rest. **A bare name is `NK1117`'s**, and it is declared by
    /// the three traits the language itself names (`Error`, Part I 7.1; `Drop`
    /// and `Cleanup`, Part I 6), a `trait` in any ledger this program reads, a
    /// `use` that brings it in, or a foreign name. Everything this cannot see
    /// is a name it must not refuse ([Part III
    /// C.4](../../docs/specification/30-nikaia-tooling.md)).
    fn a_trait_nothing_declares(&mut self, written: &str, span: &Span) {
        self.s
            .a_trait_nothing_declares(&self.world.here, self.world, written, span)
    }

    /// `?.` over a value that is never absent (`tools/check_fields.nika`).
    /// **An option's default that is computed rather than written** (ADR-318
    /// D1, D7): evaluated as a `comptime` is, and refused as one is where it
    /// cannot be. A number, text, a `bool`, and a struct or a variant of
    /// those, is what the ledger records; a list as a default is not built
    /// yet, and is said to be rather than handed to the language below empty.
    fn a_computed_default(&mut self, option: &ast::ConfigParam) {
        let name = self.parsed.text(option.name).to_string();
        self.computed_default(
            &name,
            &option.ty,
            &option.default,
            option.default_span,
            false,
        );
    }

    /// **A field's default** (ADR-331 D5): a literal is checked against the
    /// field's type as a literal's value is, and anything else is computed as
    /// an option's default is, under the same codes.
    fn a_field_default(&mut self, field: &ast::FieldDef) {
        let Some(default) = &field.default else {
            return;
        };
        let name = self.parsed.text(field.name).to_string();
        if !crate::contracts::a_literal(&default.value) {
            self.computed_default(&name, &field.ty, &default.value, default.span, true);
            return;
        }
        let found = self.expr(&default.value, &default.span);
        let declared = Ty::from_ast(self.parsed, &field.ty);
        let want = match (&declared, &default.value) {
            (_, Expr::LitNull(_)) => return,
            (Ty::Nullable(inner), _) => (**inner).clone(),
            _ => declared,
        };
        // Text written for an owned `String` is one, as an option's default
        // is (#498): the field gets text of its own.
        if matches!(&default.value, Expr::LitStr { .. })
            && matches!(&want, Ty::Named { name, .. } if name == "String")
        {
            return;
        }
        if let Some(kind) = attributes::literal_misfit(&default.value, &want) {
            self.s.findings.push(Finding {
                code: "NK1166".to_string(),
                severity: Severity::Error,
                span: default.span,
                message: format!(
                    "`{name}` holds `{}`, but its default is {kind}.",
                    want.text()
                ),
                notes: Vec::new(),
                help: Some(format!(
                    "Give `{name}` a default that is a `{}`.",
                    want.text()
                )),
                labels: Vec::new(),
            });
            return;
        }
        self.expect(&found, &want, default.span, "const", move |found, want| {
            format!("`{name}` holds `{want}`, but its default is `{found}`.")
        });
    }

    fn computed_default(
        &mut self,
        name: &str,
        ty: &ast::Type,
        default: &Expr,
        span: Span,
        field: bool,
    ) {
        let declared = Ty::from_ast(self.parsed, ty);
        let kind = match field {
            true => Computing::Field,
            false => Computing::Option,
        };
        self.computed_value(name, &declared, default, span, kind);
    }

    /// [`Checker::computed_default`] over a type already read, which an
    /// attribute's argument has: its field may be another package's
    /// (ADR-331 D4).
    fn computed_value(&mut self, name: &str, ty: &Ty, default: &Expr, span: Span, kind: Computing) {
        if crate::contracts::a_literal(default) {
            return;
        }
        let name = name.to_string();
        let walked_from = self.s.findings.len();
        self.expr(default, &span);
        if self.s.findings[walked_from..]
            .iter()
            .any(|f| f.severity == Severity::Error)
        {
            return;
        }
        self.s.computing_default = true;
        let declared_ty = crate::comptime_run::as_built(ty.clone());
        let (value, said) =
            match self.compiled_build_time_value(default, &declared_ty, &name, &span) {
                Some(outcome) => outcome,
                None => self.build_time_value(default, &name, &span),
            };
        self.s.computing_default = false;
        if said {
            return;
        }
        // **A list a `Vec` option would own is `NK1167`** (ADR-318 D4, D7):
        // refused for what it is, as a `comptime` is, with the length the
        // build computed in the way out.
        let declared = ty.clone();
        // **A view of a list takes one** (D4): `xs: ref Vec[i64] = [4, 5]`
        // crosses as the `&[4, 5]` the parameter is below. Only one it owns
        // is refused.
        if let Some(computed @ build_time::Value::List(_)) = &value
            && is_growable(&declared)
            && !declared.is_a_view()
        {
            let computed = computed.clone();
            self.s
                .a_constant_that_owns_memory(&name, &declared, &computed, "a default", &span);
            return;
        }
        // **A struct is only as writable as its fields** (D4), named as a
        // `comptime`'s is.
        if let Some(computed) = &value
            && let Some((field, held)) = self.unwritable_in(computed)
        {
            self.s
                .a_field_that_owns_memory(&name, &field, &held, "a default", &span);
            return;
        }
        let message = match &value {
            None => {
                format!("The default of `{name}` can't be computed while the program is built.")
            }
            Some(value) if crate::contracts::literal_of(value).is_none() => format!(
                "The default of `{name}` is a list of something other than numbers or \
                 `bool`s, or a list it would own, and a default of that shape is not built yet."
            ),
            Some(_) => return,
        };
        self.s.findings.push(Finding {
            code: "NK1127".to_string(),
            severity: Severity::Error,
            span,
            message,
            notes: vec![
                match kind {
                    Computing::Field => {
                        "A field's default is computed once, where the struct is declared, as \
                         a `comptime` is."
                    }
                    Computing::Option => {
                        "An option's default is computed once, where the function is declared, \
                         as a `comptime` is."
                    }
                    Computing::Attribute => {
                        "An attribute's argument is computed once, where it is written, as a \
                         `comptime` is."
                    }
                }
                .to_string(),
            ],
            help: Some(match kind {
                Computing::Field => format!(
                    "Give `{name}` a default the build can compute, or leave the default out \
                     and give the field in each literal."
                ),
                Computing::Option => format!(
                    "Give `{name}` a default the build can compute, or make it a `T?` and \
                     decide in the body."
                ),
                Computing::Attribute => {
                    format!(
                        "Give `{name}` a value the build can compute, or write the value itself."
                    )
                }
            }),
            labels: Vec::new(),
        });
    }

    /// **A value with a cleanup dies in this function**, a kept parameter's
    /// (`tools/check_crossings.nika`, U-d1).
    fn a_value_with_a_cleanup_dying(&mut self, name: &str, ty: &Ty, span: &Span) -> Option<bool> {
        self.nikaia(|state, world, _| {
            check_crossings::check_crossings_a_value_with_a_cleanup_dying(
                state, world, name, ty, span,
            )
        })
    }

    fn a_field_of_a_borrowed_subject(&mut self, value: &Expr, span: &Span, what: &str) {
        self.nikaia(|state, world, host| {
            check_core_leaves::check_core_a_field_of_a_borrowed_subject(
                state, world, host, value, span, what,
            )
        })
    }

    /// **`NK1131` for a place that is lent here and is not `self`**
    /// ([ADR-083](../../docs/specification/adr/adr-083.md), #576): a field of a
    /// `ref` parameter, of a `for` binding over a list or of a `let` over a
    /// place, handed back by value. `return l.ty` out of a `Local` the loop lent
    /// passed the check, and the language below said *cannot move out of `l.ty`
    /// which is behind a shared reference* about a file nobody wrote.
    ///
    /// **Only where the value is handed back**: a field passed to a call or
    /// bound to a name is `NK2106`'s, which says the same about what keeps it.
    /// **And not where the function declares a view**, which is what a field of
    /// a lent place is the answer to (D1's third position).
    fn a_field_of_a_lent_place_handed_back(&mut self, value: &Expr, span: &Span) {
        self.nikaia(|state, world, host| {
            check_core_leaves::check_core_a_field_of_a_lent_place_handed_back(
                state, world, host, value, span,
            )
        })
    }

    /// Part I 2.3: record where the emitter has to write the `Some(…)`.
    ///
    /// A type is non-nullable unless it says otherwise, so a plain `T` standing
    /// where a `T?` is wanted is the one widening this language has - `let mut
    /// m: &str? = null` and then `m = "World"`. `Ty::fits` allows it; the
    /// language below needs the constructor written, and this is where the
    /// emitter is told.
    ///
    /// **Which of the two**, and the answer used to be *"one, or nothing"*
    /// ([ADR-278](../../../docs/specification/adr/adr-278.md)).
    ///
    /// The constructor may only be written where the value is known **not** to
    /// be a `T?` already, because wrapping one that is would make an
    /// `Option<Option<T>>`. Two ways it can be known, and a type is only the
    /// first:
    ///
    /// * its type says so — anything this checker worked out that is not a
    ///   `T?`; or
    /// * **it is a literal**, which no literal ever is. That second one is not
    ///   a convenience: a number has no type of its own on purpose (Part I 2.4,
    ///   so that `add(3)` is right wherever the parameter is numeric), so
    ///   `return 42` against a declared `i64?` answers `Unknown` and the type
    ///   alone would leave the commonest case in the section unwrapped.
    ///
    /// **And everything else takes the conversion**, which is what changed:
    /// `value.into()` is correct whether the value is a `T` or already a `T?`,
    /// so a type this checker could not work out stops being a position it has
    /// to stay silent about. It used to be left alone — right about the risk,
    /// and the program then failed in the language below with `rustc`'s *"try
    /// wrapping the expression in `Some`"* about a form Nikaia does not have
    /// (Part III C.1).
    ///
    /// `null` is in neither, being a `T?` itself.
    /// **What a slot is handed, and how it is wrapped**: [`wrap_for`], except
    /// that a widening of an arithmetic expression is pushed to its operands
    /// ([`Checker::widen_operands`]) and nothing is wrapped around it.
    fn widened_slot(
        &mut self,
        found: &Ty,
        want: &Ty,
        value: Option<&Expr>,
        span: &Span,
    ) -> Option<Wrap> {
        self.nikaia(|state, world, host| {
            check_overflow::check_widened_slot(state, world, host, found, want, value, span)
        })
    }

    /// **A use asks the open numbers in a value for a type** (D2): a type
    /// standing beside it (`beside`), or an index, which takes any integer.
    fn number_asked(&mut self, value: &Expr, want: &Ty, at: &Span, beside: bool) {
        self.s.number_asked(self.world, value, want, at, beside);
    }

    /// **Every open number of a body, decided** once the body has been walked
    /// ([ADR-285](../../docs/specification/adr/adr-285.md) D24, D25, D27).
    fn numbers_typed_by_their_uses(&mut self) {
        self.s.numbers_typed_by_their_uses();
    }

    // --- statements ---------------------------------------------------------

    /// Walk a block and hand back the type of its tail.
    fn block(&mut self, block: &Block) -> Ty {
        self.nikaia(|state, world, host| {
            check_core_stmt::check_core_block(state, world, host, block)
        })
    }

    // --- expressions --------------------------------------------------------

    /// **The walk of one expression** (`tools/check_core_expr.nika`, U-c11).
    fn expr(&mut self, expr: &Expr, span: &Span) -> Ty {
        self.nikaia(|state, world, host| {
            check_core_expr::check_core_expr(state, world, host, expr, span)
        })
    }

    /// `Shared(x)`, `SharedMut(x)` and `Locked(x)` make a hull
    /// ([ADR-281](../../docs/specification/adr/adr-281.md) D2).
    ///
    /// **One argument, and the hull takes the type of what it is handed.** A
    /// literal hands over `Unknown`, which is not a failure here: the hull is made
    /// by a *call* in the emitted code too, so the language below gives the number
    /// its type the way it gives one to any argument. That is the difference
    /// between a constructor and an annotation, and it is why the annotation could
    /// not answer `SharedMut(0)` at all.
    /// A type **as it is written**, checked for the one spelling this language
    /// does not have ([ADR-281](../../docs/specification/adr/adr-281.md) D3).
    ///
    /// `Shared[Locked[T]]` is what `SharedMut[T]` is, and two ways to write one
    /// type is the thing the short name was given a name *instead* of
    /// ([ADR-281](../../docs/specification/adr/adr-281.md) D3). Worse than
    /// untidy: the two are the same bytes below and two different types up here,
    /// so a value of one would not fit the other while the emitted Rust could not
    /// tell them apart.
    ///
    /// It runs at the positions a **program** writes a type, and the message
    /// carries the replacement.
    fn declared(&mut self, ty: &ast::Type, span: &Span) -> Ty {
        self.s.spelling(self.world, ty, span);
        let written = Ty::from_ast(self.parsed, ty);
        self.a_key_with_no_order(&written, span);
        written
    }

    /// **`NK1247`: an ordered map or set whose key has no order**
    /// ([ADR-010](../../docs/specification/adr/adr-010.md) D10).
    ///
    /// `BTreeMap` and `BTreeSet` keep their keys in order, so the key has to
    /// have one. The types of the language answer from a list (a float has none),
    /// a type this language declares has none, and a type whose parts are Rust
    /// answers from the ledger's `orders` column, whose absence is *no*. Asked at
    /// the positions a program writes a type, over everything inside it.
    fn a_key_with_no_order(&mut self, ty: &Ty, span: &Span) {
        let beside = self.parts_beside();
        let asking = Comparable {
            beside: &beside,
            structs: &self.s.structs,
            enums: &self.s.enums,
            payloads: &self.s.enum_payloads,
            parameters: &self.s.struct_parameters,
            own: self.world.own,
            library: self.world.library,
        };
        let refused =
            nikaia_std::tools::check_compare::compare_keys_with_no_order(&asking, ty, span);
        self.s.findings.extend(refused);
    }

    // --- the one shape every check has --------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn expect_kept(
        &mut self,
        found: &Ty,
        want: &Ty,
        value: Option<&Expr>,
        keeper: &str,
        span: Span,
        what: &str,
        message: impl Fn(&str, &str) -> String,
    ) {
        self.s.expect_kept_asked(
            self.world,
            found,
            want,
            value,
            keeper,
            &span,
            what,
            &|hole, ty| message(hole, ty),
        )
    }

    fn expect(
        &mut self,
        found: &Ty,
        want: &Ty,
        span: Span,
        what: &str,
        message: impl Fn(&str, &str) -> String,
    ) {
        self.s
            .expect_type(self.world, found, want, &span, what, &message);
    }

    /// **The signed values in unsigned slots, asked of the walk** (ADR-285
    /// D32, [`crate::bounds::proven`]): one proved `>= 0` where it stands is
    /// taken as it is; one that is not is the slot's refusal, with the two
    /// ways to say it.
    fn unsigned_slots_proved(&mut self, parsed: &Parsed) {
        let slots: Vec<UnsignedSlot> = std::mem::take(&mut self.s.flow.unsigned_slots)
            .into_values()
            .flatten()
            .collect();
        if slots.is_empty() {
            return;
        }
        let nodes: BTreeSet<i64> = slots.iter().map(|s| s.node).collect();
        let proven = crate::bounds::proven(
            parsed,
            &crate::bounds::Sites::of(&self.s.facts),
            &[self.world.own, self.world.library],
            &nodes,
        );
        for slot in slots {
            if proven.nonnegative.contains(&((slot.node as usize) as i64)) {
                continue;
            }
            self.s
                .findings
                .push(check_overflow::check_unsigned_slot_refused(&slot));
        }
    }

    // --- crossing a thread (ADR-005 §1 Group B, `NK25xx`) -------------------
    //
    // Two places a value the *program* wrote reaches another thread, and **the
    // verdict is no longer the same in both** (ADR-312 D6): each asks
    // `contracts::send` about its own destination, and the lock is the type the
    // two answers differ on. Everything else answers alike wherever it is going,
    // so the split costs one argument and changes nothing else.
    //
    // The rule the destination does **not** displace is ADR-303 D7's: a value may
    // cross a thread only if it may cross any thread. What ADR-312 separates is
    // *which* thread is being talked about - one of ours, or one a library we
    // cannot read may start - and each answer is the same at both settings of
    // `user_parallelism`, which is all Group B ever asked. `contracts::send` owns
    // the walk and the reason; these two own the span and the sentence.
    //
    // Both report on `MayNot` and say nothing on `Undecided`, which is
    // `contracts::send`'s module header: refusing what this compiler cannot
    // decide would reject correct programs, and an undecided crossing is not
    // accepted here - `rustc` still type-checks the emitted crate, and
    // ADR-005 D7's `E0277` translation reports that refusal against this same
    // `.nika` line.
    //
    // **A method call is not asked**, and the limit is worth naming: a foreign
    // Rust function is reached by a qualified path (`hyper_shim::serve`), which
    // is what a name-for-name lowering makes callable at all (ADR-296 D17), so
    // the call form below is the one ADR-303 D7 is about. A method on a receiver
    // whose type no ledger describes would need the same question asked of its
    // arguments, and nothing in the corpus reaches it - ADR-288 D11's rule, that
    // an entry exists because a program asked for it.

    /// **What an entry point may be** ([ADR-284](../../docs/specification/adr/adr-284.md)
    /// D4, D5): `extern` before a function with a body exports it to C, and four
    /// shapes are refused at the declaration.
    fn an_entry_point(&mut self, item: &Item, span: &Span, of_a_trait: bool) {
        self.s.an_entry_point(self.world, item, span, of_a_trait);
    }

    /// **What a `pub extern struct` may hold**
    /// ([ADR-284](../../docs/specification/adr/adr-284.md) D14, D15): it is
    /// `pub`, it is not generic, and every field is a C value.
    fn an_extern_struct(&mut self, item: &Item, span: &Span) {
        self.s.an_extern_struct(self.world, item, span);
    }

    /// **What a `pub extern enum` may hold**
    /// ([ADR-284](../../docs/specification/adr/adr-284.md) D32): it is `pub`,
    /// and every field of every variant is a C value, as an `extern` struct's.
    fn an_extern_enum(&mut self, item: &Item, span: &Span) {
        self.s.an_extern_enum(self.world, item, span);
    }

    /// **The interpreter, and the two refusals it can raise**
    /// ([ADR-287](../../docs/specification/adr/adr-287.md) D6's second stage,
    /// bounded by [ADR-287](../../docs/specification/adr/adr-287.md)).
    ///
    /// `None` is *this compiler cannot evaluate it*, which the caller turns
    /// into `NK1127` — the refusal that has always been there and whose note
    /// says what the stage knows. A body the rule **forbids** is a different
    /// claim and gets `NK1152`: the shape is understood and the answer is no.
    ///
    /// The second half of the pair is **whether a refusal was already
    /// reported**. Two errors for one mistake is what this used to print:
    /// `NK1152` naming the callee the rule forbids, and then `NK1127` saying
    /// the compiler cannot evaluate it — which is not a second fact, it is the
    /// first one said again with less in it. The caller keeps `NK1127` for the
    /// case it is about: a shape this evaluator does not read, which nothing
    /// else has a sentence for.
    /// **An initialiser that calls a function, compiled and run**
    /// ([ADR-321](../../docs/specification/adr/adr-321.md) D1,
    /// `crate::comptime_run`), where this stage builds one: the build has a
    /// workshop, the program is this one file and declares no grammar, and the
    /// initialiser is numbers, names and calls to this file's own functions.
    /// Nothing where it does not apply, and the interpreter answers as before.
    ///
    /// **The rule is asked first** (D2): each function the initialiser calls
    /// is read off the ledger, and one that may not run is `NK1152` before
    /// anything is compiled. What a callee calls in turn is in its own entry.
    fn compiled_build_time_value(
        &mut self,
        value: &Expr,
        found: &Ty,
        bound: &str,
        span: &Span,
    ) -> Option<(Option<build_time::Value>, bool)> {
        let host = self.host();
        let done = self
            .s
            .compiled_build_time_value(self.world, &host, value, found, bound, span);
        done.answered.then_some((done.value, done.said))
    }

    fn build_time_value(
        &mut self,
        value: &Expr,
        bound: &str,
        span: &Span,
    ) -> (Option<build_time::Value>, bool) {
        let host = self.host();
        self.s.build_time_value(&host, value, bound, span)
    }

    /// A tuple of text literals where a tuple with text is wanted
    /// (`tools/check_texts.nika`).
    fn tuple_literal(&mut self, found: &Ty, want: &Ty, value: &Expr) -> Option<Ty> {
        self.s.tuple_literal(self.world, found, want, value)
    }

    /// A text literal where `String` is wanted, built as text of its own
    /// (`tools/check_texts.nika`).
    fn text_literal(&mut self, want: &Ty, value: &Expr, owned: bool) -> Option<Ty> {
        self.s.text_literal(self.world, want, value, owned)
    }
}

impl<'a> Checker<'a> {
    /// A field or a payload a `const` cannot hold, anywhere in a computed value
    /// (`tools/check_consts.nika`).
    fn unwritable_in(&self, value: &build_time::Value) -> Option<(String, String)> {
        let (parsed, beside) = (self.parsed, self.beside);
        match check_consts::unwritable_in(&self.s, self.world, value, &|ty: &str, variant: &str| {
            payload_of(parsed, beside, ty, variant)
        })
        .as_slice()
        {
            [name, held] => Some((name.clone(), held.clone())),
            _ => None,
        }
    }

    /// **The instantiations that stand in the program's other files**
    /// ([ADR-304](../../docs/specification/adr/adr-304.md) D6).
    ///
    /// A copy is written by the unit that declares the function, and which
    /// copies there are is decided at the **call** — which may stand in any
    /// file of the package, because they share one namespace (Part I 9.1). So
    /// this unit asks the others what they called its shape walks with, and
    /// without it `describe` in `shapes.nika` called from `main.nika` is a
    /// function with no copies and no generic original: a name that reaches
    /// the language below undeclared, which is
    /// [Part III C.1](../../docs/specification/30-nikaia-tooling.md)'s class.
    ///
    /// **Nothing to ask about costs nothing**: a unit that declares no shape
    /// walk returns before a second file is looked at, which is every unit of
    /// every program in the corpus.
    fn instantiations_beside(&mut self) {
        if self.s.flow.harvesting || self.beside.is_empty() {
            return;
        }
        let mine: BTreeSet<String> = self
            .parsed
            .program
            .items
            .iter()
            .filter_map(|item| match &item.node {
                Item::Fn {
                    name: Some(name), ..
                } => Some(self.parsed.text(*name).to_string()),
                _ => None,
            })
            .filter(|name| self.s.walks_fields.contains_key(name))
            .collect();
        if mine.is_empty() {
            return;
        }
        let mut found: Vec<((String, String), Vec<FieldContract>)> = Vec::new();
        for other in self.others() {
            found.extend(
                instantiations_in(
                    other,
                    self.beside,
                    self.world.own,
                    self.world.library,
                    &self.s.modules,
                    self.reads,
                )
                .into_iter()
                .filter(|((name, _), _)| mine.contains(name)),
            );
        }
        // **This unit's own calls win**, because they were typed by the walk
        // that also holds the spans: the two agree, and a harvest that
        // disagreed would be a second answer to a question already answered.
        for (instantiation, fields) in found {
            self.s.facts.unrolled.entry(instantiation).or_insert(fields);
        }
    }

    /// **The body, once per unrolled turn**
    /// ([ADR-304](../../docs/specification/adr/adr-304.md) D7).
    ///
    /// `field.of(value)` has a different type in each turn, so there is no one
    /// type to check the body against — and a body that is wrong for one field
    /// is **right** for the others, on the same line. So the walk is repeated
    /// with the field bound, and every finding it makes carries the note that
    /// says which turn it came from. Without that line this is the error class
    /// C++ templates carried for twenty years.
    ///
    /// **After the ordinary walk**, because the instantiations are what that
    /// walk found: a call may stand above the function it names, and which
    /// types a function is used with is not knowable until every call has been
    /// read.
    ///
    /// The cost is fields × instantiations, and only for the functions that
    /// walk a shape and only for the types actually used (D5). A function
    /// nobody calls is not unrolled at all.
    fn unroll(&mut self) {
        let instantiations: Vec<(String, String, Vec<FieldContract>)> = self
            .s
            .facts
            .unrolled
            .iter()
            .map(|((name, on), fields)| (name.clone(), on.clone(), fields.clone()))
            .collect();
        for (name, on, fields) in instantiations {
            let Some(item) = self.parsed.program.items.iter().find(|item| {
                matches!(&item.node, Item::Fn { name: Some(written), .. }
                    if self.parsed.text(*written) == name)
            }) else {
                continue;
            };
            let item = item.clone();
            for field in fields {
                let before = self.s.findings.len();
                let outer = self.s.flow.unrolling.replace(CheckUnrolling {
                    on: on.clone(),
                    field: field.clone(),
                });
                self.function(&item.node, None);
                self.s.flow.unrolling = outer;
                // **What the generic walk already said is not said again.**
                // The same body was walked once with nothing known, and a
                // message from that walk is about the **function** — a
                // misspelled member, say — so repeating it once per field
                // would turn one mistake into as many as the type has fields.
                // What is left is what this turn alone found, which is D5's
                // whole point.
                let already: BTreeSet<(String, usize)> = self.s.findings[..before]
                    .iter()
                    .map(|f| (f.code.clone(), f.span.at()))
                    .collect();
                let mut fresh: Vec<Finding> = self.s.findings.split_off(before);
                fresh.retain(|f| !already.contains(&(f.code.clone(), f.span.at())));
                for found in &mut fresh {
                    let (member, one) = match self.s.enums.contains_key(&on) {
                        true => (VARIANTS, "variant"),
                        false => (FIELDS, "field"),
                    };
                    found.notes.push(format!(
                        "This happened while going through `{}::{member}` for `{on}`, at the {one} `{}`.",
                        self.s.walks_fields
                            .get(&name)
                            .map(String::as_str)
                            .unwrap_or("T"),
                        field.name
                    ));
                }
                self.s.findings.extend(fresh);
            }
        }
    }

    // --- looking things up ---------------------------------------------------

    /// A number no other binding gets, for [`Local::id`]: the state's own
    /// counter, the one the Nikaia side takes from (`tools/check_state.nika`).
    fn a_new_binding(&mut self) -> i64 {
        self.s.a_new_binding()
    }

    /// A binding that nothing here refuses a change to, with a number of its own.
    fn local_free(&mut self, name: String, ty: Ty) -> Local {
        let id = self.a_new_binding();
        Local::free(name, ty, id)
    }

    /// Bind a name, with the constant it stands for where there is one
    /// (ADR-285 D29). Only [`Stmt::Let`] ever passes anything but `None`.
    fn bind_with(&mut self, name: String, ty: Ty, constant: Option<build_time::Integer>) {
        self.s.bind_with(name, ty, constant);
    }

    /// A lambda whose parameters have types, because the callee said so.
    ///
    /// The implicit form is the one that matters and the one that had no way
    /// to work: `fn { a.add(t) }` records **no parameters at all** in the AST -
    /// Part I 5.3 settles how many it takes from which of `a`, `b`, `c` the
    /// body mentions, and that is decided when it is emitted. So the names are
    /// bound here, in order, to whatever the signature says the lambda is
    /// handed. A body that mentions fewer of them simply leaves the later
    /// bindings unused.
    fn lambda(
        &mut self,
        params: &[winnow_grammar::Symbol],
        mutable: &[winnow_grammar::Symbol],
        body: &Block,
        given: &[Ty],
        promised: Promises,
        span: &Span,
    ) -> (Ty, bool) {
        self.nikaia(|state, world, host| {
            check_core_leaves::check_core_lambda(
                state, world, host, params, mutable, body, given, &promised, span,
            )
        })
    }

    /// **Every item-level `comptime`, in a frame that stays under the whole
    /// file** ([ADR-287](../../../docs/specification/adr/adr-287.md)).
    ///
    /// A pass of its own, and both halves of that are load-bearing. It runs
    /// *before* the item loop, because a `fn` declared **above** a constant
    /// sees it as much as one below — an item is visible in its whole scope,
    /// which is what makes it an item. And the frame is pushed and never
    /// popped, because this checker's scope is a stack pushed per function and
    /// there is nothing under all of them otherwise.
    ///
    /// The bodies go through [`Self::comptime_binding`], the same function the
    /// statement form uses, so the fold, the refusal and the spelling the
    /// emitter is handed are one thing in one place
    /// ([ADR-287](../../docs/specification/adr/adr-287.md) D3: the difference
    /// between the two places is where the name is visible and nothing else).
    fn item_constants(&mut self) {
        self.s.scope.push(Vec::new());
        // **Every name first, because a constant is an item.** A function
        // declared below its caller has always been callable — items are
        // order-independent — and a constant was not, because this walk goes
        // down the file and binds as it goes. So `comptime A = B * 2` above
        // `comptime B = 21` was `NK1117`, *nothing declares `B`*: a correct
        // program refused ([Part III C.4](../../docs/specification/30-nikaia-tooling.md))
        // with a sentence that was not true, since the next line declares it.
        //
        // The type is the **declared** one where there is one and `?` where
        // there is not — which says nothing, and is what a name whose value
        // this walk has not reached yet honestly is. The binding below shadows
        // it with the answer.
        // **And every constant another file of the package declares**
        // (ADR-286 D1): one namespace holds them as it holds the functions
        // and the types, so `TEXT` from `ty.nika` is read in `views.nika` -
        // where it was `NK1117`, *isn't declared anywhere* (found moving the
        // compiler's `views` into Nikaia, #125). Its declared type, read in
        // its own file; its value is that file's to check.
        for other in self.others() {
            for item in &other.program.items {
                let Item::Comptime { name, ty, .. } = &item.node else {
                    continue;
                };
                let declared = ty
                    .as_ref()
                    .map(|ty| Ty::from_ast(other, ty))
                    .unwrap_or(Ty::Unknown);
                self.bind_with(other.text(*name).to_string(), declared, None);
            }
        }
        for item in &self.parsed.program.items {
            let Item::Comptime { name, ty, .. } = &item.node else {
                continue;
            };
            let declared = ty
                .as_ref()
                .map(|ty| self.declared(ty, &item.span))
                .unwrap_or(Ty::Unknown);
            self.bind_with(self.parsed.text(*name).to_string(), declared, None);
        }
        for item in &self.parsed.program.items {
            let Item::Comptime {
                name,
                ty,
                value,
                bounds,
                ..
            } = &item.node
            else {
                continue;
            };
            self.comptime_binding(*name, ty.as_ref(), value, bounds, &item.span);
        }
    }

    /// **One `comptime`, wherever it stands** (ADR-287,
    /// `tools/check_core_comptime.nika`).
    fn comptime_binding(
        &mut self,
        name: Ident,
        ty: Option<&crate::ast::Type>,
        value: &Expr,
        bounds: &[crate::ast::ConfigArg],
        span: &Span,
    ) -> Ty {
        let bounds = bounds.to_vec();
        self.nikaia(|state, world, host| {
            check_core_comptime::check_core_comptime_binding(
                state, world, host, name, ty, value, &bounds, span,
            )
        })
    }
}

/// **Whether this file declares a type by this name**
/// ([ADR-286](../../docs/specification/adr/adr-286.md) D31).
///
/// For the package's *other* files, which share one namespace with this one
/// (Part I 9.1). It reads the item tree rather than a ledger because a ledger
/// records what a type **promises**, and what is asked here is only that the
/// word was written down: a private `enum` a neighbouring file declares is a
/// head, whatever its contract says.
/// **The names a body gives text of its own by growing them**: `s = s + "…"`,
/// `s += "…"` or `s = f"…"`, anywhere in it (0.0.232). Syntactic, because it is
/// asked at the `let`, before the assignment has been walked; and narrow for
/// the same reason - `+` and an f-string can only hand back owned text, where a
/// name or a call may hand back a view.
fn grown_text(parsed: &Parsed, body: &Block) -> BTreeSet<String> {
    fn walk(parsed: &Parsed, block: &Block, out: &mut BTreeSet<String>) {
        for stmt in &block.stmts {
            if let Stmt::Assign {
                target: Expr::Variable(name, _),
                op,
                value,
            } = &stmt.node
            {
                let grows = matches!(op, Some(BinaryOp::Add))
                    || (op.is_none()
                        && matches!(
                            value,
                            Expr::Binary {
                                op: BinaryOp::Add,
                                ..
                            } | Expr::LitInterpolated { .. }
                        ));
                if grows {
                    out.insert(parsed.text(*name).to_string());
                }
            }
            crate::contracts::sync::visit_stmt_blocks(&stmt.node, &mut |inner| {
                walk(parsed, inner, out)
            });
        }
    }
    let mut out = BTreeSet::new();
    walk(parsed, body, &mut out);
    out
}

/// What the checker reads of a parsed file, borrowed (`tools/parsed_view.nika`).
fn parsed_view_of(parsed: &Parsed) -> ParsedView<'_, '_, '_, '_, '_> {
    ParsedView {
        words: &parsed.interner,
        aliases: &parsed.aliases,
        items: &parsed.program.items,
        package: parsed.package.as_deref(),
        spans: &parsed.expr_spans,
    }
}

/// `&x`, as far as Stage 0 can say.
///
/// A view of a `String` is a `&str`, because that is what the language below
/// does at a call and what a Nikaia programmer means by writing it. A view of
/// anything with type arguments is not stated: `&Vec[T]` and `&[T]` are the
/// same expression there, and picking one would report an error that is not
/// there.
fn view_of(inner: &Ty) -> Ty {
    nikaia_std::tools::check_types::view_of_type(inner)
}

/// The type several parts of a program own at once (Part I 6.2).
const SHARED: &str = "Shared";
/// Part I 6.3's lock, which shares the `Shared` hull's construction line
/// ([ADR-281](../../../docs/specification/adr/adr-281.md)).
const LOCKED: &str = "Locked";

/// The shared mutable type: a value several parts own at once and any of them may
/// change ([ADR-281](../../../docs/specification/adr/adr-281.md) D3).
///
/// It is a **name this module carries whole**. Only the emitter expands it to the
/// count around the lock, which is what keeps a spelling nobody wrote out of every
/// message about the language's most common type
/// ([ADR-281](../../../docs/specification/adr/adr-281.md) D1).
const SHARED_MUT: &str = "SharedMut";

/// The same three, as a list something outside this module can read
/// ([ADR-313](../../docs/specification/adr/adr-313.md) D6).
///
/// **They are reached with no `use`, so they are on Part I 1.3's list**, and a
/// list that says *there are no others* needs the compiler's own answer to
/// compare against. [ADR-313](../../docs/specification/adr/adr-313.md) D10 built
/// that comparison for every **function** `std` keys bare and a type escaped it,
/// because a type is keyed under its own name rather than under nothing.
///
/// This is the answer from the compiler's side rather than the ledger's, and it
/// is the right side: what makes these three names a program writes with no
/// import is that this file knows them by name.
pub const HULLS: [&str; 3] = [SHARED, SHARED_MUT, LOCKED];

/// The two doors that take **several** locks at once
/// ([ADR-281](../../../docs/specification/adr/adr-281.md)).
///
/// They are not ledger entries and not grammar: the parser already takes
/// `access_all(a, b) fn(x, y) { … }` as an ordinary call with a trailing lambda
/// (Part I 5.3), and what is missing is only somebody to type it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiLock {
    /// `access_all` - every lock read in place, none of them changed.
    Reading,
    /// `update_all` - every lock written, one new value each.
    Writing,
}

impl MultiLock {
    /// The name a program writes, or `None` for anything else.
    pub fn named(name: &str) -> Option<MultiLock> {
        match name {
            "access_all" => Some(MultiLock::Reading),
            "update_all" => Some(MultiLock::Writing),
            _ => None,
        }
    }

    pub fn written(self) -> &'static str {
        match self {
            MultiLock::Reading => "access_all",
            MultiLock::Writing => "update_all",
        }
    }
}

/// **Whether a `'…'` as written is one ASCII character**, which is what makes
/// it a byte as well as a scalar (Part I 2.6; `tools/check_texts.nika`).
pub fn a_byte(written: &str) -> bool {
    check_texts::literal_is_a_byte(written)
}

/// The byte a `'…'` that [`a_byte`] took is, as the language below writes one.
pub fn byte_literal(written: &str) -> String {
    nikaia_std::tools::check_texts::check_byte_literal(written)
}

/// **What a table about an expression is keyed on**: the id the parser gave the
/// node ([ADR-340](../../docs/specification/adr/adr-340.md) D4), which a clone
/// keeps and an address does not.
/// Whether every end of an `if`, a `match` or a block is a value, so that it
/// can be put into `Some`: an end that jumps away (`return`, `break`,
/// `continue`, `throw`) or a block that ends in a statement is none.
fn every_end_is_a_value(expr: &Expr) -> bool {
    fn tail_is_a_value(block: &crate::ast::Block) -> bool {
        match block.stmts.last().map(|s| &s.node) {
            Some(Stmt::Expr(value)) => every_end_is_a_value(value),
            _ => false,
        }
    }
    match expr {
        Expr::If {
            then_branch,
            else_branch,
            ..
        } => {
            tail_is_a_value(then_branch)
                && else_branch.is_some()
                && else_branch.iter().all(tail_is_a_value)
        }
        Expr::Match { arms, .. } => arms.iter().all(|arm| every_end_is_a_value(&arm.body)),
        Expr::Block(block, _) => tail_is_a_value(block),
        Expr::Return(..) | Expr::Break(..) | Expr::Continue(..) | Expr::Throw(..) => false,
        _ => true,
    }
}

pub fn value_node(expr: &Expr) -> i64 {
    crate::ast::id_of(expr) as i64
}

/// **`an i32` and `a u64`**: the article as the name is said (`NK1116`'s rule).
/// A `u` is said *you*, so it takes `a`; the other vowels take `an`.
pub fn an_or_a(ty: &str) -> String {
    nikaia_std::tools::render::an_or_a(ty)
}

/// A view of a value that copies read as the value, and anything else as it is
/// ([ADR-233](../../docs/specification/adr/adr-233.md) D4).
impl Checker<'_> {}

/// **`--asserts`** ([ADR-269](../../docs/specification/adr/adr-269.md) D7):
/// every `assert` of one file, by line, and how the compiler holds it: proved,
/// a precondition and how many calls check it, checked at run time and why, a
/// test's check, or refused (D6) - and how many of each, so that the number
/// checked at run time can be watched.
///
/// **Checked as the build checks it**: with the program's other files and
/// what the build may read and compile, so that a `comptime` a call computes
/// and a call run for a claim (ADR-359 D7, D11) answer here as they did there.
pub fn claims_report(
    parsed: &Parsed,
    source: &str,
    path: &str,
    beside: &[&Parsed],
    own: &Ledger,
    library: &Ledger,
    reads: &Reads,
) -> String {
    let checked = check_against(
        parsed,
        beside,
        own,
        library,
        &BTreeSet::new(),
        &Newly::new(),
        reads,
    );
    // **Red and green only on a terminal**, and never where `NO_COLOR` is set
    // (Part III, `--asserts`).
    let colour = std::env::var_os("NO_COLOR").is_none()
        && std::io::IsTerminal::is_terminal(&std::io::stdout());
    nikaia_std::tools::check_facts::check_claims_report(
        &checked.facts.claims,
        source,
        path,
        &checked.facts.published,
        colour,
    )
}

/// The prelude's claim ([ADR-269](../../docs/specification/adr/adr-269.md) D2).
pub const ASSERT: &str = "assert";

/// In Nikaia (`tools/written.nika`, ADR-294, #125).
pub fn written(parsed: &Parsed, expr: &Expr) -> String {
    nikaia_std::tools::written::written_expression(&parsed.interner, expr)
}

/// Every `a` or `an` in front of a word in backticks, chosen by that word
/// (`check_words::article`): `a `i64`` becomes `an `i64``, and `an `u8`` `a
/// `u8``. Only the article directly before a backtick, so prose and code
/// samples are left as they are.
fn articles(text: &str) -> String {
    nikaia_std::tools::check_words::articles(text)
}

/// Whether handing a value of this type out takes nothing away.
///
/// Part I 2.2's numbers, `bool` and `char`, and any **view**: all of them are
/// copied rather than moved in the language below, so a field of one may leave a
/// borrowed subject freely. Everything else - a `String`, a `Vec`, a struct this
/// program declares - is moved, which is what `NK1131` is about.
///
/// A short list on purpose, and the polarity is the usual one: a type not on it
/// **Whether two values of this type may be compared** (ADR-204 D1):
/// `tools/check_compare.nika`.
use nikaia_std::tools::check_compare::CheckComparable as Comparable;

/// How a member that may be boxed is named, the same in the checker and the
/// emitter ([ADR-246](../../docs/specification/adr/adr-246.md)): a struct's
/// field by its name, a variant's part as `Variant#0`, and a variant's named
/// field as `Variant.name`.
pub fn boxed_member(owner: &str, at: Option<usize>, field: Option<&str>) -> String {
    nikaia_std::tools::check_words::boxed_member(
        owner,
        at.map_or(-1, |at| at as i64),
        field.unwrap_or_default(),
    )
}
