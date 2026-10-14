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
use crate::ast::{self, BinaryOp, Block, Expr, Item, MatchPattern, Span, Stmt, UnaryOp};
use crate::build_time;
use crate::contracts::SignatureOps;
use crate::contracts::ty::TyOps;
use crate::contracts::{FieldContract, FnContract, Ledger, ty, ty::Ty};
use crate::fold::Constant;
use crate::parser::Parsed;
use nikaia_std::tools::check_arguments;
use nikaia_std::tools::check_consts;
use nikaia_std::tools::check_core_args;
use nikaia_std::tools::check_core_call;
use nikaia_std::tools::check_core_comptime;
use nikaia_std::tools::check_core_leaves;
use nikaia_std::tools::check_core_method;
use nikaia_std::tools::check_core_places;
use nikaia_std::tools::check_core_stmt;
use nikaia_std::tools::check_flow::{
    CheckConstant, CheckFlow, CheckUnrolling, CheckUnsignedSlot as UnsignedSlot,
};
use nikaia_std::tools::check_items;
use nikaia_std::tools::check_names::integer_named;
use nikaia_std::tools::check_overflow;
use nikaia_std::tools::check_patterns::pattern_names;
use nikaia_std::tools::check_reports::CheckWall;
use nikaia_std::tools::check_shapes::is_growable;
pub use nikaia_std::tools::check_state::{
    CheckFinding as Finding, CheckLabel as Label, CheckSeverity as Severity,
};
use nikaia_std::tools::check_state::{
    CheckGuarded as Guarded, CheckImmutable as Immutable, CheckKind as Kind, CheckLocal as Local,
    CheckPendingCoalesce as PendingCoalesce, CheckPromises as Promises, CheckRepeats as Repeats,
    CheckState, CheckWorld, a_body_never_ends,
};
use nikaia_std::tools::check_texts::{self, ends_in_text};
use nikaia_std::tools::check_widen::{self, names_a_path};
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
        newly,
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
    let proved = crate::prove::prove(parsed, own, library, &keys);
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
    checker.language_impls();
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

/// **The call a named function stands for where a function value is kept**
/// ([`Parsed::kept_function_call`]): one construction for the checker, which
/// types it, and the emitter, which writes it.
pub(crate) fn kept_function_call(parsed: &Parsed, name: Ident, arity: usize) -> (Vec<Ident>, Expr) {
    parsed.kept_function_call(name, arity)
}

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

use nikaia_std::tools::check_arms::CheckArm;
use nikaia_std::tools::check_binds::CheckBoundary as Boundary;
struct Checker<'a> {
    /// **The checker's state that is Nikaia** (`tools/check_state.nika`, #558).
    s: core::Baton,
    /// What it looks at and does not own: the names, and the two ledgers. A
    /// reference, so the moved code can be handed it while the checker is lent.
    world: &'a CheckWorld<'a>,
    /// States with nothing in them, for the crossings into Nikaia (`core.rs`).
    spares: Vec<Box<CheckState>>,
    /// What each callee has **newly** gained since the committed ledger
    /// ([ADR-308](../../docs/specification/adr/adr-308.md) D11). Empty where
    /// there is nothing to compare, which says nothing.
    newly: &'a Newly,
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
            newly: self.newly,
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
            if let Some(value) = value
                && let Some(how @ (Wrap::Widen(_) | Wrap::Unsigned(_))) =
                    self.widened_slot(&tail, expected, Some(value), &span)
            {
                self.s.facts.wrapped.insert(value_node(value), how);
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

    /// **`NK1191`: an arithmetic operator on something that has none**
    /// (0.0.234; a declared `struct` or `enum` too, under the same code).
    ///
    /// Only where a side is **known** to be a list, a map or a set, or a
    /// `struct` or an `enum` this program declares: a value whose type this
    /// checker could not work out is not refused on a guess
    /// ([Part III C.4](../../docs/specification/30-nikaia-tooling.md)).
    ///
    /// **A declared type has no operators because the language gives it no
    /// way to have one**: there is no overloading
    /// ([ADR-094](../../docs/specification/adr/adr-094.md) §1,
    /// [ADR-286](../../docs/specification/adr/adr-286.md)) and no trait an
    /// `impl` could name to give a type `+` (Part I 4.7). `a -= 30` on an
    /// `Account` lowered as it was written and `rustc` said *binary assignment
    /// operation `-=` cannot be applied* (E0368, E0369) about a file nobody
    /// wrote (C.1). A type another file of this program, a package or a
    /// described crate declares is not asked: only this unit's own
    /// declarations are known to be Nikaia's.
    ///
    /// `sides` is each operand with what it was written as, and `compound` is
    /// whether the operator was written `-=`, which is what the help writes back.
    fn arithmetic_on_something_that_has_none(
        &mut self,
        op: BinaryOp,
        sides: [(&Expr, &Ty); 2],
        compound: bool,
        span: &Span,
    ) {
        let [(left_expr, left), (right_expr, right)] = sides;
        self.s.arithmetic_on_something_that_has_none(
            self.world, op, left_expr, left, right_expr, right, compound, span,
        );
    }

    /// A list, a map, a set, or a `struct` or an `enum` this unit declares.
    fn has_no_arithmetic(&self, ty: &Ty) -> bool {
        is_a_collection(ty) || self.declared_with_no_arithmetic(ty).is_some()
    }

    /// `"struct"` or `"enum"` where a type is known to be one **this unit**
    /// declares, the types Part I gives no operator and no way to have one.
    fn declared_with_no_arithmetic(&self, ty: &Ty) -> Option<&'static str> {
        match check_items::declared_with_no_arithmetic(&self.world.here, ty).as_deref() {
            Some("struct") => Some("struct"),
            Some(_) => Some("enum"),
            None => None,
        }
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

    /// **`NK1193`: a pattern that looks inside a part the compiler boxed,
    /// further than the lowering reaches**
    /// ([ADR-246](../../docs/specification/adr/adr-246.md) D4, D5).
    ///
    /// `Expr::Add(Expr::Num(n), b)` asks the part inside a box a question, and
    /// the language below matches no pattern through one. Since D5 the
    /// lowering binds such a part to a name, asks the question in the arm's
    /// guard and takes the part apart in the arm - one level deep, and outside
    /// an `|`. A pattern that looks inside a box *inside* that part, or does
    /// so in one alternative of several, is still refused by name.
    fn a_pattern_inside_a_box(&mut self, pattern: &MatchPattern, span: &Span) {
        self.s.a_pattern_inside_a_box(self.world, pattern, span);
    }

    /// **A guard that reads a name bound inside a boxed part**
    /// ([ADR-246](../../docs/specification/adr/adr-246.md) D5 item 2, #97).
    ///
    /// The guard runs before the arm takes the part apart, so the lowering
    /// binds those names a second time inside the guard - through the box,
    /// by reference - and this records which of them the guard reads, and
    /// which of those are a number, a `bool` or a `char`: those are copied out
    /// for the guard (`let n = *n;`), and the rest stay views of the part,
    /// which is what a guard does with them anyway - it reads.
    fn a_guard_reading_inside_a_box(&mut self, pattern: &MatchPattern, body: &Expr, guard: &Expr) {
        self.nikaia(|state, world, host| {
            state.a_guard_reading_inside_a_box(world, host, pattern, body, guard)
        })
    }

    /// The names a pattern binds **whole** to a boxed part: `a` in
    /// `Expr::Add(a, b)`, which the arm opens and a guard reads through the
    /// box.
    fn boxed_names(&self, pattern: &MatchPattern) -> Vec<String> {
        self.s.boxed_names(self.world, pattern)
    }

    /// The types of the names bound inside the boxed parts a pattern looks
    /// into, where the variant says them: each part is typed by the path its
    /// own pattern names, which is the boxed type.
    fn typed_inside_boxes(&self, pattern: &MatchPattern, out: &mut BTreeMap<String, Ty>) {
        self.s.typed_inside_boxes(self.world, pattern, out);
    }

    /// Whether a call is to the **prelude's `assert`**
    /// ([ADR-269](../../docs/specification/adr/adr-269.md) D2), and not to a
    /// name the program declared itself.
    fn the_prelude_assert(&self, func: &Expr) -> bool {
        matches!(func, Expr::Variable(name, _) if self.parsed.text(*name) == ASSERT)
            && self.s.lookup(ASSERT).is_none()
            && !self.world.own.functions.contains_key(ASSERT)
    }

    fn a_view_compared_with_a_value(
        &mut self,
        lhs: &Expr,
        left: &Ty,
        rhs: &Expr,
        right: &Ty,
        at: &Span,
    ) {
        self.s
            .a_view_compared_with_a_value(self.world, lhs, left, rhs, right, at);
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

    /// `??` over a value that is never absent (`tools/check_fields.nika`).
    fn nothing_to_fall_back_from(
        &mut self,
        value: &Expr,
        left: &Ty,
        coalesce: &Expr,
        span: &Span,
    ) -> bool {
        self.s
            .nothing_to_fall_back_from(self.world, value, left, coalesce, span)
    }

    /// One lambda parameter, bound — and refusable where it is changed without
    /// `mut` ([ADR-281](../../docs/specification/adr/adr-281.md) D12).
    ///
    /// `kasse.update fn(mut v) { v += 100 }` is where the word earns its place:
    /// the block changes the value in place and the caller whose value changes
    /// is the **lock**. D1 says a block that changes `v` without the word is
    /// `NK1138`, and it says it **about a door** — which is where this asks it
    /// and nowhere else.
    ///
    /// **Held for every lambda it would refuse correct programs**, and the
    /// corpus is what said so: `par_fold(…, fn(acc, m) { acc.record(m) })` in
    /// `examples/1brc.nika` changes `acc` and has no `mut`, and it compiles,
    /// because the **emitter** writes the word itself where it recognises a
    /// fold's accumulator; `and_modify fn(tally) { tally.bump() }` in
    /// `k-nucleotide.nika` is the same shape one library over. So *a lambda
    /// parameter changed without `mut` is already broken Rust* is false, and a
    /// rule built on it would be [Part III
    /// C.4](../../docs/specification/30-nikaia-tooling.md)'s correct program
    /// refused. Widening it is a question for whoever takes that `mut` out of
    /// the emitter, and it belongs with that work rather than ahead of it.
    fn lambda_parameter(
        &mut self,
        name: winnow_grammar::Symbol,
        mutable: &[winnow_grammar::Symbol],
        ty: Ty,
        span: &Span,
    ) -> Local {
        self.s.lambda_parameter(self.world, name, mutable, ty, span)
    }

    /// **Whether a type's values have a cleanup**, and whether it can fail
    /// ([ADR-297](../../docs/specification/adr/adr-297.md) D1): an `impl
    /// Cleanup` for it in any ledger this build reads - or for what it holds,
    /// in a field, an element or a nullable, which dies with it. A view is not
    /// the value, and dies nowhere. The type named is the one with the
    /// cleanup; the answer fails where any cleanup it holds can.
    fn cleanup_of(&self, ty: &Ty) -> Option<(String, bool)> {
        self.s
            .cleanup_of(self.world, ty)
            .map(|held| (held.owner, held.fails))
    }

    /// **A value with a cleanup may die in the function being walked**
    /// (ADR-297 D2): its body becomes a settle point's, and the settle is a call
    /// every analysis reads - one that pauses, and one that fails where the
    /// function says `throws` and the cleanup can fail.
    fn a_settle_in_this_function(&mut self, fails: bool) {
        if let Some(key) = self.s.a_settle_in_this_function(fails) {
            self.reached_method(Some(&key));
        }
    }

    /// A value with a cleanup, made by a call or a literal (ADR-297 D2).
    fn a_value_with_a_cleanup_made(&mut self, ty: &Ty) {
        if let Some((_, fails)) = self.cleanup_of(ty) {
            self.a_settle_in_this_function(fails);
        }
    }

    /// **A value with a cleanup, bound by a `let`** (ADR-297 D2, D3): the
    /// block is settled where it ends, the settle is a pause wherever pausing
    /// is refused, and a cleanup that can fail is the function's to declare.
    fn a_value_with_a_cleanup_bound(&mut self, name: &str, ty: &Ty, span: &Span) {
        if let Some(fails) = self.a_value_with_a_cleanup_dying(name, ty, span) {
            self.s.facts.settle_lets.insert(span.at() as i64, fails);
        }
    }

    /// **A value with a cleanup dies in this function**, a `let`'s or a kept
    /// parameter's: the function settles it, the settle is a pause, and a
    /// cleanup that can fail is the function's to declare. Whether it can fail,
    /// where it has a cleanup at all.
    fn a_value_with_a_cleanup_dying(&mut self, name: &str, ty: &Ty, span: &Span) -> Option<bool> {
        let (owner, fails) = self.cleanup_of(ty)?;
        self.a_settle_in_this_function(fails);
        self.s.a_cleanup_that_may_pause(name, span);
        if fails {
            self.s.a_failing_cleanup_undeclared(name, ty, &owner, span);
        }
        Some(fails)
    }

    /// **`NK1164` for `[T: Eq]` and `[T: Ord]`: the argument's type is neither**
    /// ([ADR-341](../../docs/specification/adr/adr-341.md) D4).
    ///
    /// The bound is the language's, so no `impl` in a ledger answers it: the
    /// walk does (`Comparable`), with the program's `impl Eq` and `impl Ord`.
    /// A type parameter of the caller's answers for its own bounds, and a type
    /// nobody could work out answers yes (Part III C.4).
    fn a_standard_bound_not_met(&mut self, key: &str, bound: &BTreeMap<String, Ty>, span: &Span) {
        let Some(wanted) = self.s.declared_bounds.get(key) else {
            return;
        };
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
        let refused = nikaia_std::tools::check_compare::compare_standard_bounds(
            &asking,
            &self.s.type_parameters,
            key,
            wanted,
            bound,
            span,
        );
        self.s.findings.extend(refused);
    }

    /// **`NK1248`: a comparison on a type parameter that no bound allows**
    /// ([ADR-341](../../docs/specification/adr/adr-341.md) D3).
    ///
    /// `fn lt[T](a: T, b: T) -> bool { return a < b }` passed the checker and
    /// reached `rustc` as *binary operation `<` cannot be applied to type `T`*
    /// (Part III C.1). `==` and `!=` need `Eq` or `Ord`, the order operators
    /// need `Ord`; a trait of the program's own with one of those names gives
    /// no operator.
    fn an_operator_needs_a_bound(&mut self, op: BinaryOp, left: &Ty, right: &Ty, at: &Span) {
        self.s
            .an_operator_needs_a_bound(self.world, op, left, right, at);
    }

    /// **`NK1249`: an `impl Eq` or `impl Ord` the language cannot honour**
    /// ([ADR-341](../../docs/specification/adr/adr-341.md) D5).
    ///
    /// For a type another package declares the ledger answers, and `rustc`
    /// would refuse the `impl` as an orphan; and an order for a type that is
    /// not `Eq` is not one `BTreeMap` can use.
    fn language_impls(&mut self) {
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
            nikaia_std::tools::check_compare::compare_language_impls(&asking, &self.world.here);
        self.s.findings.extend(refused);
    }

    /// **`NK1188`: `==` on a type that does not compare**
    /// ([ADR-204](../../docs/specification/adr/adr-204.md) D3).
    ///
    /// The words were `rustc`'s about a file nobody wrote — *binary operation
    /// `==` cannot be applied to type `P`* — with *consider annotating `P` with
    /// `#[derive(PartialEq)]`* as the help, which is a way out the source cannot
    /// take: [Part III C.1 and
    /// C.2](../../docs/specification/30-nikaia-tooling.md) at once.
    ///
    /// **Asked of one side and only where its type is known.** A type this
    /// compiler could not work out compares, which is the answer an absent claim
    /// gets everywhere here ([Part III
    /// C.4](../../docs/specification/30-nikaia-tooling.md)); and the two sides
    /// disagreeing is `NK1102`'s business, one refusal over.
    fn a_type_that_does_not_compare(&mut self, left: &Ty, right: &Ty, span: &Span) {
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
        let refused = nikaia_std::tools::check_compare::compare_refusal(&asking, left, right, span);
        self.s.findings.extend(refused);
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

    /// **A task of `task::scope` runs beside the others** (Part II 12.7):
    /// asked as a walk's lambda is, at both settings, so the program means
    /// the same at each. **And it fails in the scope's channel** (ADR-346 D7).
    pub(super) fn a_task_of_a_scope(&mut self, args: &[Expr]) {
        self.s.in_parallel = self.s.in_parallel.or(Some(self.s.scope.len() as i64));
        self.s.scoped_task = true;
        if let Some(lambda) = args.first() {
            self.s.facts.scope_tasks.insert(value_node(lambda));
        }
    }

    /// **A lambda that pauses, handed to a `std` entry**
    /// ([ADR-233](../../docs/specification/adr/adr-233.md)).
    ///
    /// Every `std` entry that takes a lambda describes a **Rust** signature
    /// whose closure is synchronous, so the call is lowered to the entry's
    /// counterpart in `nikaia_std::seq`, which takes the language below's own
    /// `async` closure (D1, D2). Which counterpart is a question about the
    /// receiver's type, which the emitter has none of, so it is answered here.
    ///
    /// * `map` and `filter` of a sequence hand back **a sequence whose step
    ///   pauses** (D1): nothing is walked here, and whatever walks the result
    ///   pauses - a `for` gives its thread up at each step, and `collect`,
    ///   `count`, `nth` and `join` are awaited, exactly as for `io::lines()`.
    /// * A list's `map`, `sort_by_key`, `or_insert_with` and `and_modify` are
    ///   **eager** (D2): the call itself runs the lambda, so the call pauses.
    ///
    /// The four doors take their lambda **while a lock is held**, where a
    /// pause is `NK2202` by Part II 12.2's own rule - asked of every call
    /// inside a door, not here (D3).
    #[allow(clippy::too_many_arguments)]
    fn a_lambda_that_pauses_at_an_entry(
        &mut self,
        key: &str,
        receiver: &Expr,
        on: &Ty,
        lambda: Option<&Expr>,
        result: Ty,
        span: &Span,
    ) -> Ty {
        // A `Par[T]`'s lambda may not pause at all: `NK2209` has said so.
        if matches!(on, Ty::Seq { parallel: true, .. }) {
            return result;
        }
        let chained = a_paused_chain(key, on);
        let Some(lambda) = lambda else {
            // A plain lambda over a sequence that already pauses: the step
            // still does, so the result is one too.
            return match chained {
                true => a_pausing_sequence(result, on),
                false => result,
            };
        };
        let entry = match (key, chained) {
            ("Seq::map", true) => PausingEntry::Method("then".to_string()),
            ("Seq::filter", true) => PausingEntry::Method("then_filter".to_string()),
            ("Seq::map", false) => PausingEntry::Function("then".to_string()),
            ("Seq::filter", false) => PausingEntry::Function("then_filter".to_string()),
            ("list::ListExt::map", _) => PausingEntry::Function("map_list".to_string()),
            // A `mut` parameter is a `&mut Vec` already, and handed on as it
            // is: a second `&mut` may not be taken of it (ADR-094 D3).
            ("Vec::sort_by_key", _) => match self
                .s
                .flow
                .receiver_name
                .as_deref()
                .and_then(|name| self.s.binding(name))
                .is_some_and(|local| local.changing)
            {
                true => PausingEntry::Function("sort_by_key".to_string()),
                false => PausingEntry::Lent("sort_by_key".to_string()),
            },
            ("Entry::or_insert_with", _) => PausingEntry::Function("or_insert_with".to_string()),
            // **A task of `task::scope` that pauses** (ADR-328 D10): handed to
            // the scope to poll, and the scope around it pauses.
            ("task::Scope::spawn", _) => PausingEntry::Method("spawn_paused".to_string()),
            ("Entry::and_modify", _) => PausingEntry::Function("and_modify".to_string()),
            // An entry with no counterpart keeps the lowering's refusal, which
            // names the call and the way out.
            _ => return result,
        };
        self.s
            .facts
            .pausing_lambdas
            .insert(value_node(lambda), entry);
        if matches!(key, "Seq::map" | "Seq::filter") {
            return a_pausing_sequence(result, on);
        }
        // The eager ones pause where they are called, and the call is one that
        // may pause for every rule that asks: the function around it, the
        // lambda around it, the door around it.
        let written = value_node(receiver);
        self.s.flow.settled_methods.remove(&written);
        self.s.flow.pausing_methods.insert(written);
        let callee = key.to_string();
        self.s.a_call_that_may_pause(&callee, true, false, span);
        result
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

    /// **The open numbers a value is made of**: a name bound to one, through
    /// the operations whose result is of their operands' type - arithmetic, a
    /// bit operation, a negation. A call, a cast and a comparison make a value
    /// of a type of their own and stop it; so does a shift's count, which is
    /// any integer (ADR-285 D9).
    fn open_numbers_in(&self, expr: &Expr, out: &mut Vec<usize>) {
        out.extend(
            self.s
                .open_numbers_in(self.world, expr)
                .into_iter()
                .map(|at| at as usize),
        );
    }

    /// **A value that is nothing but numbers**: literals and open numbers,
    /// joined by the operations [`Self::open_numbers_in`] walks. A `let` of one
    /// is an open number itself (D1).
    fn number_shaped(&self, expr: &Expr) -> bool {
        self.s.number_shaped(self.world, expr)
    }

    /// **A use asks the open numbers in a value for a type** (D2): a type
    /// standing beside it (`beside`), or an index, which takes any integer.
    fn number_asked(&mut self, value: &Expr, want: &Ty, at: &Span, beside: bool) {
        self.s.number_asked(self.world, value, want, at, beside);
    }

    /// **One operation, one type**: the open numbers on both sides of it are
    /// one question from here on (D1).
    fn open_numbers_joined(&mut self, numbers: &[usize]) {
        let numbers: Vec<i64> = numbers.iter().map(|&at| at as i64).collect();
        self.s.open_numbers_joined(&numbers);
    }

    /// **Every open number of a body, decided** once the body has been walked
    /// ([ADR-285](../../docs/specification/adr/adr-285.md) D24, D25, D27).
    fn numbers_typed_by_their_uses(&mut self) {
        self.s.numbers_typed_by_their_uses();
    }

    /// ADR-285 D29.5: a division whose divisor is a constant zero is refused
    /// here, in this language's words.
    ///
    /// It was already refused where it was written, and by the same mechanism as
    /// the sum `constant_fits` takes back: `rustc`'s `unconditional_panic`, with
    /// *"attempt to divide `1_i32` by zero"* about a file nobody wrote
    /// (Part III, C.1).
    ///
    /// **`NK1118`, and the polarity is the fold's.** A divisor that does not
    /// fold to a constant says nothing: a division by something this checker
    /// cannot evaluate is the ordinary case, and it aborts at run time with the
    /// Nikaia line the table names (ADR-300). Only a zero it can *prove* is
    /// refused.
    /// **`NK1116` at the operation that overflows**, wherever it stands
    /// (0.0.251, issue #176): in an `f"…"` hole, a condition, a list,
    /// a receiver - not only where a `let`, a `return` or a parameter stands
    /// beside it. Where a name pinned the type
    /// ([ADR-285](../../docs/specification/adr/adr-285.md) D26) the operation
    /// has one, and a constant that does not fit it is refused here rather than
    /// by `rustc`'s *this arithmetic operation will overflow* about the
    /// generated file.
    ///
    /// **The innermost that overflows, once**: the operands are walked first,
    /// so one that overflowed has already said so for this statement, and the
    /// operation around it says nothing more.
    fn an_operation_that_overflows(
        &mut self,
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        reach: Option<&str>,
        span: &Span,
    ) {
        self.s
            .an_operation_that_overflows(self.world, op, lhs, rhs, reach, span);
    }

    fn divisor_is_not_zero(&mut self, op: BinaryOp, rhs: &Expr, span: &Span) {
        self.s.divisor_is_not_zero(self.world, op, rhs, span);
    }

    // --- statements ---------------------------------------------------------

    /// Walk a block and hand back the type of its tail.
    fn block(&mut self, block: &Block) -> Ty {
        self.nikaia(|state, world, host| {
            check_core_stmt::check_core_block(state, world, host, block)
        })
    }

    /// **`NK2109`: a bound handle is used up before it leaves its scope**
    /// ([ADR-346](../../docs/specification/adr/adr-346.md) D3). A `let h =
    /// spawn …` whose name no later statement of the block mentions is
    /// forgotten: nobody joins, cancels, detaches or hands it on.
    fn a_bound_handle_is_taken(&mut self, block: &Block) {
        for (at, stmt) in block.stmts.iter().enumerate() {
            let Stmt::Let { names, value, .. } = &stmt.node else {
                continue;
            };
            let ([single], Expr::Spawn { .. }) = (names.as_slice(), value) else {
                continue;
            };
            let name = self.parsed.text(*single).to_string();
            if name == "_" {
                continue;
            }
            let rest: Vec<&Stmt> = block.stmts[at + 1..].iter().map(|s| &s.node).collect();
            if matches!(
                self.handle_path(&name, &rest, false),
                HandlePath::Taken | HandlePath::Gone
            ) {
                continue;
            }
            self.s.findings.push(Finding {
                severity: Severity::Error,
                span: stmt.span,
                code: "NK2109".to_string(),
                message: format!("The task handle `{name}` is never joined, cancelled or detached."),
                notes: vec![
                    "An error of a task nobody waits for would be lost, so a bound handle is used up \
                     before its scope ends."
                        .to_string(),
                ],
                help: Some(format!(
                    "Write `{name}.join()` to wait for the task, or `{name}.detach()` if nobody waits for it."
                )),
                labels: Vec::new(),
            });
        }
    }

    /// Whether `name` is mentioned in `stmt`.
    fn handle_named(&self, name: &str, stmt: &Stmt) -> bool {
        let mut seen = std::collections::BTreeSet::new();
        nikaia_std::tools::names::names_in_statement(stmt, &self.parsed.interner, &mut seen);
        seen.contains(name)
    }

    /// Whether `name` is mentioned in `expr`.
    fn handle_named_in(&self, name: &str, expr: &Expr) -> bool {
        self.handle_named(name, &Stmt::Expr(expr.clone()))
    }

    /// **Every path through `stmts` takes the handle `name`** (`NK2109`):
    /// a mention takes it (a `join`, `detach`, `cancel`, or a hand-over);
    /// a `return`, `throw`, or a `break`/`continue` outside a nested loop
    /// that does not mention it leaves it behind; branches must all take it.
    fn handle_path(&self, name: &str, stmts: &[&Stmt], in_loop: bool) -> HandlePath {
        for stmt in stmts {
            let flow = match stmt {
                Stmt::Return(value) => match value {
                    Some(v) if self.handle_named_in(name, v) => HandlePath::Gone,
                    _ => HandlePath::Left,
                },
                Stmt::Break | Stmt::Continue if !in_loop => HandlePath::Left,
                Stmt::Expr(e) => self.handle_expr_path(name, e, in_loop),
                Stmt::Let { value, .. } => self.handle_expr_path(name, value, in_loop),
                Stmt::For { iter, body, .. } => {
                    if self.handle_named_in(name, iter) {
                        HandlePath::Taken
                    } else {
                        self.handle_loop_body(name, body)
                    }
                }
                Stmt::While { cond, body } => {
                    if self.handle_named_in(name, cond) {
                        HandlePath::Taken
                    } else {
                        self.handle_loop_body(name, body)
                    }
                }
                other => {
                    if self.handle_named(name, other) {
                        HandlePath::Taken
                    } else {
                        HandlePath::Falls
                    }
                }
            };
            if flow != HandlePath::Falls {
                return flow;
            }
        }
        HandlePath::Falls
    }

    /// A loop body: a mention anywhere takes the handle, a `return` or
    /// `throw` before any mention leaves it.
    fn handle_loop_body(&self, name: &str, body: &Block) -> HandlePath {
        let stmts: Vec<&Stmt> = body.stmts.iter().map(|s| &s.node).collect();
        match self.handle_path(name, &stmts, true) {
            HandlePath::Left => HandlePath::Left,
            HandlePath::Gone => HandlePath::Taken,
            _ if stmts.iter().any(|s| self.handle_named(name, s)) => HandlePath::Taken,
            _ => HandlePath::Falls,
        }
    }

    fn handle_expr_path(&self, name: &str, expr: &Expr, in_loop: bool) -> HandlePath {
        let block = |b: &Block| {
            let stmts: Vec<&Stmt> = b.stmts.iter().map(|s| &s.node).collect();
            self.handle_path(name, &stmts, in_loop)
        };
        let branches: Vec<HandlePath> = match expr {
            Expr::Throw(inner, _) => {
                return if self.handle_named_in(name, inner) {
                    HandlePath::Gone
                } else {
                    HandlePath::Left
                };
            }
            Expr::Return(value, _) => {
                return match value.as_ref() {
                    Some(v) if self.handle_named_in(name, v) => HandlePath::Gone,
                    _ => HandlePath::Left,
                };
            }
            Expr::Block(b, _) => return block(b),
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                if self.handle_named_in(name, cond) {
                    return HandlePath::Taken;
                }
                vec![
                    block(then_branch),
                    else_branch.as_ref().map_or(HandlePath::Falls, block),
                ]
            }
            Expr::Match { value, arms, .. } => {
                if self.handle_named_in(name, value) {
                    return HandlePath::Taken;
                }
                arms.iter()
                    .map(|arm| {
                        if arm
                            .guard
                            .as_ref()
                            .is_some_and(|g| self.handle_named_in(name, g))
                        {
                            HandlePath::Taken
                        } else {
                            self.handle_expr_path(name, &arm.body, in_loop)
                        }
                    })
                    .collect()
            }
            other => {
                return if self.handle_named_in(name, other) {
                    HandlePath::Taken
                } else {
                    HandlePath::Falls
                };
            }
        };
        // A branch that left the scope with the handle does not go on.
        let on: Vec<HandlePath> = branches
            .into_iter()
            .filter(|b| *b != HandlePath::Gone)
            .collect();
        if on.is_empty() {
            HandlePath::Gone
        } else if on.iter().all(|b| *b == HandlePath::Taken) {
            HandlePath::Taken
        } else if on.iter().all(|b| *b == HandlePath::Falls) {
            HandlePath::Falls
        } else {
            HandlePath::Left
        }
    }

    // --- expressions --------------------------------------------------------

    fn expr(&mut self, expr: &Expr, span: &Span) -> Ty {
        self.s.flow.reached = self.s.flow.reaching.take();
        let said_before = self.s.flow.unknown_said;
        let ty = self.value_of(expr, span);
        let said_inside = self.s.flow.unknown_said > said_before;
        // **A call on a number written as its receiver is as open as the
        // number** (#417): `7.wrapping_add(1)` is whatever integer its use
        // asks for, as `7 + 1` is, so its type is not claimed here and the
        // first use that asks decides the literal's.
        let ty = match expr {
            Expr::MethodCall { receiver, .. } if matches!(**receiver, Expr::LitInt { .. }) => {
                let Expr::LitInt {
                    value, negative, ..
                } = **receiver
                else {
                    unreachable!("matched above")
                };
                let key = (
                    value_node(receiver),
                    nikaia_std::tools::integers::integer(value, negative),
                );
                let own = self.s.facts.unsigned_literals.get(&key.0).cloned();
                let its_own = |ty: &Ty| {
                    own.as_deref()
                        .is_some_and(|own| matches!(ty, Ty::Named { name, args, view: false } if name == own && args.is_empty()))
                };
                let open = match &ty {
                    Ty::Nullable(inner) => its_own(inner),
                    other => its_own(other),
                };
                match open {
                    true => {
                        self.s.flow.literal_receivers.push(key);
                        Ty::Unknown
                    }
                    false => ty,
                }
            }
            _ => ty,
        };
        // **`get` on a map of `T?` is one `T?`, as the brackets are**
        // ([ADR-293](../../docs/specification/adr/adr-293.md) D12): the
        // signature hands back `V?`, and a `V` that is a `T?` would make it a
        // `T?` of a `T?`.
        let ty = match (expr, ty) {
            (Expr::MethodCall { method, args, .. }, Ty::Nullable(outer))
                if self.parsed.text(*method) == "get"
                    && args.len() == 1
                    && matches!(outer.as_ref(), Ty::Nullable(_)) =>
            {
                self.s.facts.flat_reads.insert(value_node(expr));
                *outer
            }
            (_, ty) => ty,
        };
        if self.s.flow.read_a_map
            && (matches!(expr, Expr::Index { .. })
                || matches!(expr, Expr::MethodCall { method, .. }
                    if self.parsed.text(*method) == "get"))
        {
            self.s
                .flow
                .map_read_sites
                .insert((value_node(expr) as usize) as i64);
        }
        // **A value with a cleanup made here may die in this function**
        // (ADR-297 D2): by a call or a literal, which is where one comes from.
        // A read of a name or a field is not a death, and is not asked.
        if matches!(
            expr,
            Expr::Call { .. }
                | Expr::MethodCall { .. }
                | Expr::SafeMethod { .. }
                | Expr::StructLit { .. }
        ) {
            self.a_value_with_a_cleanup_made(&ty);
        }
        if ty.is_unknown() && !said_inside {
            self.a_value_of_no_known_type(expr, span);
        }
        ty
    }

    /// **`NK1245`: an expression whose type the compiler does not know**
    /// ([ADR-286](../../docs/specification/adr/adr-286.md) D41, #497). What is
    /// not known is not checked, and a mistake there reached the language below
    /// as an error about a file nobody wrote (Part III C.1). Not asked of what
    /// is typed later by its uses (a number literal, an open number, an integer
    /// `comptime` without a type), of an expression that is unknown only
    /// because a part of it is (an operator, a branch, a block, a jump), of a
    /// `panic`, of a reflected field's `of` in a generic body, and not again
    /// where a part inside was already refused: one cause, one refusal.
    fn a_value_of_no_known_type(&mut self, expr: &Expr, span: &Span) {
        self.s.a_value_of_no_known_type(self.world, expr, span);
    }

    fn value_of(&mut self, expr: &Expr, span: &Span) -> Ty {
        match expr {
            // A bare number fits every numeric type, exactly as it does in the
            // language below. Committing it to one here would make `add(3)`
            // wrong wherever the parameter is not that one.
            Expr::LitInt { .. } => Ty::Unknown,
            // **A float literal is an `f64`** (#503): it is the only float type
            // (Part I 2.2), so there is nothing to choose between and nothing
            // a use could decide. `let mut px = 0.0` is an `f64`, and so is
            // everything computed from it.
            Expr::LitFloat(_, _) => Ty::named("f64"),
            // Part I 2.4 calls a string literal a `String`; Stage 0 emits a
            // Rust string literal, which is a view of static text. The checker
            // says what is emitted - see ADR-024 D5.
            //
            // **The type comes from the syntax** (ADR-309 D2). `"…"` is a view
            // of static text and `f"…"` is a `format!`, which is a `String` -
            // and which one a literal is can be read off its first character
            // rather than worked out from whether somebody happened to type a
            // brace somewhere in it.
            Expr::LitStr { text, .. } => {
                self.an_escape_nothing_names(text, span);
                self.unmarked_hole(text, span);
                Ty::view("str")
            }
            // **A hole is checked like anything else** (ADR-309 D13). It is
            // Nikaia source written inside a literal, and until that walk
            // existed it was source no analysis could see - the same mistake
            // was caught outside a hole and silently passed inside one.
            Expr::LitInterpolated { parts, .. } => {
                // **The literal's own text runs**: a hole is code, and a
                // string inside one is a literal of its own, asked when the
                // hole is walked (ADR-309 D7).
                for part in parts {
                    if let crate::ast::FPart::Text(text) = part {
                        self.an_escape_nothing_names(text, span);
                    }
                }
                self.holes(expr, span);
                // **With a file's name in a hole it builds a name** (ADR-319
                // D4): the parts are joined in the platform's encoding, and a
                // position that asks for text refuses it at the hole
                // (`a_name_in_text`).
                match self.s.facts.path_holes.contains_key(&value_node(expr)) {
                    true => Ty::named("fs::Path"),
                    false => Ty::named("String"),
                }
            }
            // A character literal is kept as written too, so `'\q'` is the
            // same refusal one literal over.
            Expr::LitChar(text, _) => {
                self.an_escape_nothing_names(text, span);
                Ty::named("scalar")
            }
            Expr::LitBool(_, _) => Ty::named("bool"),
            // **A nullable of it-does-not-say.** `null` names the absence of a
            // value without naming what value, so the inside is `Unknown` and
            // anything nullable fits it - which is what makes `let mut m: &str?
            // = null` right and what keeps `let m = null` from being refused
            // here: `let mut m = null` then `m = "hi"` is a correct program
            // (Part III, C.4), and it is `rustc` that asks for an annotation
            // where nothing ever says.
            Expr::LitNull(_) => Ty::Nullable(Box::new(Ty::Unknown)),

            Expr::Variable(name, _) => {
                let name = self.parsed.text(*name);
                // `NK2101`: where this name is read, for the question a `spawn`
                // asks about what comes after it.
                self.s.read_at.push((name.to_string(), span.at() as i64));
                // **D2's question, answered where every name is read**
                // ([ADR-135](../../docs/specification/adr/adr-135.md)): an
                // empty list takes its element type from its first use, so a
                // use - any use - is what says one exists.
                if let Some(at) = self
                    .s
                    .binding(name)
                    .and_then(|local| local.empty_list.map(|n| n as usize))
                {
                    self.s.flow.empty_lists.remove(&(at as i64));
                }
                match self.s.lookup(name) {
                    Some(ty) => {
                        self.a_read(expr, name.to_string(), span);
                        self.s.a_sequence_is_taken(name, &ty, span);
                        ty
                    }
                    // **`assert` is called** (ADR-269 D2): the statement
                    // form Part III 14.1 used to write, `assert c`, is the
                    // name alone and then `c`, and the name is only ever the
                    // head of a call.
                    None if self.the_prelude_assert(expr) => {
                        self.s.an_assert_of_another_shape(
                            "`assert` is a function, and is called: write `assert(cond)`"
                                .to_string(),
                            span,
                        );
                        Ty::Unknown
                    }
                    // **A function or a type's constructor named as a value**
                    // (#502, #507): `xs.map(double)`, `par_fold(M, Summary, …)`
                    // - what its declaration says it takes and gives.
                    None if self.s.function_as_a_value(self.world, name).is_some() => self
                        .s
                        .function_as_a_value(self.world, name)
                        .unwrap_or(Ty::Unknown),
                    None => {
                        // **The specific message wins.** A free `a`, `b` or `c`
                        // is the mistake a reader of the old specification
                        // makes, and `withdrawn_automatic_name` says what
                        // happened to the form rather than only that the name
                        // is unknown (ADR-277). Where it has spoken, the
                        // general refusal would be a second finding about one
                        // mistake.
                        if !self.s.withdrawn_automatic_name(name, span) {
                            self.s.nothing_declares_it(self.world, expr, span);
                        }
                        Ty::Unknown
                    }
                }
            }

            Expr::Path(segments, _) => {
                // `Op::Times` is a value of the enum that declares it. Anything
                // else a path can name, this compiler does not resolve.
                let names: Vec<&str> = segments.iter().map(|s| self.parsed.text(*s)).collect();
                // **A constructor handed over as a value** is the other half of
                // [ADR-140](../../docs/specification/adr/adr-140.md) D2, and the
                // one that record names: `par_fold(M, Summary::new, …)` is how
                // `1brc.nika` wrote it, against a type declaring an anonymous
                // constructor and no `new`.
                self.s
                    .a_constructor_written_as_new(self.world, &names.join("::"), span);
                match names.as_slice() {
                    [ty, variant] if self.s.is_variant(ty, variant) => Ty::named(*ty),
                    // **A method named as a value** is a function whose first
                    // parameter is the receiver (#507): `Summary::merge`.
                    [ty, method]
                        if self
                            .s
                            .function_as_a_value(self.world, &format!("{ty}::{method}"))
                            .is_some() =>
                    {
                        self.s
                            .function_as_a_value(self.world, &format!("{ty}::{method}"))
                            .unwrap_or(Ty::Unknown)
                    }
                    // **A `std` enum's case is a value of that enum** (#507):
                    // `fs::Root::Anywhere` is an `fs::Root`, from the ledger's
                    // `variants` column, as `Op::Times` is an `Op` from the
                    // declaration. Only a case that holds nothing: one that
                    // holds something is its constructor.
                    [module, ty, variant] => {
                        // And a package's (#497): `http::Method::Post`, from
                        // the ledger the program absorbed it into.
                        let owner = self.parsed.unaliased(&format!("{module}::{ty}"));
                        match [self.world.library, self.world.own].iter().any(|ledger| {
                            ledger.types.get(&owner).is_some_and(|t| {
                                t.variants
                                    .iter()
                                    .any(|v| v.name == *variant && v.holds.is_empty())
                            })
                        }) {
                            true => Ty::named(owner),
                            false => Ty::Unknown,
                        }
                    }
                    // **`T::fields` is a list of the type's fields**
                    // ([ADR-304](../../docs/specification/adr/adr-304.md) D2,
                    // built by [ADR-304](../../docs/specification/adr/adr-304.md)).
                    // What it hands back is a sequence, so the `for` over it is
                    // the `for` this checker already reads — D4's *one loop*,
                    // arrived at rather than added.
                    [ty, member]
                        if self.s.walks_fields.values().any(|p| p == ty)
                            && self.shape_element(ty, member).is_some() =>
                    {
                        Ty::Named {
                            name: "Vec".to_string(),
                            args: vec![Ty::named(
                                self.shape_element(ty, member).unwrap_or_default(),
                            )],
                            view: false,
                        }
                    }
                    [ty, member] => {
                        self.a_member_this_type_does_not_have(ty, member, span);
                        Ty::Unknown
                    }
                    _ => Ty::Unknown,
                }
            }

            // A `seq` block is a block for every purpose but one: what it says
            // is about the *order* its statements run in (ADR-292 D7), not
            // about what any of them mean or what it hands back.
            // **`unsafe { … }` is a block with a value and no other rule**
            // ([ADR-302](../../docs/specification/adr/adr-302.md) D3): what is
            // inside is checked exactly as anything else is.
            Expr::Block(block, _) => self.block(block),

            // **`unsafe { … }` is a block with a value and no other rule**
            // ([ADR-302](../../docs/specification/adr/adr-302.md) D3): what is
            // inside is checked exactly as anything else is. The one thing it
            // changes is that a call to an `extern` name is allowed here, which
            // is the whole of what the word buys.
            Expr::Unsafe(block, _) => {
                let outer = std::mem::replace(&mut self.s.inside_unsafe, true);
                let value = self.block(block);
                self.s.inside_unsafe = outer;
                value
            }

            // **Part I 8.1.2: each statement is a branch, and the block's value
            // is the tuple of their results in written order**
            // ([ADR-292](../../docs/specification/adr/adr-292.md) D2).
            //
            // Walked here rather than through the `Block` arm above, because a
            // block hands back its *last* statement and this hands back all of
            // them. The scope frame is the block's own, so a name a branch binds
            // does not leak - and `branches_meet_on_nothing` is what refuses one
            // that binds at all, since the block's value already carries it.
            Expr::Overlap(block, _) => {
                self.s.scope.push(Vec::new());
                let parts: Vec<Ty> = block
                    .stmts
                    .iter()
                    .map(|stmt| {
                        // Each branch is an `async` block of its own (ADR-292
                        // D2), so the boundary is per branch rather than per
                        // block.
                        let saved = self.enter_boundary("`overlap` branch");
                        let ty = match &stmt.node {
                            Stmt::Expr(value) => self.expr(value, &stmt.span),
                            other => {
                                self.nikaia(|state, world, host| {
                                    check_core_stmt::check_core_stmt(
                                        state, world, host, other, &stmt.span,
                                    )
                                });
                                Ty::Unknown
                            }
                        };
                        self.leave_boundary(saved);
                        ty
                    })
                    .collect();
                self.s.scope.pop();
                self.branches_meet_on_nothing(block, span);
                Ty::Tuple(parts)
            }

            // **Part II 12.4: every arm is started and the first to finish
            // wins** ([ADR-292](../../docs/specification/adr/adr-292.md) D12).
            //
            // Each raced expression is an `async` block of its own, exactly as
            // an `overlap` branch is, so the boundary is per arm. What the arm
            // *binds* is the value that came back, so the body is walked in a
            // scope holding that one name - and `_` binds nothing, which is
            // [ADR-291](../../docs/specification/adr/adr-291.md) D6's ignore
            // pattern rather than a catch-all arm.
            //
            // **The type is the `match` rule one construct over**: an arm that
            // jumps is not one of the types that have to agree
            // ([ADR-276](../../docs/specification/adr/adr-276.md) D20), and
            // where the rest do not say the same the answer is `Unknown`
            // rather than a guess. Part II 12.4's own example is two jumping
            // arms, so it is a `select` of no value at all.
            Expr::Select(arms, _) => {
                let mut result: Option<Ty> = None;
                let mut agree = true;
                for arm in arms {
                    let saved = self.enter_boundary("`select` arm");
                    let raced = self.expr(&arm.value, span);
                    self.leave_boundary(saved);
                    let frame = match arm.binding {
                        Some(name) => {
                            vec![self.local_free(self.parsed.text(name).to_string(), raced)]
                        }
                        None => Vec::new(),
                    };
                    self.s.scope.push(frame);
                    let saved = self.enter_boundary("`select` arm");
                    let ty = self.block(&arm.body);
                    self.leave_boundary(saved);
                    self.s.scope.pop();
                    if self.s.block_leaves(self.world, &arm.body) {
                        continue;
                    }
                    // An arm of no known type is an open number beside the
                    // others (`=> { -1 }`), or was refused itself (#497).
                    if ty.is_unknown() {
                        continue;
                    }
                    match &result {
                        None => result = Some(ty),
                        Some(seen) if *seen == ty => {}
                        Some(_) => agree = false,
                    }
                }
                match result {
                    Some(ty) if agree => ty,
                    _ => Ty::Unknown,
                }
            }

            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let cond_ty = self.expr(cond, span);
                let at = self.span_at(cond, *span);
                self.expect_bool(&cond_ty, &at, "An `if` decides on a `bool`.");
                // **A decision taken on what a lock said is stale too**
                // ([ADR-281](../../docs/specification/adr/adr-281.md) D25's
                // second shape). It reaches inward and **accumulates**: a
                // nested `if` under a stamped one is still under it, and a
                // plain condition inside does not clear the outer one.
                let outer_condition = self.s.stamped_condition;
                if cond_ty.is_seen() {
                    self.s.stamped_condition = Some(span.at() as i64);
                }
                let choice = self.s.flow.next_choice;
                self.s.flow.next_choice += 1;
                self.s.branch.push((choice, 0));
                let from = self.s.taken_so_far();
                let then = self.block(then_branch);
                // **A `continue` or a `break` leaves the turn as a `return`
                // leaves the function**: what the branch took is not read
                // after it on any path (found moving `types` into Nikaia,
                // #125). A name from outside the loop is the loop rule's, which
                // counts the hand-over itself.
                let leaves = self.s.block_exits(self.world, then_branch)
                    || self.s.block_leaves(self.world, then_branch);
                self.s.a_branch_that_leaves(from, leaves, span);
                self.s.branch.pop();
                let branches = match else_branch {
                    Some(otherwise) => {
                        self.s.branch.push((choice, 1));
                        let from = self.s.taken_so_far();
                        let other = self.block(otherwise);
                        let leaves = self.s.block_exits(self.world, otherwise)
                            || self.s.block_leaves(self.world, otherwise);
                        self.s.a_branch_that_leaves(from, leaves, span);
                        self.s.branch.pop();
                        // Only when both arms agree is there something to say
                        // - or when they meet at text (ADR-282 D4).
                        // **An arm that leaves gives no value** (#375):
                        // `let t = if c { name } else { return }` is the other
                        // arm's type, as a `match` arm that returns is.
                        let leaves = |block: &Block| {
                            self.s.block_exits(self.world, block)
                                || self.s.block_leaves(self.world, block)
                        };
                        // **Each arm's type is kept for the position the value
                        // goes into** (#596).
                        let (then_leaves, other_leaves) = (leaves(then_branch), leaves(otherwise));
                        for (block, ty, leaving) in [
                            (then_branch, &then, then_leaves),
                            (otherwise, &other, other_leaves),
                        ] {
                            if let (Some(tail), false) = (tail_of(block), leaving) {
                                self.s
                                    .arm_types
                                    .insert(value_node(peeled(tail)), ty.clone());
                            }
                        }
                        if then == other || (other_leaves && !then_leaves) {
                            then
                        } else if then_leaves && !other_leaves {
                            other
                        } else if let Some(typed) = self.a_number_beside_a_number(
                            (tail_of(then_branch), &then),
                            (tail_of(otherwise), &other),
                            span,
                        ) {
                            typed
                        } else {
                            let arms = [(tail_of(then_branch), then), (tail_of(otherwise), other)];
                            let met = self
                                .arms_meet_at_text(&arms, span)
                                .or_else(|| self.arms_meet_at_null(&arms));
                            if met.is_none() {
                                self.arms_that_disagree(expr, &arms, span);
                            }
                            met.unwrap_or(Ty::Unknown)
                        }
                    }
                    // An `if` with no `else` is a statement's worth of value.
                    None => Ty::Unknown,
                };
                self.s.stamped_condition = outer_condition;
                branches
            }

            Expr::Match { value, arms, .. } => {
                let on = self.expr(value, span);
                // `error` in a handler is untyped at its binding; where one
                // type arrives, that is what its parts are read from.
                let typed = match (&on, &**value) {
                    (Ty::Unknown, Expr::Variable(name, _))
                        if self.parsed.text(*name) == "error" && !self.s.flow.caught_several =>
                    {
                        self.s.flow.caught_one.clone().unwrap_or(Ty::Unknown)
                    }
                    // Several: each arm's pattern names its member (D17, D18).
                    (on, _) if several_errors(on).is_some() => Ty::Unknown,
                    _ => on.clone(),
                };
                let lendable = self.an_owned_place(value, &typed);
                // **A `match` over a `mut` parameter, or a place in one**
                // (#576): below it is a `&mut`, so the arms bind references,
                // and an arm that takes a part, or reads a number it binds,
                // works on the value instead.
                let mut_subject = self.s.a_mut_parameter_place(self.world, value);
                // The parameter itself is the `&mut` the arms bind through; a
                // field of one is a place the arms bind out of.
                let through_the_reference =
                    mut_subject.is_some() && matches!(value.as_ref(), Expr::Variable(_, _));
                // **A field reached through a view is matched as the view it
                // is** (found moving `traits` into Nikaia, #125): `match
                // item.node` for a lent `item` is a place behind a reference
                // below, and a pattern that binds a part of it moved it out -
                // `rustc`'s *cannot move out of `item.node`*. Its parts are
                // views here, so the emitter matches `&item.node`. **And one
                // reached through the brackets of a view** is the same place:
                // `match block.stmts[i].node` over a `ref Block` bound its
                // parts by reference below while they were values here (found
                // moving the compiler's `views` into Nikaia, #125).
                let through_a_view = {
                    let mut root = &**value;
                    while let Expr::Field { base, .. } | Expr::Index { base, .. } = root {
                        root = base;
                    }
                    matches!(&**value, Expr::Field { .. } | Expr::Index { .. })
                        && matches!(root, Expr::Variable(name, _)
                            if self.s.binding(self.parsed.text(*name))
                                .is_some_and(|local| local.lent || local.ty.is_a_view()))
                };
                let typed = match through_a_view
                    && !typed.is_a_view()
                    && self.s.takes_away(self.world, &typed)
                {
                    true => {
                        self.s.facts.lent_scrutinees.insert(value_node(value));
                        // And read as the view it is below, so a part that
                        // copies is copied out at the head of the arm.
                        view_of(&typed)
                    }
                    false => typed,
                };
                // **And so is a name lent to this function's walk**
                // (`Local::lent`): `target` bound by `Stmt::Assign { target,
                // .. }` over a lent statement is a reference below whatever
                // its type says, so `match target { Expr::Variable(name, _) =>
                // … }` binds a `&Symbol` there. Read as the view it is, a part
                // that copies is copied out at the head of the arm - it was
                // handed on as the reference, and `rustc` answered *expected
                // `Symbol`, found `&Symbol`* (found moving the compiler's
                // `views` into Nikaia, #125).
                // **And a `let` over a place** (#567): `let v = vs[0]` is a
                // `&V` below whatever type it has here.
                let lent_name = matches!(&**value, Expr::Variable(name, _)
                    if self.s.binding(self.parsed.text(*name)).is_some_and(|local| local.lent))
                    || self.s.a_lent_let(self.world, value);
                let typed = match lent_name
                    && !typed.is_a_view()
                    && self.s.takes_away(self.world, &typed)
                {
                    true => view_of(&typed),
                    false => typed,
                };
                // **A `match` is a condition too** (ADR-281 D25).
                let outer_condition = self.s.stamped_condition;
                if on.is_seen() {
                    self.s.stamped_condition = Some(span.at() as i64);
                }
                let mut result: Option<Ty> = None;
                let mut agree = true;
                let mut answered: Vec<(Option<&Expr>, Ty)> = Vec::new();
                let choice = self.s.flow.next_choice;
                self.s.flow.next_choice += 1;
                // **An error, one of several, is taken apart by member**
                // (ADR-280 D30, D17): an arm whose pattern names a member's
                // variant reads its parts from that member.
                let several: Option<Vec<Ty>> = several_errors(&on);
                for (taken, arm) in arms.iter().enumerate() {
                    let typed = match &several {
                        Some(members) => {
                            let owner = match &arm.pattern {
                                MatchPattern::Path(path)
                                | MatchPattern::Tuple { path, .. }
                                | MatchPattern::Named { path, .. }
                                    if path.len() >= 2 =>
                                {
                                    path[..path.len() - 1]
                                        .iter()
                                        .map(|s| self.parsed.text(*s))
                                        .collect::<Vec<_>>()
                                        .join("::")
                                }
                                _ => String::new(),
                            };
                            members
                                .iter()
                                .find(|m| m.text() == owner)
                                .cloned()
                                .unwrap_or(Ty::Unknown)
                        }
                        None => typed.clone(),
                    };
                    // **Every alternative of an or-pattern binds the same
                    // names** ([ADR-291](../../docs/specification/adr/adr-291.md)
                    // D1), asked before the body is walked so that the body's
                    // scope is one this checker can stand behind.
                    self.s
                        .an_or_pattern_that_binds_unevenly(self.world, &arm.pattern, span);
                    // **And a pattern that names a variant the type does not
                    // have**, which the exhaustiveness check below cannot say:
                    // it reads which variants were *covered*, and a misspelling
                    // covers none, so what it reports is the variant that is
                    // missing rather than the name that is wrong — and with an
                    // `else` arm beside it, nothing at all.
                    self.a_pattern_naming_a_member_a_type_does_not_have(&arm.pattern, span);
                    self.a_pattern_inside_a_box(&arm.pattern, span);
                    if let Some(guard) = &arm.guard {
                        self.a_guard_reading_inside_a_box(&arm.pattern, &arm.body, guard);
                    }
                    // **Over a place this function owns, a part is lent**
                    // (ADR-291): the bindings are typed from the
                    // variant, so that what the arm keeps is seen as kept.
                    // **Over a value the `match` was lent, a part is a view
                    // of it** (0.0.252), and a number is its copy: `why` in
                    // `Refused::Because(why)` over a `ref self` is a view of
                    // text. Untyped, `why.clone()` was a method on nothing
                    // known, and a function that called one could not be
                    // `sync` - found moving the template splitter into Nikaia,
                    // where it made an `Error`'s `message` a future.
                    let parts = match (lendable, typed.is_a_view()) {
                        (true, _) => self.s.pattern_parts(self.world, &arm.pattern, &typed),
                        (false, true) => self
                            .s
                            .pattern_parts(self.world, &arm.pattern, &typed)
                            .into_iter()
                            .map(|(name, ty)| match self.s.copied(self.world, &ty) {
                                true => (name, ty),
                                false => (name, view_of(&ty)),
                            })
                            .collect(),
                        // **Over a value made here, a part is the value's own**
                        // (issue #171): `match g(n) { R::Value(c) =>
                        // c, … }` moves the temporary apart, so `c` is the
                        // variant's `C`. Untyped, the whole `match` was
                        // unknown, a method on what it bound resolved to
                        // nothing, and the function around it was inferred to
                        // pause - found moving `fold` into Nikaia.
                        (false, false) => self.s.pattern_parts(self.world, &arm.pattern, &typed),
                    };
                    // **A number bound out of a value the `match` was lent**
                    // is copied out at the head of the arm (0.0.236): the
                    // language below binds a `&i64` there, and
                    // `Expr::Num(n) => n` handed back a reference where the
                    // function says `i64` - `rustc`'s *mismatched types*
                    // about a file nobody wrote.
                    // **And so is a part declared as a view**, `kind: ref
                    // String` in `Line::Table`: a view copies, and bound
                    // through a lent value it was a view of a view - `kind ==
                    // "fn"` compared a `&&str` with a `str` (found moving the
                    // ledger's reader onto a grammar).
                    // **And so is one bound inside a boxed part** (ADR-246
                    // D5): the arm opens the box it was lent, and what it
                    // takes apart is a view too - `Expr::Add(Expr::Num(n), b)
                    // => n` over a `ref Expr` handed back a `&i64`.
                    if typed.is_a_view() || through_the_reference {
                        let at = value_node(&arm.body);
                        let mut parts = self.s.pattern_parts(self.world, &arm.pattern, &typed);
                        self.typed_inside_boxes(&arm.pattern, &mut parts);
                        for (name, ty) in parts {
                            if self.s.copied(self.world, &ty) || ty.is_a_view() {
                                self.s.facts.copied_bindings.insert((at, name));
                            }
                        }
                    }
                    // **And such a part is a view the language below lent**
                    // (`Local::lent`), whatever its type says: `name` in
                    // `S::Named { name, .. }` over a `ref S` is an
                    // `&Option<String>` below, where a `ref String?` parameter
                    // is an `Option<&str>`. The two type alike here, and a call
                    // that hands the first to the second needs the option
                    // opened (found moving `Ty` into Nikaia, ADR-294).
                    let lent_parts = !lendable && typed.is_a_view();
                    // **Owned text bound through a lent value is a `&String`
                    // below**, though it types as the view of text a `ref
                    // String` parameter is (a `&str` there): so it is lent,
                    // and a comparison with a value reads it - `(x ?? "") ==
                    // wanted` for `Ask::Changes(wanted)` over a `ref Ask`
                    // compared a `String` with a `&String` (found moving
                    // `bounds`' body questions into Nikaia, #125).
                    let owned_text: BTreeSet<String> = match lent_parts {
                        true => self
                            .s
                            .pattern_parts(self.world, &arm.pattern, &typed)
                            .into_iter()
                            .filter(|(_, ty)| {
                                matches!(ty, Ty::Named { name, view: false, .. }
                                    if ty::base(name) == "String")
                            })
                            .map(|(name, _)| name)
                            .collect(),
                        false => BTreeSet::new(),
                    };
                    let frame = self
                        .pattern_bindings(&arm.pattern)
                        .into_iter()
                        .map(|local| match parts.get(&local.name) {
                            Some(ty) => {
                                let text = owned_text.contains(&local.name);
                                let mut bound = self.local_free(local.name, ty.clone());
                                bound.lent = lent_parts && (!self.s.copied(self.world, ty) || text);
                                bound
                            }
                            None => local,
                        })
                        .collect();
                    self.s.scope.push(frame);
                    self.s.branch.push((choice, taken as i64));
                    let from = self.s.taken_so_far();
                    let changed_from = self.s.changed.len();
                    // **The guard is walked inside the arm's scope** (D2): it
                    // reads the names the pattern bound, and a condition is a
                    // `bool` here exactly as anywhere else.
                    if let Some(guard) = &arm.guard {
                        // **A part bound whole out of a box is a view of it
                        // in the guard** (ADR-246 D5 item 2, #97): the guard
                        // runs before the arm opens the box, so the lowering
                        // reads it through the box (`peek`), and a comparison
                        // with a value reads the view as `Expr::Add(a, _) if
                        // a == Expr::Num(1)` means.
                        let boxed = self.boxed_names(&arm.pattern);
                        let mut was: Vec<(usize, Ty, bool)> = Vec::new();
                        if let Some(frame) = self.s.scope.last_mut() {
                            for (at, local) in frame.iter_mut().enumerate() {
                                if boxed.contains(&local.name) {
                                    was.push((at, local.ty.clone(), local.lent));
                                    local.ty = view_of(&local.ty);
                                    local.lent = true;
                                }
                            }
                        }
                        let found = self.expr(guard, span);
                        if let Some(frame) = self.s.scope.last_mut() {
                            for (at, ty, lent) in was {
                                frame[at].ty = ty;
                                frame[at].lent = lent;
                            }
                        }
                        let at = self.span_at(guard, *span);
                        self.expect_bool(&found, &at, "A `match` guard is a condition.");
                    }
                    let ty = self.expr(&arm.body, span);
                    self.a_part_lent_where_it_is_not_kept(
                        &arm.body,
                        &parts,
                        from,
                        changed_from,
                        // **A name that is a view binds by reference by itself**
                        // (D30): `let held = m.get(k) ?? continue` then `match
                        // held`, and `ref` there is rustc's error.
                        through_the_reference
                            || match value.as_ref() {
                                Expr::Variable(name, _) => {
                                    on.is_a_view()
                                        || self.s.got_past_a_jump.contains(self.parsed.text(*name))
                                }
                                _ => false,
                            },
                    );
                    let leaves = self.s.exits(self.world, &arm.body);
                    self.s.a_branch_that_leaves(from, leaves, span);
                    if let Some(subject) = &mut_subject {
                        self.parts_taken_out_of_a_mut_parameter(
                            subject,
                            &arm.body,
                            &typed,
                            &parts,
                            from.1 as usize,
                        );
                    }
                    self.s.branch.pop();
                    self.s.scope.pop();
                    // **An arm that jumps is not one of the types that have to
                    // agree** ([ADR-276](../../docs/specification/adr/adr-276.md)
                    // D1): its type is *never*, which fits every expected type
                    // without widening anything, so an arm that throws sits
                    // beside an arm that hands back a `&str` and the `match` is
                    // a `&str`. The language below reads the jumping arm as the
                    // `!` it is and agrees by construction.
                    if self.s.leaves(self.world, &arm.body) {
                        continue;
                    }
                    answered.push((Some(&arm.body), ty.clone()));
                    self.s
                        .arm_types
                        .insert(value_node(peeled(&arm.body)), ty.clone());
                    match &result {
                        None => result = Some(ty),
                        Some(seen) if *seen == ty => {}
                        Some(_) => agree = false,
                    }
                }
                self.s.stamped_condition = outer_condition;
                self.a_match_over_several_error_types(value, arms, span);
                // **`match error { … }` is a `match` over the type that
                // arrived**, where exactly one does. The binding itself is
                // untyped (see [`Checker::caught_one`]); this hands the one
                // question that can only gain by knowing.
                let on = match (&on, &**value) {
                    (Ty::Unknown, Expr::Variable(name, _))
                        if self.parsed.text(*name) == "error" && !self.s.flow.caught_several =>
                    {
                        self.s.flow.caught_one.clone().unwrap_or(Ty::Unknown)
                    }
                    _ => on,
                };
                self.a_match_that_misses_a_case(&on, arms, span);
                self.a_variant_matched_on_a_nullable(&on, value, arms, span);
                // Every arm of a `match` is a value of the same type, but what
                // that type is, is only known when every arm says the same -
                // or when the arms meet at text (ADR-282 D4).
                match result {
                    Some(ty) if agree => ty,
                    _ => {
                        let met = self
                            .arms_meet_at_text(&answered, span)
                            .or_else(|| self.arms_meet_at_null(&answered));
                        if met.is_none() {
                            self.arms_that_disagree(expr, &answered, span);
                        }
                        met.unwrap_or(Ty::Unknown)
                    }
                }
            }

            Expr::Call {
                func, args, config, ..
            } => self.nikaia(|state, world, host| {
                check_core_call::check_core_call(state, world, host, func, args, config, span)
            }),

            Expr::MethodCall {
                receiver,
                method,
                args,
                config,
                ..
            }
            | Expr::SafeMethod {
                receiver,
                method,
                args,
                config,
                ..
            } => {
                let safe = matches!(expr, Expr::SafeMethod { .. });
                self.nikaia(|state, world, host| {
                    check_core_method::check_core_method(
                        state, world, host, expr, safe, receiver, *method, args, config, span,
                    )
                })
            }

            Expr::Field { base, name, .. } | Expr::SafeField { base, name, .. } => {
                let safe = matches!(expr, Expr::SafeField { .. });
                self.nikaia(|state, world, host| {
                    check_core_places::check_core_field(
                        state, world, host, expr, safe, base, *name, span,
                    )
                })
            }

            Expr::StructLit { name, fields, .. } => self.nikaia(|state, world, host| {
                check_core_places::check_core_struct_literal(
                    state, world, host, expr, *name, fields, span,
                )
            }),

            // **`value with { field: … }`**
            // ([ADR-118](../../docs/specification/adr/adr-118.md) D1): a copy of
            // a value with named fields changed, of the same type. Every rule
            // about *naming* a field is the literal's — the braces are the
            // literal's — so what is new here is the **operand**: it has to be
            // a struct this compiler can name, because what it lowers to is
            // Rust's `Point { x: 1, ..p }` and the type is written there.
            Expr::With { base, fields, .. } => self.nikaia(|state, world, host| {
                check_core_places::check_core_with(state, world, host, expr, base, fields, span)
            }),

            // A lambda's arguments are the ones it names (ADR-277). There is
            // nothing to read off the body any more, so a `fn { … }` pushes an
            // empty frame - and a body reaching for `a` is then a body naming
            // something nothing declares, which `NK1117` refuses.
            Expr::Closure {
                params,
                mutable,
                body,
                ..
            } => {
                // **Its parameters are what the place it stands in says**
                // (#497), as a callee's signature says for an argument.
                let declared = self.s.flow.lambda_expected.take();
                let given = match &declared {
                    Some(Ty::Fn { params: given, .. }) if given.len() == params.len() => {
                        given.clone()
                    }
                    _ => Vec::new(),
                };
                let frame: Vec<Local> = params
                    .iter()
                    .enumerate()
                    .map(|(at, p)| {
                        let ty = given.get(at).cloned().unwrap_or(Ty::Unknown);
                        let local = self.lambda_parameter(*p, mutable, ty, span);
                        if local.ty.is_unknown() {
                            self.s.flow.unknown_bound.insert(local.id as usize as i64);
                        }
                        local
                    })
                    .collect();
                for local in &frame {
                    self.s
                        .nameable(&local.name.clone(), span, "a lambda's argument");
                }
                let depth = self.s.scope.len() as i64;
                self.s.repeats.push(Repeats::a_lambda(depth));
                self.s.scope.push(frame);
                // Part I 3.3: a lambda is a closure below, and a jump does not
                // leave one.
                let saved = self.enter_boundary("lambda");
                self.block(body);
                self.leave_boundary(saved);
                self.s.scope.pop();
                self.s.repeats.pop();
                // And it is of the type the place declares.
                match declared {
                    Some(fn_type @ Ty::Fn { .. }) if !given.is_empty() || params.is_empty() => {
                        fn_type
                    }
                    _ => Ty::Unknown,
                }
            }

            whole @ Expr::Unary { op, expr, .. } => {
                if matches!(op, UnaryOp::Neg) {
                    self.s.flow.reaching = self.s.flow.reached.take();
                }
                // `&s` looks at a sequence without taking it (ADR-293 D19). Only
                // a name directly under the `&`: `&f(s)` still hands `s` over.
                let looks = matches!(op, UnaryOp::Ref) && matches!(&**expr, Expr::Variable(_, _));
                let outer = std::mem::replace(&mut self.s.reading_only, looks);
                let inner = self.expr(expr, span);
                self.s.reading_only = outer;
                match op {
                    UnaryOp::Neg => {
                        if let Ty::Named {
                            name,
                            args,
                            view: false,
                        } = value_of_a_copy(inner.unseen())
                            && args.is_empty()
                            && (name == "i32" || name == "i64")
                            && !matches!(**expr, Expr::LitInt { .. })
                        {
                            self.s.facts.negations.insert(value_node(whole), name);
                        }
                        inner
                    }
                    // `!` is a `bool`'s, and the language below spells a
                    // bitwise complement the same way. Nikaia has no bitwise
                    // operator today, so this claims `bool` where the operand
                    // agrees and claims nothing where it does not - rather than
                    // insisting on `bool` and being wrong the day one arrives.
                    UnaryOp::Not => {
                        let boolean = Ty::named("bool");
                        // **`!` on an integer flips its bits**
                        // ([ADR-285](../../docs/specification/adr/adr-285.md)
                        // D3), and the integer is what it makes.
                        let value = value_of_a_copy(inner.unseen());
                        if integer_named(&value).is_some() {
                            value
                        } else if inner.fits(&boolean) {
                            boolean
                        } else {
                            Ty::Unknown
                        }
                    }
                    UnaryOp::Ref => view_of(&inner),
                }
            }

            Expr::Binary {
                op,
                lhs,
                rhs,
                span: at,
                ..
            } => {
                let arithmetic = matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
                );
                let reach = self.s.flow.reached.take().filter(|_| arithmetic);
                self.s.flow.reaching = reach.clone();
                let left = self.expr(lhs, span);
                self.s.flow.reaching = reach.clone();
                let right = self.expr(rhs, span);
                // **A `'…'` beside a byte is a byte** (ADR-320, Part I 2.6):
                // `b[0] == 'a'` compares two `u8`s, as `x == 7` beside an
                // `i32` compares two `i32`s.
                let byte = Ty::named("u8");
                let left = match value_of_a_copy(right.unseen()) == byte
                    && self.text_literal(&byte, lhs, false).is_some()
                {
                    true => byte.clone(),
                    false => left,
                };
                let right = match value_of_a_copy(left.unseen()) == byte
                    && self.text_literal(&byte, rhs, false).is_some()
                {
                    true => byte.clone(),
                    false => right,
                };
                self.divisor_is_not_zero(*op, rhs, span);
                self.an_operation_that_overflows(*op, lhs, rhs, reach.as_deref(), span);
                // **The stamp sticks**
                // ([ADR-281](../../docs/specification/adr/adr-281.md) D23):
                // `stand + 100` is a `Seen[i64]` and `stand > 100` a
                // `Seen[bool]`. An operator cannot put a value back into the
                // lock it came from, so what it makes still carries where it
                // came from — which is what lets `set` refuse it and every
                // other sink take it.
                let stamped = left.is_seen() || right.is_seen();
                // **A `T?` has no operator but `==` and `!=`** (Part I 2.3).
                if !matches!(
                    op,
                    BinaryOp::Eq | BinaryOp::Ne | BinaryOp::And | BinaryOp::Or
                ) {
                    self.s.a_maybe_in_an_operation(*op, &left, &right, at);
                }
                // **A bit operator beside a comparison is parenthesised**
                // ([ADR-285](../../docs/specification/adr/adr-285.md) D10).
                if op.is_comparison() {
                    self.a_bit_operation_beside_a_comparison(*op, lhs, rhs, at);
                    self.two_integer_types_compared([(lhs, &left), (rhs, &right)], at);
                }
                // **Two number types in one operation** (D1): `u64 + i64` is
                // `rustc`'s *mismatched types* about the generated file.
                // **Unless they have a common type** (ADR-285 D32): `u32 + i64`
                // is computed in `i64`, and the narrower side is widened.
                let common =
                    match !op.is_comparison() && !matches!(op, BinaryOp::And | BinaryOp::Or) {
                        true => self.two_number_types(*op, [(lhs, &left), (rhs, &right)], at),
                        false => None,
                    };
                // **A literal beside an unsigned side is of that side's type**
                // (ADR-285 D3), and the emitter is told so.
                if !matches!(op, BinaryOp::And | BinaryOp::Or) {
                    for (side, other, count) in [(lhs, &right, false), (rhs, &left, true)] {
                        let other = value_of_a_copy(other.unseen());
                        if count && matches!(op, BinaryOp::Shl | BinaryOp::Shr) {
                            continue;
                        }
                        if let Some(unsigned) =
                            integer_named(&other).filter(|n| matches!(n.as_str(), "u32" | "u64"))
                        {
                            self.literals_are(side, &unsigned, span);
                        }
                    }
                }
                // **An operation is a use of the numbers on both sides**
                // ([ADR-285](../../docs/specification/adr/adr-285.md) D23, D24):
                // a side with a type asks the other for it, and two open sides
                // are one number's question from here on. A shift's count is
                // any integer and is neither.
                if !matches!(
                    op,
                    BinaryOp::And | BinaryOp::Or | BinaryOp::Shl | BinaryOp::Shr
                ) {
                    self.number_asked(lhs, &right, at, true);
                    self.number_asked(rhs, &left, at, true);
                    let mut both = Vec::new();
                    self.open_numbers_in(lhs, &mut both);
                    self.open_numbers_in(rhs, &mut both);
                    self.open_numbers_joined(&both);
                }
                let outcome = match op {
                    // **`&`, `|`, `^` and the shifts take integers** (D3): a
                    // `bool` has `&&` and `||`, which say it once.
                    BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr => {
                        self.s
                            .a_bit_operator_on_something_else(*op, &left, &right, at);
                        let left = value_of_a_copy(left.unseen());
                        let right = value_of_a_copy(right.unseen());
                        match (op, left.is_unknown()) {
                            // A shift is its left side's, whatever counts it.
                            (BinaryOp::Shl | BinaryOp::Shr, _) => left,
                            (_, true) => right,
                            _ => left,
                        }
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        let (on_left, on_right) =
                            (self.span_at(lhs, *span), self.span_at(rhs, *span));
                        self.expect_bool(&left, &on_left, "`&&` and `||` join two `bool`s.");
                        self.expect_bool(&right, &on_right, "`&&` and `||` join two `bool`s.");
                        Ty::named("bool")
                    }
                    BinaryOp::Eq | BinaryOp::Ne => {
                        // **A comparison only reads its sides** (ADR-279 D5):
                        // a `??` of text on either side lends its left side.
                        // Text alone, because the language below compares a
                        // view of text with text and with a view, and a view of
                        // anything else with nothing but a view.
                        for side in [lhs, rhs] {
                            self.a_pending_coalesce_of_text_is_lent(side);
                        }
                        // **`x == null` asks whether there is one**, and the
                        // emitter writes it `x.is_none()` (0.0.250): nothing
                        // of what `x` holds is compared, so a type that does
                        // not compare is no reason to refuse it.
                        let asks_for_null = matches!(lhs.as_ref(), Expr::LitNull(_))
                            || matches!(rhs.as_ref(), Expr::LitNull(_));
                        if !asks_for_null {
                            self.an_operator_needs_a_bound(*op, &left, &right, at);
                            self.a_type_that_does_not_compare(&left, &right, at);
                        }
                        if !asks_for_null {
                            self.s.a_maybe_compared_with_a_value(&left, &right, span);
                        }
                        self.a_view_compared_with_a_value(lhs, &left, rhs, &right, at);
                        if names_a_path(&left, &right) {
                            self.s.facts.path_comparisons.insert(at.at() as i64);
                        }
                        Ty::named("bool")
                    }
                    BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                        self.an_operator_needs_a_bound(*op, &left, &right, at);
                        self.s.a_maybe_compared_with_a_value(&left, &right, span);
                        self.a_view_compared_with_a_value(lhs, &left, rhs, &right, at);
                        Ty::named("bool")
                    }
                    // **A `+` where either side is text is a concatenation**
                    // ([ADR-081](../../docs/specification/adr/adr-081.md) D2),
                    // and it comes to a `String` whichever side was owned. This
                    // used to be the `_ => Ty::Unknown` below, with a comment
                    // saying that guessing which side names the result was the
                    // one guess this checker does not make. It was not a guess
                    // that was missing but a **decision**: a concatenation makes
                    // a new value, so `String` is the only thing it can be, and
                    // saying `&str` - which is what two borrowed sides used to
                    // come to - made `-> String` a false refusal (`NK1104`).
                    //
                    // Recorded by the operator's own span, because `a + b + c`
                    // is two of them and the statement they stand in is one.
                    BinaryOp::Add if is_text(&left) || is_text(&right) => {
                        self.s.facts.concatenations.insert(at.at() as i64);
                        Ty::named("String")
                    }
                    // **`NK1191`: arithmetic on something that has none**
                    // (0.0.234). `[1] + [2]` lowered as it was written and the
                    // language below said *cannot add `Vec<i64>` to `Vec<i64>`*
                    // about a file nobody wrote. Part I gives a list no
                    // operator, and ADR-253 keeps it that way for `+` on two
                    // lists; a declared `struct` or `enum` has none either.
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
                        if self.has_no_arithmetic(&left) || self.has_no_arithmetic(&right) =>
                    {
                        self.arithmetic_on_something_that_has_none(
                            *op,
                            [(lhs, &left), (rhs, &right)],
                            false,
                            span,
                        );
                        Ty::Unknown
                    }
                    // Arithmetic on two of the same thing is that thing, and
                    // a bare number is neither - so one known side decides. Two
                    // known sides that disagree still decide nothing, now that
                    // the one case anybody met is taken above.
                    //
                    // **And a view of a number is the number here**
                    // ([ADR-233](../../docs/specification/adr/adr-233.md) D4):
                    // `0 - y` over a `ref i64` makes an `i64`, which is what
                    // the language below's operators make of a `&i64` too.
                    _ => {
                        // **One whole-number type on both sides**, or one side
                        // a literal: what an unchecked operation may name
                        // (ADR-306 D10). A view is not the number here - the
                        // operation would be handed a reference.
                        if matches!(op, BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul) {
                            let whole = |side: &Expr, ty: &Ty| match (side, ty.unseen()) {
                                (Expr::LitInt { .. }, _) => Some(None),
                                (
                                    _,
                                    Ty::Named {
                                        name,
                                        args,
                                        view: false,
                                    },
                                ) if args.is_empty() && is_a_whole_number(&name) => {
                                    Some(Some(name))
                                }
                                _ => None,
                            };
                            if let (Some(a), Some(b)) = (whole(lhs, &left), whole(rhs, &right)) {
                                let named = match (a, b) {
                                    (Some(a), Some(b)) if a == b => Some(a),
                                    (Some(a), None) | (None, Some(a)) => Some(a),
                                    _ => None,
                                };
                                if let Some(name) = named {
                                    self.s.facts.arithmetic.insert(at.at() as i64, name);
                                }
                            }
                        }
                        match (
                            left.is_unknown(),
                            right.is_unknown(),
                            value_of_a_copy(left),
                            value_of_a_copy(right),
                        ) {
                            (true, _, _, right) => right,
                            (_, true, left, _) => left,
                            (_, _, left, right) if left == right => left,
                            _ => Ty::Unknown,
                        }
                    }
                };
                let outcome = match common {
                    Some(common) => Ty::named(common),
                    None => outcome,
                };
                match stamped {
                    true => Ty::seen(outcome.unseen()),
                    false => outcome,
                }
            }

            Expr::Cast { expr, ty, .. } => self.nikaia(|state, world, host| {
                check_core_places::check_core_cast(state, world, host, expr, ty, span)
            }),

            Expr::Tuple(parts, _) => self.nikaia(|state, world, host| {
                check_core_call::check_core_tuple(state, world, host, parts, span)
            }),

            // **`[1, 2, 3]` is a `Vec[T]`**
            // ([ADR-135](../../docs/specification/adr/adr-135.md) D1), and `T`
            // is what the elements agree on.
            Expr::ListLit { items, .. } => self.nikaia(|state, world, host| {
                check_core_args::check_core_list_literal(state, world, host, items, span)
            }),

            // A `?` unwraps a failure, a `??` unwraps an absence, an index
            // reaches into a container and a range is an iterator: four things
            // Stage 0 has no signature for.
            Expr::Try(inner, _) => {
                self.expr(inner, span);
                Ty::Unknown
            }
            // Kap 7.1: `throw` leaves the function, so it has no value of its
            // own - the same shape a `return` has. What it throws is walked,
            // because a mistyped constructor inside it is still a mistake.
            Expr::Throw(inner, _) => {
                let thrown = self.expr(inner, span);
                self.s
                    .a_thrown_value_that_is_not_an_error(inner, &thrown, span);
                self.s.a_way_out("throw", "", span);
                Ty::Unknown
            }

            // **`return`, `break` and `continue` where an expression stands**
            // ([ADR-276](../../docs/specification/adr/adr-276.md) D20). Nothing
            // about what they do changes (D2), so each asks exactly what its
            // statement form asks — the answer to *what is a `return` worth* is
            // the same wherever it is written.
            Expr::Return(value, _) => {
                self.returns((**value).as_ref(), span, true);
                self.s.a_way_out("return", "", span);
                Ty::Unknown
            }
            Expr::Break(_) | Expr::Continue(_) => {
                if self.s.flow.loops == 0 {
                    let word = match expr {
                        Expr::Break(_) => "break",
                        _ => "continue",
                    };
                    self.a_jump_with_nowhere_to_go(word, span);
                }
                Ty::Unknown
            }
            Expr::Coalesce {
                value, fallback, ..
            } => {
                let outer = std::mem::replace(&mut self.s.flow.read_a_map, false);
                let left = self.expr(value, span);
                let read_now = std::mem::replace(&mut self.s.flow.read_a_map, outer);
                // **The left side alone** (D13): the fallback is still checked,
                // and what it would take is not taken, because it never runs.
                if self.nothing_to_fall_back_from(value, &left, expr, span) {
                    let from = self.s.taken_so_far();
                    self.expr(fallback, span);
                    self.s.walked.truncate(from.0 as usize);
                    self.s.handed.truncate(from.1 as usize);
                    return left;
                }
                // **A map read, or a name a `let` bound to one** (#297): the
                // left side is a view of what the map keeps either way.
                let from_a_map = match &**value {
                    Expr::Index { .. } | Expr::MethodCall { .. } => read_now,
                    Expr::Variable(name, _) => {
                        self.s
                            .binding(self.parsed.text(*name))
                            .is_some_and(|local| {
                                self.s
                                    .flow
                                    .map_views
                                    .contains(&((local.id as usize) as i64))
                            })
                    }
                    _ => false,
                };
                // **A fallback that jumps takes its takings with it**
                // (ADR-293 D30), as a branch that leaves does: `let c = f(k) ??
                // return Refused(k)` hands `k` over only on the path that
                // leaves, so a use of `k` on the next line is no use after it -
                // it was `NK2105` (found moving `contracts::order`'s walk into
                // Nikaia, #125). A block that ends in a jump is one (#457).
                let from = self.s.taken_so_far();
                let other = self.expr(fallback, span);
                let leaves = self.s.leaves(self.world, fallback);
                self.s.a_branch_that_leaves(from, leaves, span);
                // **A name on the left of `??` is taken where the answer is
                // kept and lent where it is only read** (ADR-279 D5), which is
                // the rule an argument follows (ADR-094 D1). Which one is known
                // once the position is: a call that only reads the answer lends
                // it (`the_compiler_writes_the_reference`), and every other `??`
                // is taken when its statement ends (`block`). Taken, a use of
                // the name after the line is `NK2105` (0.0.286). A view and a
                // copy are neither, which `hands_over` asks.
                // A field is a place as a name is: `self.parameter ?? ""` over
                // a `ref self` lends the field where the answer is read, and
                // where it is kept it is a part of a loan handed over, which is
                // refused in the language's words - unrecorded, it reached
                // `rustc` as *cannot move out of `self.parameter`* (found moving
                // the ledger's records into Nikaia, ADR-294).
                // **A loan on the left and a jump on the right is a view**
                // ([ADR-279](../../docs/specification/adr/adr-279.md) D10):
                // `let s = c.signature ?? return false` over a lent `c` cannot
                // take the field, and a jump has no value, so the answer is a
                // view of the left side wherever it stands - a `let` binds a
                // view, as `let found = m[k]` does.
                let lent_and_jumps = matches!(&**value, Expr::Variable(_, _) | Expr::Field { .. })
                    && self.s.jumps(self.world, fallback)
                    && matches!(&left, Ty::Nullable(inner) if self.s.takes_away(self.world, &inner.unseen()))
                    && self
                        .s
                        .rooted_at(self.world, value)
                        // **So is a local `get` handed back past a jump** (D35).
                        .is_some_and(|root| {
                            self.s.lent_here(&root) || self.s.got_past_a_jump.contains(&root)
                        });
                if lent_and_jumps {
                    let text = matches!(&left, Ty::Nullable(inner)
                        if matches!(inner.as_ref(), Ty::Named { name, args, view: false }
                            if args.is_empty() && name == ty::TEXT));
                    self.s.facts.lent_coalesces.insert(
                        value_node(expr),
                        if text {
                            ".as_deref()".to_string()
                        } else {
                            ".as_ref()".to_string()
                        },
                    );
                    if let Ty::Nullable(inner) = &left {
                        return view_of(inner);
                    }
                }
                if matches!(&**value, Expr::Variable(_, _) | Expr::Field { .. }) {
                    match self
                        .s
                        .a_coalesce_that_may_lend(self.world, &left, fallback, &other)
                    {
                        Some(opened) => self.s.flow.pending_coalesces.push(PendingCoalesce {
                            key: value_node(expr),
                            left: value_node(value),
                            ty: left.clone(),
                            span: *span,
                            opened: opened.to_string(),
                        }),
                        None => self.hands_over(value, &left, "given to `??`", span),
                    }
                }
                if let (Ty::Nullable(inner), Expr::LitStr { .. }) = (&left, &**fallback) {
                    if **inner == Ty::view("str") {
                        self.s.facts.view_fallbacks.insert(value_node(fallback));
                    }
                    // **A map's text, read, is a view of it** (ADR-293 D28): the
                    // map hands out its `String` and keeps it, so the literal
                    // after `??` is the other view the answer can be, and the
                    // whole is a view of text - `m[k] ?? "-"`. It used to be
                    // built into a `String` below and handed to a place that
                    // wanted the map's reference: `rustc`'s words, about a file
                    // nobody wrote.
                    if **inner == Ty::named("String") && from_a_map {
                        self.s.facts.view_fallbacks.insert(value_node(fallback));
                        return Ty::view("str");
                    }
                    // **And a list's** (#342, ADR-279 D12): `ys[1] ?? "none"`
                    // over a `Vec[String?]` is a view - the read is written as
                    // the element lent, not moved out of the list.
                    if **inner == Ty::named("String")
                        && !from_a_map
                        && matches!(&**value, Expr::Index { .. })
                    {
                        self.s.facts.view_fallbacks.insert(value_node(fallback));
                        self.s.facts.lent_list_reads.insert(value_node(expr));
                        return Ty::view("str");
                    }
                }
                // **And a name that is a view of text beside it** (#297):
                // `imports.get(call) ?? call` for a `call: ref String` is a
                // view whichever side answers, written as it is.
                if let Ty::Nullable(inner) = &left
                    && **inner == Ty::named("String")
                    && from_a_map
                    && other.is_a_view()
                    && matches!(&**fallback, Expr::Variable(_, _) | Expr::Field { .. })
                    && matches!(&other, Ty::Named { name, .. }
                        if matches!(crate::contracts::ty::base(name), "String" | "str"))
                {
                    self.s.facts.view_coalesces.insert(value_node(expr));
                    return Ty::view("str");
                }
                // **And a name of the type the map holds** (ADR-279 D7):
                // `m[k] ?? spare` reads the map's value, a view, and a view
                // of `spare` exists for as long as the answer does - it is a
                // name in scope at the `??` - so the name is lent, as
                // `r ?? spare` lends it over a `ref T?`, rather than refused
                // as a value of its own. Text has its own answers above.
                if let Ty::Nullable(inner) = &left
                    && from_a_map
                    && matches!(&**fallback, Expr::Variable(_, _))
                    && !other.is_a_view()
                    && let Ty::Named { name, args, .. } = inner.as_ref()
                    && !matches!(name.as_str(), "str" | "String")
                    && other
                        == (Ty::Named {
                            name: name.clone(),
                            args: args.clone(),
                            view: false,
                        })
                    && crate::contracts::keeps::moves(&other)
                {
                    self.s.facts.lent_map_fallbacks.insert(value_node(expr));
                    return view_of(&other);
                }
                // **A value written out beside a view of a type of the program's
                // own** (#574): `x ?? BinaryOp::Add` for an `x: ref BinaryOp?`
                // is a view whichever side answers, so the fallback is lent
                // too - a constant the language below promotes to a `&'static`.
                // It was the value, and `rustc` expected the reference.
                if let Ty::Nullable(inner) = &left
                    && !from_a_map
                    && inner.is_a_view()
                    && !other.is_a_view()
                    && let Ty::Named { name, .. } = inner.as_ref()
                    && !is_number(name)
                    && !matches!(
                        crate::contracts::ty::base(name),
                        "bool" | "char" | "scalar" | "String" | "str"
                    )
                    && self.a_value_written_out(fallback)
                {
                    self.s.facts.lent_map_fallbacks.insert(value_node(expr));
                    return *inner.clone();
                }
                if let Ty::Nullable(inner) = &left {
                    match from_a_map {
                        // **What a map holds has its own sentence**, whether
                        // the read is written here or a name holds it (#297).
                        true => {
                            // A view of text is `str` to this checker and
                            // `String` to the language (ADR-184 D2).
                            let held = match inner.as_ref() {
                                Ty::Named { name, .. } if name == "str" => {
                                    Ty::named(crate::contracts::ty::TEXT)
                                }
                                Ty::Named { name, args, .. } => Ty::Named {
                                    name: name.clone(),
                                    args: args.clone(),
                                    view: false,
                                },
                                other => other.clone(),
                            };
                            self.a_fallback_beside_what_a_map_holds(value, &held, &other, span);
                            // Text has its own answer, a view beside a view;
                            // a name holding the read is one already.
                            if held == Ty::named(crate::contracts::ty::TEXT) {
                                self.a_fallback_that_owns_what_the_left_side_views(
                                    inner, &other, fallback, span,
                                );
                            }
                        }
                        false => self.a_fallback_that_owns_what_the_left_side_views(
                            inner, &other, fallback, span,
                        ),
                    }
                }
                // **`a ?? b` where `b` may be `null` too is a `T?`** (Part I
                // 3.5): the first that has a value, and there may be none. It
                // was read as a `T` whatever the fallback was, and the fallback
                // was converted into what the option holds - which an option
                // is not, so `rustc` refused the file (found moving `dsl` into
                // Nikaia, #125).
                if let (Ty::Nullable(_), Ty::Nullable(_)) = (&left, &other) {
                    self.s.facts.optional_fallbacks.insert(value_node(expr));
                    return other;
                }
                // **`a ?? b` on a `T?` is a `T`** (Part I 3.5): that is what
                // ending the chain means, and claiming nothing about it cost
                // everything downstream — the day a map read became a `T?`
                // ([ADR-293](../../docs/specification/adr/adr-293.md) D1),
                // `let s = m[k] ?? panic(…)` made `s` unknown and `s.mean()`
                // one more method call nobody could answer.
                //
                // **Only that shape.** A `??` over something this checker could
                // not type claims nothing, which is
                // [Part III C.4](../../../docs/specification/30-nikaia-tooling.md):
                // the answer is more information than before and no new claim.
                //
                // **A view of a number beside a value is the number**
                // (ADR-233 D4): `let found = m[k]` binds a view of what the
                // map keeps, and `found ?? 0` is `index::or`'s copy of it, as
                // `m[k] ?? 0` is - it was typed a `ref i64`, and `return found
                // ?? 0` from an `-> i64` was `NK1104` (found moving `sharing`'s
                // classes into Nikaia, #125). A view beside it is the other
                // impl, whose answer stays the reference.
                match left {
                    // **What the left side holds is not known, and the
                    // fallback says it** (#497): `rx.recv() ?? "none"` over a
                    // channel nothing typed is text, because both sides are
                    // one value's.
                    Ty::Nullable(inner) if inner.is_unknown() && !other.is_unknown() => other,
                    Ty::Nullable(inner) if !other.is_a_view() => value_of_a_copy(*inner),
                    Ty::Nullable(inner) => *inner,
                    _ => Ty::Unknown,
                }
            }
            // Indexing a container yields what the container holds - but only
            // where the container's type says so.
            //
            // This claimed nothing at all until ADR-288, and the cost was not
            // the missing type but everything downstream of it: in
            // `n-body.nika`, `let b = &self.bodies[i]` made `b` unknown, so
            // `b.x` was unknown, so `dx * dx + dy * dy` was unknown, so
            // `.sqrt()` could not be resolved and `energy` could not be shown
            // to be pure computation. One `Unknown` at the bottom of an
            // expression erases everything built on it.
            //
            // A shape it does not recognise still claims nothing: `s[i]` over
            // text is a slice in some languages and a byte in others, and
            // Nikaia has not said. `?` is the absence of a claim (ADR-024 D1).
            Expr::Index { base, index, .. } => self.nikaia(|state, world, host| {
                check_core_places::check_core_index(state, world, host, expr, base, index, span)
            }),
            // **A range is a sequence that replays**
            // ([ADR-293](../../docs/specification/adr/adr-293.md) D18): its
            // elements are produced as they are asked for, it can be walked
            // from either end, its length is known, and walking it walks a copy
            // - it is two numbers, and a value. Before, it had no type at all,
            // so `let r = 0..<3` and a `for` over `r` went to `rustc` and came
            // back as *no method named `iter`*, about a file nobody wrote.
            //
            // The item is whichever end says one; a literal says none
            // (Part I 2.4), so `0..<3` is a range of `?` and `0..<n` one of
            // what `n` is.
            Expr::Range { start, end, .. } => self.nikaia(|state, _world, host| {
                check_core_places::check_core_range(state, host, start, end, span)
            }),

            Expr::TryCatch { expr, handler, .. } => {
                // Kap 7.1: the handler is what handles the failure, so the
                // guarded expression is where a fallible call needs no
                // `throws` on the function around it (`NK2605`). The handler
                // itself is ordinary code again - a failure raised inside one
                // leaves the function like any other.
                let outer = std::mem::replace(&mut self.s.flow.caught, true);
                // **And the lambda's own record of it** (ADR-277 D7): a
                // `catch` in a lambda's body keeps a failure from leaving the
                // lambda, which `self.s.flow.caught` cannot say because it also holds
                // for a `catch` around the call the lambda is handed to. Put
                // back at the same moment, so the handler's own calls count.
                let lambda_outer = self
                    .s
                    .handed_over
                    .as_mut()
                    .map(|handed| std::mem::replace(&mut handed.caught, true));
                // **A guard of its own, and the enclosing one set aside**
                // ([ADR-308](../../../docs/specification/adr/adr-308.md)). A
                // `catch` inside another one's guarded expression handles its
                // own failures, so those are not the outer one's to count -
                // `(a.parse() catch { 1 }) catch { 2 }` has nothing left for the
                // second. The outer is told *no answer* rather than *nothing
                // fails*, because what a handler's own failure does is
                // [ADR-292](../../../docs/specification/adr/adr-292.md)'s
                // question and this refusal does not need it answered.
                let enclosing = self.s.flow.guarded.replace(Guarded::nothing());
                let answers = self.expr(expr, span);
                let guarded = std::mem::replace(&mut self.s.flow.guarded, enclosing);
                if self.s.flow.guarded.is_some() {
                    self.guard_has_no_answer();
                }
                self.s.flow.caught = outer;
                if let (Some(handed), Some(was)) = (&mut self.s.handed_over, lambda_outer) {
                    handed.caught = was;
                }
                let guarded = guarded.unwrap_or_else(Guarded::nothing);
                self.a_fatal_error_passes(&guarded, handler, span);
                // **`error` is what the guarded calls throw** (Part I 7.1,
                // #501): `fs::read_to_string(…) catch { … {error} … }` binds an
                // `io::IoError`, as the ledger records the call. One type, and
                // every call looked up; several are a sum the handler tells
                // apart with `match`, and claim nothing here.
                let caught = match (
                    guarded.unanswered,
                    guarded.thrown.iter().collect::<Vec<_>>().as_slice(),
                ) {
                    (false, [one]) if *one != "?" => Ty::named(one.as_str()),
                    // **Several: an `Error`, one of them**
                    // ([ADR-280](../../docs/specification/adr/adr-280.md) D30):
                    // known by its trait, with its members kept for what is
                    // said about it.
                    (false, members @ [_, _, ..]) if members.iter().all(|m| *m != "?") => {
                        Ty::Named {
                            name: "Error".to_string(),
                            args: members.iter().map(|m| Ty::named(m.as_str())).collect(),
                            view: false,
                        }
                    }
                    _ => Ty::Unknown,
                };
                self.s.nothing_here_can_fail(&guarded, span);
                let error = self.local_free("error".to_string(), caught);
                self.s.scope.push(vec![error]);
                let arriving = self.several_arrive(expr);
                let one = self.the_one_error(expr);
                let several = std::mem::replace(&mut self.s.flow.caught_several, arriving);
                let single = std::mem::replace(&mut self.s.flow.caught_one, one);
                let from = self.s.taken_so_far();
                let handled = self.block(handler);
                // **A handler's value stands where the guarded one would**, so
                // a text literal it ends in is built into text of its own where
                // the guarded value is (ADR-282 D3, one more of ADR-282 D3's
                // positions): `read() catch { "" }`.
                let literal =
                    tail_of(handler).and_then(|tail| self.text_literal(&answers, tail, true));
                let leaves = self.s.block_exits(self.world, handler);
                self.s.a_branch_that_leaves(from, leaves, span);
                self.s.flow.caught_several = several;
                self.s.flow.caught_one = single;
                self.s.scope.pop();
                // **And so the `catch` is the guarded value's type**
                // (0.0.230, issue #154 before it): where the handler leaves,
                // or hands back that type, or something this checker cannot
                // name. It used to be `?` always, so `let report = Log::file(d)
                // catch { return }` typed nothing that followed - a field read
                // on a lookup in `report.paths` reached `rustc` instead of
                // `NK1125`. A handler whose value is known to be something
                // else keeps the old answer, which claims nothing.
                let handled = literal.unwrap_or(handled);
                match self.s.block_leaves(self.world, handler)
                    || handled.is_unknown()
                    || handled.fits(&answers)
                {
                    true => answers,
                    false => Ty::Unknown,
                }
            }

            Expr::Spawn { body, .. } => {
                // **Before** the body is walked, so that the task's own `let`s
                // are not yet in scope: a name bound inside the task is the
                // task's own and crosses nothing.
                self.crosses_into_a_task(body, span);

                // **A `spawn` hands back a `TaskHandle` of what the task's body
                // comes to** (Part I 8.2, ADR-055 D5), and that is why the body
                // is walked here rather than through the generic closure arm:
                // what a lambda hands back is not written down anywhere
                // (ADR-288 D13), but a task's body is a *block*, and a block's
                // value is a thing this checker already knows. So `spawn fn {
                // work(21) }` is a `TaskHandle[i64]` and `handle.join()` is an
                // `i64`, with nothing inferred that a signature did not say.
                // The handles a task duplicates are keyed by its lambda's node
                // (ADR-340 D5), the one the emitter is handed.
                let task = value_node(body);
                let Expr::Closure { params, body, .. } = body.as_ref() else {
                    self.expr(body, span);
                    return Ty::Unknown;
                };
                // A task is handed nothing, so a parameter has nothing to be
                // bound from. Refused rather than dropped: `spawn fn (x) { … }`
                // reads as though `x` arrives from somewhere.
                self.s.a_task_takes_no_arguments(self.world, params, span);
                // **What the task takes with it** (`NK2101`), read off the body
                // before it is walked: a name the body binds for itself is the
                // task's own, and the frame pushed below is what keeps the two
                // apart.
                self.a_task_takes_these(
                    &Expr::Closure {
                        params: params.clone(),
                        // A task is handed nothing, so no parameter of one is
                        // ever `mut` - `a_task_takes_no_arguments` above has
                        // already refused the list that is not empty.
                        mutable: Vec::new(),
                        body: body.clone(),
                        id: crate::ast::fresh_id(),
                    },
                    task,
                    span,
                );
                self.s.scope.push(Vec::new());
                self.s.task_bindings.push(Vec::new());
                // **A task started with `spawn` runs later and elsewhere**
                // ([ADR-281](../../docs/specification/adr/adr-281.md) D29), so
                // taking a lock inside one is the ordinary case and not this
                // door's reach. `contracts::locks` makes the same split when it
                // builds the column; this is it at the refusal.
                //
                // A **scope**'s tasks are the other way round and are not this:
                // a scope waits for them, so they run during the call.
                let outer_inside = std::mem::replace(&mut self.s.inside_a_door, false);
                // A task's body is an `async` block below (ADR-055 §6), which
                // is a function too: `break` may not leave it either.
                let saved = self.enter_boundary("task");
                let value = self.block(body);
                self.leave_boundary(saved);
                self.s.scope.pop();
                self.s.inside_a_door = outer_inside;
                let bound = self.s.task_bindings.pop().unwrap_or_default();
                // **After** the walk, because the question needs both halves of
                // what the walk found: the types of what the body bound, and
                // which of its method calls pause (`pausing_methods`).
                self.a_task_holds_these_across_a_pause(body, &bound, span);
                Ty::Named {
                    name: "TaskHandle".to_string(),
                    args: vec![value],
                    view: false,
                }
            }

            // A template's holes are Nikaia too (ADR-017), and what the
            // template *produces* is still the emitter's business.
            Expr::Dsl {
                target,
                package: None,
                ..
            } if self.a_package_where_a_grammar_stands(*target, span) => {
                self.guard_has_no_answer();
                Ty::Unknown
            }
            // **A block over a grammar this program declares is run while the
            // program is built** (Part II 10.5, ADR-299 D20, #518): it is the
            // `comptime` of the grammar's entry on the block's text, computed
            // by the same evaluator, and its type is the entry's result.
            Expr::Dsl {
                target,
                package: None,
                context: None,
                content,
                ..
            } if self
                .s
                .grammars
                .get(self.parsed.text(*target))
                .is_some_and(|entries| entries.len() == 1)
                && !crate::dsl::is_deferred(self.parsed.text(*target), content) =>
            {
                self.a_block_over_a_grammar(expr, *target, content, span)
            }
            Expr::Dsl { .. } => {
                // Which is also why `NK1134` says nothing about one: what this
                // becomes is decided after the checker has run, so whether it
                // can fail is not this walk's to answer.
                self.guard_has_no_answer();
                self.holes(expr, span);
                // **`dsl html` builds a `String`** (#505): the compiler is that
                // template's grammar, and what it writes is text of its own. A
                // block for a grammar that is not this compiler's is the
                // grammar's to type (its entry rule, or a statement's row).
                match expr {
                    Expr::Dsl {
                        target,
                        package: None,
                        context: None,
                        content,
                        ..
                    } if self.parsed.text(*target) == "html"
                        && !crate::dsl::is_deferred("html", content) =>
                    {
                        Ty::named(ty::TEXT)
                    }
                    // **A statement with `:name` holes is its own text**
                    // (ADR-296 D5, #497): it reaches its driver as written,
                    // and the values arrive beside it.
                    Expr::Dsl {
                        target,
                        package: None,
                        context: None,
                        content,
                        ..
                    } if crate::dsl::is_deferred(self.parsed.text(*target), content) => {
                        Ty::view("str")
                    }
                    _ => Ty::Unknown,
                }
            }

            // A grammar and an `asm` block: what these produce is the business
            // of the emitter that compiles them.
            Expr::Asm { .. } => {
                self.guard_has_no_answer();
                Ty::Unknown
            }
        }
    }

    /// **`NK1228`: a `dsl` block that names a package where a grammar
    /// stands** ([ADR-299](../../docs/specification/adr/adr-299.md) D20):
    /// `dsl sqlite { … }` for `dsl sqlite::Sql { … }`. A package is what a
    /// `use` line brings in, by its alias or its last name; `html` is the
    /// compiler's own template and a grammar this program declares is named
    /// bare, so neither is one.
    fn a_package_where_a_grammar_stands(
        &mut self,
        target: winnow_grammar::Symbol,
        span: &Span,
    ) -> bool {
        self.s
            .a_package_where_a_grammar_stands(&self.world.here, target, span)
    }

    /// Every integer literal a constant expression is made of, recorded as
    /// being of an unsigned type ([`Checked::unsigned_literals`]).
    fn literals_are(&mut self, value: &Expr, ty: &str, _span: &Span) {
        self.s.literals_are(value, ty);
    }

    /// `NK1197` (`tools/check_exprs.nika`).
    fn a_bit_operation_beside_a_comparison(
        &mut self,
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        at: &Span,
    ) {
        self.s
            .a_bit_operation_beside_a_comparison(self.world, op, lhs, rhs, at);
    }

    /// A comparison of two integer types (`tools/check_numbers2.nika`).
    fn two_integer_types_compared(
        &mut self,
        [(lhs, left), (rhs, right)]: [(&Expr, &Ty); 2],
        at: &Span,
    ) {
        self.s.two_integer_types_compared(lhs, left, rhs, right, at);
    }

    /// Two sides of one operation of different number types
    /// (`tools/check_numbers2.nika`).
    fn two_number_types(
        &mut self,
        op: BinaryOp,
        [(lhs, left), (rhs, right)]: [(&Expr, &Ty); 2],
        at: &Span,
    ) -> Option<&'static str> {
        self.s
            .two_number_types(op, lhs, left, rhs, right, at)
            .map(|common| a_static_integer(&common))
    }

    /// Whether an expression is a variant of an enum this file declares, with
    /// or without its parts, or a struct written out: a value that stands for
    /// itself, and that the language below can lend as a constant.
    fn a_value_written_out(&self, written: &Expr) -> bool {
        self.s.a_value_written_out(self.world, written)
    }

    /// `NK1185` (`tools/check_fields.nika`).
    fn a_fallback_that_owns_what_the_left_side_views(
        &mut self,
        viewed: &Ty,
        fallback: &Ty,
        written: &Expr,
        span: &Span,
    ) {
        self.s.a_fallback_that_owns_what_the_left_side_views(
            self.world, viewed, fallback, written, span,
        );
    }

    /// `NK1185` (`tools/check_exprs.nika`).
    fn a_fallback_beside_what_a_map_holds(
        &mut self,
        read: &Expr,
        held: &Ty,
        fallback: &Ty,
        span: &Span,
    ) {
        self.s
            .a_fallback_beside_what_a_map_holds(self.world, read, held, fallback, span);
    }

    /// `NK1184` for an escape in a literal that nothing names
    /// (`tools/check_reports.nika`).
    fn an_escape_nothing_names(&mut self, text: &str, span: &Span) {
        if let Some(refused) = crate::build_time::an_escape_nothing_names(text) {
            self.s.an_escape_nothing_names(
                text,
                &refused.written,
                &refused.why,
                crate::build_time::ESCAPES,
                span,
            );
        }
    }

    /// **The one thing this checker warns about rather than refusing** - a
    /// plain string that was written when every string was a template
    /// (ADR-309 D15).
    ///
    /// Two shapes, because the change has two halves. `"hello {name}"` used to
    /// interpolate and is now text, and `"{{}}"` used to *be* `{}` and is now
    /// four characters. Both change what a program prints without changing
    /// whether it compiles, and Part III C.1 calls a silent change of meaning a
    /// bug in this compiler.
    ///
    /// **Why a warning and not an error.** `"{ margin: 0 }"` is correct CSS and
    /// `"\\d{3}"` a correct regular expression; refusing either would break the
    /// property the whole checker rests on - that it never rejects a program
    /// that is right. So the test is deliberately narrow: the braces have to
    /// hold something that **parses as an expression and resolves to something
    /// that is actually here** - a variable in scope, or a function a ledger
    /// knows. A name nobody declared is text, and text says nothing.
    fn unmarked_hole(&mut self, text: &str, span: &Span) {
        let host = self.host();
        self.s.unmarked_hole(self.world, &host, text, span);
    }

    /// Every expression a literal hides, walked where it stands.
    ///
    /// The result is discarded: what a hole evaluates to is the emitter's
    /// business - `Render` decides for a template and `Display` for a string -
    /// and what the checker is here for is everything *inside* it.
    fn holes(&mut self, literal: &Expr, span: &Span) {
        self.nikaia(|state, world, host| {
            check_core_leaves::check_core_holes(state, world, host, literal, span)
        })
    }

    /// **`NK1201`, a file's name in an `f"…"` used as text** (ADR-319 D4):
    /// a name holds the platform's bytes, which need not be text, so it has no
    /// text form where the `f"…"` is text. Refused at the hole, the same at
    /// every position; whether the name is text is the program's to say,
    /// with `to_text()` or `display()`.
    fn a_name_in_text(&mut self, want: &Ty, value: Option<&Expr>, span: &Span) -> bool {
        self.s.a_name_in_text(self.world, want, value, span)
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

    /// **A generic call whose result only an open number binds** (#497):
    /// `hand(7)` for `fn hand[T](x: T) -> T` hands back the `7`, whose type
    /// its uses decide (ADR-285 D1), so the call is that open number.
    /// **A scope with a task that pauses polls it, and pauses itself**
    /// (ADR-328 D10): its lambda paused because a task in it did. Asked by
    /// `tools/check_core_call.nika` and kept here for the task work (#605).
    pub(super) fn a_scope_that_pauses(&mut self, args: &[Expr], paused: &[i64], span: &Span) {
        if let Some(body) = args.first().filter(|arg| paused.contains(&value_node(arg))) {
            self.s.facts.paused_scopes.insert(value_node(body));
            self.s
                .a_call_that_may_pause("task::scope", true, false, span);
        }
    }

    fn a_call_that_hands_its_number_back(
        &mut self,
        contract: &FnContract,
        bound: &BTreeMap<String, Ty>,
        args: &[Expr],
    ) {
        let Some(signature) = &contract.signature else {
            return;
        };
        let Some(Ty::Var { name, .. }) = &signature.result else {
            return;
        };
        if bound.get(name).is_some_and(|ty| !ty.is_unknown()) {
            return;
        }
        let at: Vec<usize> = signature
            .arguments()
            .iter()
            .enumerate()
            .filter(|(_, (_, ty))| matches!(ty, Ty::Var { name: n, .. } if n == name))
            .map(|(at, _)| at)
            .filter(|&at| at < args.len())
            .collect();
        if !at.is_empty() && at.iter().all(|&at| self.number_shaped(&args[at])) {
            self.s.flow.open_calls.insert(
                value_node(&args[0]),
                at.into_iter().map(|at| at as i64).collect(),
            );
        }
    }

    /// The count and the types of what a call passes, against what it takes.
    ///
    /// `given` is the argument expressions, for the one diagnostic that has to
    /// name the **value** rather than its type: `NK1115` points at the line where
    /// the sharing belongs, and that line starts with the name the caller wrote.
    #[allow(clippy::too_many_arguments)]
    fn arguments(
        &mut self,
        key: &str,
        // The callee as the source writes it, which is the part the emitter can
        // match a recorded argument against (`nullable_args`).
        written: &str,
        contract: &FnContract,
        given: &[Expr],
        found: &[Ty],
        passed: &[(String, Ty)],
        span: &Span,
    ) -> Ty {
        let keeps: Vec<(String, bool, String)> = self
            .newly
            .keeps
            .get(key)
            .map(|all| {
                all.iter()
                    .map(|kept| (kept.param.clone(), kept.now, kept.ty.clone()))
                    .collect()
            })
            .unwrap_or_default();
        self.nikaia(|state, world, host| {
            check_arguments::check_arguments(
                state, world, host, key, written, contract, given, found, passed, &keeps, span,
            )
        })
    }

    // --- the one shape every check has --------------------------------------

    /// Whether `found` fits `want` once a transparent container is seen through.
    ///
    /// A type whose ledger records a `deref` is one a caller is not meant to
    /// notice: Part III says of `fs::Mapped` that "a parser cannot tell the
    /// difference", and the entry spells it — `fs::Mapped::deref` is
    /// `(&Mapped) -> &str`. Nothing consulted that entry, so a function taking a
    /// text view refused a mapped file, which is the opposite of what the
    /// specification promises about it.
    ///
    /// **Only as a rescue, never as a rule of its own.** It is asked after a
    /// direct comparison has already failed, so seeing through a container can
    /// make a refusal into an acceptance and can never turn an acceptance into a
    /// refusal. One step only: a container inside a container is two questions,
    /// and nothing in the language has asked the second yet
    /// ([ADR-288](../../../docs/specification/adr/adr-288.md) D11).
    /// **One arm a number of a type and the other a number nothing typed**
    /// (#497): `if at >= 2 { at - 2 } else { 0 }` is the typed arm's type, and
    /// the other arm is a use of it (ADR-285 D24).
    fn a_number_beside_a_number(
        &mut self,
        (a_tail, a): (Option<&Expr>, &Ty),
        (b_tail, b): (Option<&Expr>, &Ty),
        span: &Span,
    ) -> Option<Ty> {
        let (typed, open) = match (a.is_unknown(), b.is_unknown()) {
            (false, true) => (a, b_tail?),
            (true, false) => (b, a_tail?),
            _ => return None,
        };
        let number = value_of_a_copy(typed.unseen());
        if integer_named(&number).is_none() || !self.number_shaped(open) {
            return None;
        }
        self.number_asked(open, &number, span, true);
        Some(number)
    }

    /// [`Checker::expect`], for a position that has the value in hand, so a
    /// view of text kept where text of its own is wanted can say **why** here
    /// ([ADR-282](../../docs/specification/adr/adr-282.md) D25). `keeper` is the
    /// position's own words for what keeps it.
    /// **Where `expr` was written, or the statement's `span`** (#107): the
    /// parser's span of the expression when it made the node and the node
    /// stands inside the statement; a node the compiler built, or one from
    /// another file, is reported at the statement as before.
    fn span_at(&self, expr: &Expr, span: Span) -> Span {
        nikaia_std::tools::check_arm_leaves::check_span_at(self.world, expr, &span)
    }

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

    /// `NK1108` for a condition that is not a `bool` (`tools/check_exprs.nika`).
    fn expect_bool(&mut self, found: &Ty, span: &Span, why: &str) {
        self.s.expect_bool(found, span, why);
    }

    /// **What a task takes with it** (Part I 8.3, `NK2101`).
    ///
    /// Every name the body reads that is a local or a parameter **of the
    /// enclosing function**, with its type - because the type decides whether
    /// taking it is taking it away:
    ///
    ///   * **Data that is copied** - a number, a `bool`, a `char`, a view -
    ///     goes into the task and stays here too. Rust copies it, so there is
    ///     nothing to refuse, and `let n = 7` followed by a `spawn` that prints
    ///     `n` and a `println` that prints it again is a correct program.
    ///   * **A handle on a `Shared[T]`** is *duplicated* rather than moved
    ///     ([ADR-312](../../docs/specification/adr/adr-312.md) D1, D5), so the
    ///     name outside the task keeps working. Part I 8.3 says `NK2101`
    ///     belongs to the data case only, and this is where that holds.
    ///   * **A type nothing describes** is not claimed about at all. Refusing
    ///     one would be refusing on a guess (Part III, C.4).
    ///
    /// What is left is data that a move takes away, which is what the
    /// diagnostic is about.
    fn a_task_takes_these(&mut self, body: &Expr, task: i64, span: &Span) {
        self.s.a_task_takes_these(self.world, body, task, span);
    }

    /// **A value handed to something that keeps it**
    /// ([ADR-293](../../docs/specification/adr/adr-293.md) D29, which builds
    /// [ADR-094](../../docs/specification/adr/adr-094.md) D2's last paragraph).
    ///
    /// Called at the places a value is given away rather than lent: an argument
    /// the callee keeps, a key or a value written into a map or a list, a field
    /// of a struct being built, an element of a list or a tuple, a `let` that
    /// names it again and an assignment. Only a **name**, and only data a move
    /// takes away - a number, a `bool`, a `char` and a view are copied
    /// (`moves_away`, the rule `NK2101` has for a task).
    ///
    /// What is recorded is asked at the end of the body, as `NK2702` is: a later
    /// read on the same path is `NK2105`. And, as for a sequence, a name from
    /// outside a loop or a lambda handed over inside it is refused where it is
    /// handed, because the next turn hands over what is already gone.
    fn hands_over(&mut self, value: &Expr, ty: &Ty, to: &str, span: &Span) {
        self.s.hands_over(self.world, value, ty, to, span)
    }

    /// The same for a `??` of text, where the position is a comparison
    /// (issue #98).
    fn a_pending_coalesce_of_text_is_lent(&mut self, given: &Expr) -> bool {
        self.s.a_pending_coalesce_of_text_is_lent(given)
    }

    /// A read of a name, recorded once per expression and statement.
    fn a_read(&mut self, expr: &Expr, path: String, span: &Span) {
        self.s.a_read(expr, &path, span);
    }

    /// **The parts of a `mut` parameter an arm took out** (`NK2108`,
    /// [ADR-094](../../docs/specification/adr/adr-094.md) D7): the arm of a
    /// `match` over the parameter, or a place in it, bound a part and handed
    /// it to something that keeps it. A part only read, or changed in place
    /// through a `mut` parameter of its own, is still in the value.
    ///
    /// **And the arm that does is told to the emitter** (#576): it works on
    /// the value taken out of the place, and writes the whole back.
    fn parts_taken_out_of_a_mut_parameter(
        &mut self,
        path: &str,
        body: &Expr,
        subject: &Ty,
        parts: &BTreeMap<String, Ty>,
        handed_from: usize,
    ) {
        self.s.parts_taken_out_of_a_mut_parameter(
            self.world,
            path,
            body,
            subject,
            parts,
            handed_from as i64,
        );
    }

    /// **`NK2104`: two branches of an `overlap` meet on something**
    /// ([ADR-292](../../docs/specification/adr/adr-292.md) D3).
    ///
    /// **This is [ADR-292](../../docs/specification/adr/adr-292.md)'s analysis
    /// used the other way round**, and it is the return on machinery built for
    /// an inference that D1 withdraws. The touch sets no longer decide *whether
    /// the compiler may* overlap two statements; they decide *whether the
    /// programmer was right* to say so. Every mainstream form for "run these
    /// together" takes the programmer's word for the independence — here the
    /// claim is checked against what the ledger records each call touching.
    ///
    /// **A branch this compiler cannot account for is not refused.** `verdict`
    /// answers `NotAccountedFor` where a statement performs nothing it can
    /// name, and reading that as "they meet" would refuse a correct program on
    /// an absence (Part III, C.4) — which is the same polarity every other
    /// answer in this compiler takes about a thing nobody wrote down.
    fn branches_meet_on_nothing(&mut self, block: &Block, span: &Span) {
        self.s.branches_meet_on_nothing(self.world, block);
        let _ = span;
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

    /// Part II 11.2: a task runs on a thread of its own, so everything it takes
    /// with it has to be able to cross one (`NK2501`).
    ///
    /// Over-approximate in the direction that costs nothing: the walk collects
    /// every name the body mentions, including function and field names, and a
    /// name that is not in scope is not found and says nothing.
    fn crosses_into_a_task(&mut self, body: &Expr, span: &Span) {
        self.s.crosses_into_a_task(self.world, body, span);
    }

    /// **What a task holds across a pause** ([ADR-055](../../docs/specification/adr/adr-055.md)
    /// §2 D6's third sharp edge), which is `NK2501`'s question asked of what a
    /// body **binds** rather than of what it captures.
    ///
    /// A task's body is an `async` block below, and a value bound inside it and
    /// still live at a suspension point further down is held **inside the
    /// future**. The pool's starter asks for `Send` of that whole future, so
    /// such a value crosses a thread exactly as a captured one does — and until
    /// now the only thing that said so was the backend, about the generated
    /// file, which [Part III C.1](../../docs/specification/30-nikaia-tooling.md)
    /// calls a bug in this compiler.
    ///
    /// **Live is over-approximated, and the over-approximation is bounded by
    /// the verdict rather than by the walk.** *Bound before a pause and named
    /// after it* is all this asks; Rust's own answer is narrower. That would be
    /// a *correct program refused* ([Part III
    /// C.4](../../docs/specification/30-nikaia-tooling.md)) if the refusal
    /// stood on it alone — it does not. It stands on `contracts::send`'s
    /// `MayNot`, which is a claim about a **type** and is never `Undecided`'s
    /// silence, so a name this walk is too generous about is refused only if a
    /// value of its type could not have crossed from anywhere.
    ///
    /// **It reports nothing today, and that is why it is built now.** No type
    /// answers `MayNot` at `Destination::Ours`: `Shared` stopped being one when
    /// [ADR-037](../../docs/specification/adr/adr-037.md) D6 gave it one
    /// representation at both settings, and a lock is `MayNot` only at a
    /// *foreign* destination. A refusal costs nothing before there are programs
    /// it would reject, and the same refusal added afterwards breaks them. What
    /// reaches it first is a type from outside this language, which is what
    /// [ADR-290](../../docs/specification/adr/adr-290.md) is for.
    fn a_task_holds_these_across_a_pause(
        &mut self,
        body: &Block,
        bound: &[(String, Ty, i64)],
        span: &Span,
    ) {
        self.nikaia(|state, world, host| {
            state.a_task_holds_these_across_a_pause(world, host, body, bound, span)
        })
    }

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

    /// Whether more than one error **type** can arrive at a handler
    /// ([ADR-280](../../docs/specification/adr/adr-280.md) D18).
    ///
    /// Read off the guarded call's own set, the way the lowering reads it: only
    /// the outermost call and only one by name, because that is what a `catch`
    /// guards in every program in the tree.
    fn several_arrive(&self, guarded: &Expr) -> bool {
        self.s.several_arrive(self.world, guarded)
    }

    /// The **one** error type a guarded expression can fail with, where exactly
    /// one is written down.
    ///
    /// The same resolution [`Checker::several_arrive`] does, asked for the other
    /// answer and over **both** ledgers: a `std` call's failure is in the
    /// library's, and that is the one a corpus handler actually catches.
    ///
    /// `None` where several arrive, where the callee is not one this compiler
    /// can resolve, or where the single member is `"?"` — the absence of a claim
    /// ([ADR-024](../../docs/specification/adr/adr-024.md) D1), which names no
    /// type and therefore no cases.
    fn the_one_error(&self, expr: &Expr) -> Option<Ty> {
        self.s.the_one_error(self.world, expr)
    }

    /// **`NK1151` for a `match` over a `catch`'s error**
    /// ([ADR-280](../../docs/specification/adr/adr-280.md) D18).
    ///
    /// The variants **within** one error type are closed and the set of error
    /// **types** is open ([ADR-023](../../docs/specification/adr/adr-023.md)
    /// D4), so a handler that names variants of two of them has covered no set
    /// at all — a callee that gains a failure sends a third type here, and the
    /// `match` has nowhere to put it. `else` is what says the rest, which is
    /// the same answer this code gives for every type whose cases cannot be
    /// enumerated in arms.
    ///
    /// It is the caret for a refusal the **lowering** also states, which is the
    /// arrangement `NK1132` already has: the emitter cannot write the file
    /// either way, and this is where a reader finds out why.
    fn a_match_over_several_error_types(
        &mut self,
        value: &Expr,
        arms: &[ast::MatchArm],
        span: &Span,
    ) {
        self.s
            .a_match_over_several_error_types(self.world, value, arms, span);
    }

    /// **`NK1205`: a variant matched on a `T?`** (issue #180): `match
    /// g(0) { W::B => 2, else => 3 }` for a `g` that hands back a `W?` names a
    /// variant of `W` where the value may be `null`, and was lowered as a
    /// `match` over an `Option<W>` with `W::B` as its pattern - `rustc`'s
    /// *mismatched types* about a file nobody wrote (Part III C.1). The way
    /// out is to say what `null` stands for first, with `??`.
    fn a_variant_matched_on_a_nullable(
        &mut self,
        on: &Ty,
        value: &Expr,
        arms: &[ast::MatchArm],
        span: &Span,
    ) {
        self.s
            .a_variant_matched_on_a_nullable(self.world, on, value, arms, span);
    }

    fn a_match_that_misses_a_case(&mut self, on: &Ty, arms: &[ast::MatchArm], span: &Span) {
        self.s
            .a_match_that_misses_a_case(self.world, on, arms, span);
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

    /// **Arms that are known and disagree** (#596, `NK1245`): the compiler
    /// cannot tell the type of the value, and no arm is unknown for another
    /// refusal's sake. Said unless a position asks the arms one by one.
    fn arms_that_disagree(&mut self, expr: &Expr, arms: &[(Option<&Expr>, Ty)], span: &Span) {
        if self.s.discarded.contains(&value_node(expr)) {
            return;
        }
        let asked: Vec<CheckArm<'_>> = arms
            .iter()
            .map(|(tail, ty)| CheckArm {
                tail: *tail,
                ty: ty.clone(),
            })
            .collect();
        let is_if = matches!(expr, Expr::If { .. });
        let Some(mut finding) =
            nikaia_std::tools::check_arms::arms_that_disagree(&asked, is_if, span)
        else {
            return;
        };
        // **The caret stands on the first arm that is not the first one's
        // type** (#107), or on the `if` or `match` where none can be named.
        let first = arms.first().map(|(_, ty)| ty.unseen());
        finding.span = arms
            .iter()
            .skip(1)
            .find_map(|(tail, ty)| (Some(ty.unseen()) != first).then_some(*tail)?)
            .map_or_else(|| self.span_at(expr, *span), |arm| self.span_at(arm, *span));
        self.s.arm_disagreements.insert(value_node(expr), finding);
    }

    /// A text literal where `String` is wanted, built as text of its own
    /// (`tools/check_texts.nika`).
    fn text_literal(&mut self, want: &Ty, value: &Expr, owned: bool) -> Option<Ty> {
        self.s.text_literal(self.world, want, value, owned)
    }

    /// **Arms that disagree only because some are text literals and some are
    /// text of their own agree on `String`** (ADR-282 D4): the literals take
    /// their neighbours' type, as they do in a list. `if c { name } else
    /// { "anonymous" }` over a `name: String` is a `String`, and the literal
    /// arm is constructed as one. A **view** arm is not a literal, so an arm
    /// holding one keeps the disagreement.
    /// **Arms that meet at `T?`** because one of them is `null`: the plain
    /// ones are `Some(…)` below ([`Checked::some_tails`]), and the choice is a
    /// `T?`. Asked only where every arm that is not `null` is a `T` or a `T?`
    /// of one `T`, so a choice this cannot type stays unknown.
    ///
    /// **Or because one of them is a `T?` already** (#435): `match t { Int(n)
    /// => Form { … }, Add(a, b) => sum(…) }` with `sum` handing back a
    /// `Form?` is a `Form?`, the plain arm `Some(…)`. Wrapped as a whole, the
    /// choice went below as `match … { … }.into()`, and the language below
    /// said *`match` arms have incompatible types* before the conversion
    /// could be reached.
    fn arms_meet_at_null(&mut self, arms: &[(Option<&Expr>, Ty)]) -> Option<Ty> {
        let arms: Vec<CheckArm<'_>> = arms
            .iter()
            .map(|(tail, ty)| CheckArm {
                tail: *tail,
                ty: ty.clone(),
            })
            .collect();
        self.s.arms_meet_at_null(self.world, &arms)
    }

    fn arms_meet_at_text(&mut self, arms: &[(Option<&Expr>, Ty)], span: &Span) -> Option<Ty> {
        let text = Ty::named("String");
        if !arms.iter().any(|(_, ty)| *ty == text) {
            return None;
        }
        // **An arm beside text is text** (ADR-319 D4): an `f"…"` with a
        // file's name in it, there, is refused at the hole.
        for (arm, _) in arms {
            self.a_name_in_text(&text, *arm, span);
        }
        let meets = arms
            .iter()
            .all(|(arm, ty)| *ty == text || arm.is_some_and(ends_in_text));
        if !meets {
            return None;
        }
        for (arm, ty) in arms {
            if let (Some(arm), false) = (arm, *ty == text) {
                self.text_literal(&text, arm, true);
            }
        }
        Some(text)
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

    /// **`NK1171`: `X::y`, where `X` is a type this program declares and `y` is
    /// nothing it has.**
    ///
    /// **Only where the compiler *knows*.** A path whose head names nothing at
    /// all — `nowhere::wobble` — is a different question and is not this one:
    /// a module of a package, a foreign crate's item and a name the ledger has
    /// not been told about all look the same from here, and refusing on absence
    /// would refuse correct programs. That case is
    /// issue #141, with the measurement
    /// that says how rare it is. What is answered here is the case where this
    /// compiler has read the declaration and can see that the name is not in
    /// it: the same knowledge `NK1135`'s map and the exhaustiveness check
    /// already read, asked one question earlier.
    ///
    /// **What it was before.** `Op::Mul` beside `enum Op { Add, Sub }` lowered,
    /// and `rustc` refused the **generated file** — [Part III
    /// C.1](../../docs/specification/30-nikaia-tooling.md)'s class. The
    /// exhaustiveness check could not say it: it reads which variants an arm
    /// *covered*, so a misspelling covers none and what it reports is the
    /// variant that is missing; with an `else` arm beside it, it reports
    /// nothing.
    fn a_member_this_type_does_not_have(&mut self, ty: &str, member: &str, span: &Span) {
        self.s
            .a_member_this_type_does_not_have(self.world, ty, member, span);
    }

    /// The same question asked of a `match` arm's pattern
    /// ([ADR-291](../../docs/specification/adr/adr-291.md) D10's shapes), which
    /// is where a misspelled variant is most likely to be written and least
    /// likely to be noticed.
    fn a_pattern_naming_a_member_a_type_does_not_have(
        &mut self,
        pattern: &MatchPattern,
        span: &Span,
    ) {
        self.s
            .a_pattern_naming_a_member_a_type_does_not_have(self.world, pattern, span);
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

    /// **One instantiation of a function that walks a type's fields**
    /// ([ADR-304](../../docs/specification/adr/adr-304.md) D6).
    ///
    /// The type argument is read off the **first argument**, which is the shape
    /// Part II 10.3 writes and the only one this compiler can read: a bound
    /// binds `T` from a parameter's type, and `describe(u)` is where `u`'s type
    /// says which struct. A call whose argument this checker did not type is
    /// left alone rather than guessed at — `?` fits everything
    /// ([ADR-024](../../docs/specification/adr/adr-024.md) D1) and an
    /// instantiation made from one would be a wrong answer where a missing one
    /// is right ([ADR-010](../../docs/specification/adr/adr-010.md) D1).
    /// The element type of `T::member`, where `T` is a type parameter whose
    /// shape bound makes that member reachable.
    fn shape_element(&self, ty: &str, member: &str) -> Option<&'static str> {
        self.s.type_parameters.get(ty)?.iter().find_map(|bound| {
            shape_member(bound)
                .and_then(|(reached, element)| (reached == member).then_some(element))
        })
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

    /// Walk something the language below makes **a function of its own**.
    ///
    /// A lambda, a task, an `overlap` branch: three constructs that are one
    /// block above and a closure or an `async` block below. A loop outside one
    /// of them is not reachable from inside it, so the count starts again at
    /// zero and the word for what stands in the way is carried for the message.
    fn enter_boundary(&mut self, what: &str) -> Boundary {
        self.s.enter_boundary(what)
    }

    /// Back out of what [`Checker::enter_boundary`] went into.
    fn leave_boundary(&mut self, saved: Boundary) {
        self.s.leave_boundary(&saved);
    }

    /// **A fatal error passes a `catch` that does not name it**
    /// ([ADR-345](../../docs/specification/adr/adr-345.md) D2), so it leaves
    /// the function, which says `throws` (`NK2605`), or the `catch` around it
    /// counts it.
    fn a_fatal_error_passes(&mut self, guarded: &Guarded, handler: &Block, span: &Span) {
        self.s
            .a_fatal_error_passes(self.world, guarded, handler, span);
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

    /// **`dsl G { … } eod` over a grammar of this program** (#518,
    /// `tools/check_core_comptime.nika`).
    fn a_block_over_a_grammar(
        &mut self,
        block: &Expr,
        target: Ident,
        content: &str,
        span: &Span,
    ) -> Ty {
        self.nikaia(|state, world, host| {
            check_core_comptime::check_core_a_block_over_a_grammar(
                state, world, host, block, target, content, span,
            )
        })
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

    /// **What a `return` hands back, asked once**
    /// ([ADR-276](../../docs/specification/adr/adr-276.md) D21): the statement
    /// and the expression are the same `return`, so they ask the same
    /// questions in the same order rather than in two places that drift.
    fn returns(&mut self, value: Option<&Expr>, span: &Span, in_an_expression: bool) {
        self.nikaia(|state, world, host| {
            check_core_leaves::check_core_returns(state, world, host, value, span, in_an_expression)
        })
    }

    /// `NK1132`: a `break` or a `continue` with no loop to act on.
    ///
    /// Two shapes and one code, because they are one mistake reached from two
    /// sides - there is no loop, or there is one and a function boundary
    /// stands between. The second is the one worth the code: it is a program
    /// that *looks* right, and without this it would be refused by `rustc`
    /// about the emitted file (Part III, C.1).
    fn a_jump_with_nowhere_to_go(&mut self, word: &str, span: &Span) {
        self.s.a_jump_with_nowhere_to_go(word, span);
    }

    /// Note where a method call in the function being walked went (ADR-288).
    ///
    /// `None` is "I could not find out", and it is recorded rather than
    /// dropped: an analysis that claims a property has to be able to tell that
    /// apart from a body that called nothing.
    fn reached_method(&mut self, key: Option<&str>) {
        // **Before the `current` gate**, which is about which function's set an
        // answer lands in and not about whether there is one: a method nothing
        // describes says nothing about failing, wherever it stands
        // ([ADR-308](../../../docs/specification/adr/adr-308.md)).
        if key.is_none() {
            self.guard_has_no_answer();
        }
        for frame in &mut self.s.stored_frames {
            match key {
                Some(key) => {
                    frame.resolved.insert(key.to_string());
                }
                None => frame.unresolved = true,
            }
        }
        let Some(current) = self.s.current.clone() else {
            return;
        };
        // **Which side of a `spawn` this call is on** (ADR-281 D29). The stack
        // is the one the task walk already keeps, so "inside a task" is
        // exactly what it says.
        let inside_a_task = !self.s.task_bindings.is_empty();
        let entry = self
            .s
            .methods
            .entry(current)
            .or_insert_with(MethodCalls::empty);
        match key {
            Some(key) => {
                entry.resolved.insert(key.to_string());
                if inside_a_task {
                    entry.in_a_task.resolved.insert(key.to_string());
                }
            }
            None => {
                entry.unresolved = true;
                entry.unanswered += 1;
                entry.in_a_task.unresolved |= inside_a_task;
            }
        }
    }

    /// **Something the guarded expression of a `catch` holds and this compiler
    /// cannot see the end of** ([ADR-308](../../../docs/specification/adr/adr-308.md)).
    ///
    /// Called where a call could not be resolved and where a `catch` stands
    /// inside another one's guarded expression. A no-op outside a guarded
    /// expression, which is most of a program.
    fn guard_has_no_answer(&mut self) {
        if let Some(guarded) = &mut self.s.flow.guarded {
            guarded.unanswered = true;
        }
    }

    /// **Whether a `match` is over a place this function owns**: a name, or a
    /// field of one, that is not itself a view. Only there can a part be lent
    /// and the whole stay: a temporary is gone after the `match` anyway, and a
    /// view's parts are views already.
    fn an_owned_place(&self, value: &Expr, on: &Ty) -> bool {
        self.s.an_owned_place(self.world, value, on)
    }

    /// **The parts an arm only reads are lent** (ADR-291): a part
    /// that does not copy, which nothing in the arm took, returned or handed
    /// back as the arm's value, is bound as a view, and the scrutinee stays
    /// whole for what comes after the `match`.
    fn a_part_lent_where_it_is_not_kept(
        &mut self,
        body: &Expr,
        parts: &BTreeMap<String, Ty>,
        (_, from): (i64, i64),
        changed_from: usize,
        bound_by_reference: bool,
    ) {
        let lent = self
            .s
            .parts_lent(self.world, body, parts, from, changed_from as i64);
        if !lent.is_empty() {
            self.a_lent_part_is_handed_on_as_it_is(body, &lent);
            // **Over a `mut` parameter the parts are references already**
            // (the place is a `&mut` below), and `ref` in a pattern that
            // borrows by itself is the language below's error (#576).
            if !bound_by_reference {
                let at = value_node(body);
                self.s.facts.lent_bindings.insert(at, lent);
            }
        }
    }

    /// **A part the arm binds `ref` is a view already** (issue #270):
    /// a call that lends it hands it on as it is, where the reference the
    /// compiler writes for a lent argument made it a view of a view - the same
    /// code below, and a `&` the language below's lint points at.
    ///
    /// Decided here, after the arm, because whether a part is lent is known
    /// only once the arm is walked; the arguments recorded meanwhile are
    /// found again by their ids. Only where nothing in the arm binds the name
    /// again.
    fn a_lent_part_is_handed_on_as_it_is(&mut self, body: &Expr, lent: &BTreeSet<String>) {
        let host = self.host();
        self.s
            .a_lent_part_is_handed_on_as_it_is(self.world, &host, body, lent);
    }

    /// The names a `match` arm brings into scope, all of them unknown: what a
    /// variant carries is not in the ledger yet.
    fn pattern_bindings(&mut self, pattern: &MatchPattern) -> Vec<Local> {
        let mut bound = Vec::new();
        for name in pattern_names(self.world, pattern) {
            bound.push(self.local_free(name, Ty::Unknown));
        }
        bound
    }
}

/// **A lazy walk of a sequence whose step pauses**
/// ([ADR-233](../../docs/specification/adr/adr-233.md) D1,
/// [ADR-234](../../docs/specification/adr/adr-234.md) D1): one this compiler
/// made pause, by handing a pausing lambda to a `map` or a `filter`, or
/// `io::lines()`, whose step can fail as well. Both have every lazy walk `Seq`
/// has but `rev`, as adapters of their own.
fn a_paused_chain(key: &str, on: &Ty) -> bool {
    nikaia_std::tools::check_calls::a_paused_chain(key, on)
}

/// What a `map` or a `filter` hands back where its step pauses
/// ([ADR-233](../../docs/specification/adr/adr-233.md) D1): a sequence whose
/// step pauses, walked from the front only and with no length until the end.
///
/// **And one whose step can fail where what it walks can**
/// ([ADR-234](../../docs/specification/adr/adr-234.md) D1): a `map` of
/// `io::lines()` has produced nothing yet, so its failures are still ahead of
/// whatever walks it, and the `?` goes there.
fn a_pausing_sequence(result: Ty, on: &Ty) -> Ty {
    nikaia_std::tools::check_calls::a_pausing_sequence(result, on)
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
fn parsed_view_of(parsed: &Parsed) -> ParsedView<'_> {
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
pub fn value_node(expr: &Expr) -> i64 {
    crate::ast::id_of(expr) as i64
}

/// **`an i32` and `a u64`**: the article as the name is said (`NK1116`'s rule).
/// A `u` is said *you*, so it takes `a`; the other vowels take `an`.
pub fn an_or_a(ty: &str) -> String {
    nikaia_std::tools::render::an_or_a(ty)
}

/// **A handler's `error` where several error types arrive** (ADR-280 D30):
/// an `Error` with its members.
fn several_errors(ty: &Ty) -> Option<Vec<Ty>> {
    Some(nikaia_std::tools::check_expect::several_errors_of(ty)).filter(|m| !m.is_empty())
}

/// The value a block ends in: its last statement, where that is an expression.
/// The value a braced arm ends in: `{ { x } }` is `x`.
fn peeled(value: &Expr) -> &Expr {
    nikaia_std::tools::check_types::check_peeled(value)
}

fn tail_of(block: &Block) -> Option<&Expr> {
    nikaia_std::tools::check_types::tail_value_of(block)
}

/// A view of a value that copies read as the value, and anything else as it is
/// ([ADR-233](../../docs/specification/adr/adr-233.md) D4).
impl Checker<'_> {}

fn value_of_a_copy(ty: Ty) -> Ty {
    nikaia_std::tools::check_types::value_of_a_copy(&ty)
}

/// **`--asserts`** ([ADR-269](../../docs/specification/adr/adr-269.md) D7):
/// every `assert` of one file, by line, and how the compiler holds it: proved,
/// a precondition and how many calls check it, checked at run time and why, a
/// test's check, or refused (D6) - and how many of each, so that the number
/// checked at run time can be watched.
pub fn claims_report(
    parsed: &Parsed,
    source: &str,
    path: &str,
    own: &Ledger,
    library: &Ledger,
) -> String {
    let checked = check(parsed, own, library);
    nikaia_std::tools::check_facts::check_claims_report(&checked.facts.claims, source, path)
}

/// The prelude's claim ([ADR-269](../../docs/specification/adr/adr-269.md) D2).
pub const ASSERT: &str = "assert";

/// In Nikaia (`tools/written.nika`, ADR-294, #125).
pub fn written(parsed: &Parsed, expr: &Expr) -> String {
    nikaia_std::tools::written::written_expression(&parsed.interner, expr)
}

fn is_number(name: &str) -> bool {
    nikaia_std::tools::check_types::a_number_name(name)
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

/// A list, a map or a set - the types Part I gives no operator.
fn is_a_collection(ty: &Ty) -> bool {
    nikaia_std::tools::check_types::is_a_collection_type(ty)
}

/// Whether a type is one this language calls text.
///
/// `String` and `&str`, and nothing else. A `T?` is deliberately **not** text:
/// `maybe + "x"` is a member reached off a nullable, which Part I 2.3 answers
/// and this must not quietly paper over.
fn is_text(ty: &Ty) -> bool {
    nikaia_std::tools::check_types::is_text_type(ty)
}

/// The whole-number types an operation may be written unchecked in
/// (ADR-306 D10).
fn is_a_whole_number(name: &str) -> bool {
    nikaia_std::tools::bounds_shape::is_whole_number(name)
}

/// Where a task handle stands on the paths through some statements (`NK2109`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum HandlePath {
    /// Every path takes it.
    Taken,
    /// Every path takes it and leaves the scope (`return h`, `throw`).
    Gone,
    /// Some path leaves the scope, or ends a branch, without taking it.
    Left,
    /// No path mentions it; control falls through.
    Falls,
}
