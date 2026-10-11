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

use crate::assets::{ASSET, Reads};
use crate::ast::{self, BinaryOp, Block, Expr, Item, MatchPattern, Span, Stmt, UnaryOp};
use crate::build_time;
use crate::contracts::SignatureOps;
use crate::contracts::ty::TyOps;
use crate::contracts::{FieldContract, FnContract, Ledger, ty, ty::Ty};
use crate::fold::Constant;
use crate::parser::Parsed;
use nikaia_std::tools::check_consts::{self, CheckCrossing as Crossing};
use nikaia_std::tools::check_core_pilot;
use nikaia_std::tools::check_flow::{
    CheckConstant, CheckFlow, CheckOpenNumber as OpenNumber, CheckUnrolling,
    CheckUnsignedSlot as UnsignedSlot,
};
use nikaia_std::tools::check_items;
use nikaia_std::tools::check_literals::element_kind;
use nikaia_std::tools::check_lookups::names_of;
use nikaia_std::tools::check_names::{integer_named, is_a_literal as is_literal};
use nikaia_std::tools::check_patterns::pattern_names;
use nikaia_std::tools::check_reports::CheckWall;
use nikaia_std::tools::check_scope::{resolve_call, resolve_method};
use nikaia_std::tools::check_shapes::{
    an_element_beneath, constructed, element_below, is_fixed, is_growable, roots_in_a_binding,
    rust_constant_type, slice_element,
};
pub use nikaia_std::tools::check_state::{
    CheckFinding as Finding, CheckLabel as Label, CheckSeverity as Severity,
};
use nikaia_std::tools::check_state::{
    CheckGive as Give, CheckGuarded as Guarded, CheckHanded as Handed, CheckImmutable as Immutable,
    CheckKind as Kind, CheckLocal as Local, CheckPendingCoalesce as PendingCoalesce,
    CheckPromises as Promises, CheckRepeats as Repeats, CheckState, CheckWorld, a_body_never_ends,
};
use nikaia_std::tools::check_texts::{self, ends_in_text};
use nikaia_std::tools::check_widen::{
    self, an_integer_that_does_not_widen, names_a_path, number_where_no_number_is,
    views_into_nullable_text, views_into_text,
};
use nikaia_std::tools::check_with::CheckCopyable as Copyable;
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
            parsed
                .program
                .items
                .iter()
                .find_map(|item| match &item.node {
                    Item::Enum { name, variants, .. } if parsed.text(*name) == ty => variants
                        .iter()
                        .find(|held| parsed.text(held.name) == variant)
                        .map(|held| match &held.fields {
                            ast::VariantFields::Unit => Vec::new(),
                            ast::VariantFields::Tuple(types) => {
                                types.iter().map(|t| Ty::from_ast(parsed, t)).collect()
                            }
                            ast::VariantFields::Named(fields) => {
                                fields.iter().map(|f| Ty::from_ast(parsed, &f.ty)).collect()
                            }
                        }),
                    _ => None,
                })
        })
}

/// How a lent `??` opens its option, as the emitter is told.
fn opened_text(opened: &str) -> &'static str {
    match opened {
        ".as_deref()" => ".as_deref()",
        _ => ".as_ref()",
    }
}

use winnow_grammar::Symbol as Ident;

/// The types Part I 2.2 offers, which is what an `as` may name
/// ([ADR-285](../../../docs/specification/adr/adr-285.md) D14).
///
/// Wider than `record_cast`'s `NUMERIC`, and the two answer different questions:
/// that one is which conversions are **checked** at run time
/// ([ADR-285](../../../docs/specification/adr/adr-285.md) D12), this one is which
/// types a program may **write**. `bool`, `char`, `String` and `&str` are types
/// of this language and no conversion between them narrows anything.
const OFFERED: [&str; 11] = [
    "i32", "i64", "u8", "u32", "u64", "f64", "bool", "char", "scalar", "String", "str",
];

/// **What one call was handed**, positional and named, with the types found for
/// each.
///
/// One parameter rather than four, because the two callers of
/// [`Checker::crosses_into_an_unseen_call`] pass the same four and the question
/// they differ on is *why* — which is the argument beside this one.
struct Arguments<'a> {
    args: &'a [Expr],
    found: &'a [Ty],
    config: &'a [ast::ConfigArg],
    passed: &'a [(String, Ty)],
}

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

/// A field's default as the checker keeps it (ADR-331 D5): a literal's text,
/// or a mark that one is there, which is all a literal leaving the field out
/// asks. The value a computed one has is the ledger's.
fn default_written(default: &ast::FieldDefault) -> String {
    match crate::contracts::a_literal(&default.value) {
        true => nikaia_std::tools::declared::default_text(&default.value),
        false => "computed".to_string(),
    }
}

fn wrap_for(found: &Ty, want: &Ty, literal: bool) -> Option<Wrap> {
    if let Some(into) = widened_into(found, want) {
        return Some(Wrap::Widen(into.to_string()));
    }
    if let Some(view) = text_as_a_path(found, want) {
        return Some(match view {
            true => Wrap::PathView,
            false => Wrap::Path,
        });
    }
    if !matches!(want, Ty::Nullable(_)) || matches!(found, Ty::Nullable(_)) {
        return None;
    }
    match !found.is_unknown() || literal {
        true => Some(Wrap::Constructor),
        false => Some(Wrap::Conversion),
    }
}

/// **The unsigned type a signed value goes into where it is not negative**
/// (`tools/check_widen.nika`).
fn unsigned_from_signed(found: &Ty, want: &Ty) -> Option<&'static str> {
    check_widen::unsigned_from_signed(found, want).map(|into| a_static_integer(&into))
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

/// **The integer type a slot states, where a narrower one goes into it without
/// loss** (`tools/check_widen.nika`).
fn widened_into(found: &Ty, want: &Ty) -> Option<&'static str> {
    check_widen::widened_into(found, want).map(|into| a_static_integer(&into))
}

/// The ledger keys of a file name's methods (ADR-319 D3).
const PATH_METHODS: &str = "fs::Path::";

/// **Text stands wherever a file's name is asked**: whether `found` is text and
/// `want` an `fs::Path`, and then whether a view of one is wanted.
fn text_as_a_path(found: &Ty, want: &Ty) -> Option<bool> {
    match check_widen::text_as_a_path(found, want) {
        -1 => None,
        view => Some(view == 1),
    }
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

/// The Rust element type of a build-time array nothing declared a type for
/// (`tools/check_consts.nika`).
fn rust_array_type(items: &[build_time::Value]) -> Option<String> {
    nikaia_std::tools::check_consts::rust_array_type(items)
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

/// One argument as [`Checker::the_compiler_writes_the_reference`] reads it:
/// where it stands in the call, what was written there, the type that was
/// found, and the one the parameter wants. Four values that only ever travel
/// together, and a name for them so the rule reads as one question.
struct Argument<'a> {
    /// The position among the arguments, the receiver not counted.
    at: usize,
    /// What the source wrote there, where there is an expression to read.
    given: Option<&'a Expr>,
    /// The type of what was written.
    found: &'a Ty,
    /// The type the parameter declares.
    want: &'a Ty,
    /// The callee is a function or method of this program's own.
    own: bool,
}

/// **What a `?.` reaches**, which Part I 3.5 calls a *member*
/// ([ADR-278](../../../docs/specification/adr/adr-278.md)).
///
/// The two spellings are one operator, and the only place the difference has to
/// be said out loud is the refusal: a reader who wrote a call is owed `.m(…)`
/// as the way out and not `.m`.
enum Reached<'a> {
    Field(&'a str),
    Method(&'a str),
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
        // **A generic struct another file of the package declares**
        // (Part I 9.1): its fields and type parameters, so `item.node` over a
        // `Spanned[Item]` that `ast.nika` declares is an `Item` in
        // `traits.nika` as it is in `ast.nika` (found moving `traits` into
        // Nikaia, #125). This file's own declarations come after, and a name
        // it declares is its own.
        for other in self.beside {
            for item in &other.program.items {
                // **And an enum's variants**: `Item::Impl { methods, .. }`
                // over an `Item` `ast.nika` declares binds a `methods` of the
                // type the variant says, here as there.
                if let Item::Enum { name, variants, .. } = &item.node {
                    let owner = other.text(*name).to_string();
                    for variant in variants {
                        let key = format!("{owner}::{}", other.text(variant.name));
                        match &variant.fields {
                            ast::VariantFields::Named(fields) => {
                                let held: Vec<FieldContract> = fields
                                    .iter()
                                    .map(|f| FieldContract {
                                        name: other.text(f.name).to_string(),
                                        ty: Ty::from_ast(other, &f.ty),
                                        public: true,
                                        default: String::new(),
                                        attributes: Vec::new(),
                                    })
                                    .collect();
                                self.s.variant_owner.insert(key.clone(), owner.clone());
                                self.s.structs.insert(key, held);
                            }
                            ast::VariantFields::Tuple(types) => {
                                let parts =
                                    types.iter().map(|ty| Ty::from_ast(other, ty)).collect();
                                self.s.variant_parts.insert(key, parts);
                            }
                            _ => {}
                        }
                    }
                    continue;
                }
                let Item::Struct {
                    name,
                    generics,
                    fields,
                    ..
                } = &item.node
                else {
                    continue;
                };
                if generics.is_empty() {
                    continue;
                }
                let order: Vec<String> = generics
                    .iter()
                    .map(|g| other.text(g.name).to_string())
                    .collect();
                let parameters: BTreeSet<String> = order.iter().cloned().collect();
                let held: Vec<FieldContract> = fields
                    .iter()
                    .map(|f| FieldContract {
                        name: other.text(f.name).to_string(),
                        ty: Ty::from_ast(other, &f.ty).parameterise(&parameters),
                        public: f.is_public,
                        default: f.default.as_ref().map(default_written).unwrap_or_default(),
                        attributes: Vec::new(),
                    })
                    .collect();
                let named = other.text(*name).to_string();
                self.s.struct_parameters.insert(named.clone(), order);
                self.s.structs.insert(named, held);
            }
        }
        for item in &self.parsed.program.items {
            match &item.node {
                Item::Struct {
                    name,
                    generics,
                    fields,
                    ..
                } => {
                    // In **declaration order**, because that is what a type
                    // argument's position means: `Pair[i64]`'s `i64` is the
                    // first parameter and nothing else says which
                    // ([ADR-295](../../docs/specification/adr/adr-295.md) D2).
                    let order: Vec<String> = generics
                        .iter()
                        .map(|g| self.parsed.text(g.name).to_string())
                        .collect();
                    let parameters: BTreeSet<String> = order.iter().cloned().collect();
                    for f in fields {
                        let name = self.parsed.text(f.name).to_string();
                        self.s.nameable(&name, &f.span, "a field");
                    }
                    let fields: Vec<FieldContract> = fields
                        .iter()
                        .map(|f| FieldContract {
                            name: self.parsed.text(f.name).to_string(),
                            ty: self.declared(&f.ty, &f.span).parameterise(&parameters),
                            public: f.is_public,
                            default: f.default.as_ref().map(default_written).unwrap_or_default(),
                            attributes: Vec::new(),
                        })
                        .collect();
                    let own = self.parsed.text(*name).to_string();
                    // **An item's own name was never asked**, which `not_self`
                    // did not have to be: `struct self` cannot parse, because
                    // the receiver takes the word. `struct crate` parses fine
                    // and went to the language below (ADR-298 D11).
                    self.s.nameable(&own, &item.span, "a struct");
                    if !order.is_empty() {
                        self.s.struct_parameters.insert(own.clone(), order);
                    }
                    self.s.structs.insert(own, fields);
                }
                Item::Enum { name, variants, .. } => {
                    let own = self.parsed.text(*name).to_string();
                    self.s.nameable(&own, &item.span, "an enum");
                    let named: Vec<String> = variants
                        .iter()
                        .map(|v| self.parsed.text(v.name).to_string())
                        .collect();
                    for name in &named {
                        self.s.nameable(name, &item.span, "a variant");
                    }
                    // **A variant with named fields wears a struct literal**,
                    // and until this was here nothing read it: `Shape::Spot {
                    // x: 1 }` found no fields, so every field went unchecked,
                    // and the literal's *type* came out as `Shape::Spot` -
                    // which no declaration can be written as, so a correct
                    // program was refused with a way out that cannot be taken
                    // ([Part III C.2](../../docs/specification/30-nikaia-tooling.md)
                    // and C.4 at once).
                    //
                    // The fields go in the same map a `struct`'s do, under the
                    // qualified key, so `fields_of` answers for both and there
                    // is one field check rather than two. **`pub` is not asked
                    // of them**: a variant carries no visibility word, so its
                    // fields are as reachable as the `enum` is (Part I 9.2).
                    for variant in variants {
                        let ast::VariantFields::Named(fields) = &variant.fields else {
                            continue;
                        };
                        let held: Vec<FieldContract> = fields
                            .iter()
                            .map(|f| FieldContract {
                                name: self.parsed.text(f.name).to_string(),
                                ty: self.declared(&f.ty, &f.span),
                                public: true,
                                default: String::new(),
                                attributes: Vec::new(),
                            })
                            .collect();
                        let key = format!("{own}::{}", self.parsed.text(variant.name));
                        self.s.variant_owner.insert(key.clone(), own.clone());
                        self.s.structs.insert(key, held);
                    }
                    for variant in variants {
                        if let ast::VariantFields::Tuple(types) = &variant.fields {
                            let key = format!("{own}::{}", self.parsed.text(variant.name));
                            let parts = types
                                .iter()
                                .map(|ty| Ty::from_ast(self.parsed, ty))
                                .collect();
                            self.s.variant_parts.insert(key, parts);
                        }
                    }
                    // **Every kind of payload**, positional included — see
                    // [`Checker::enum_payloads`] for why a map of its own.
                    let held: Vec<Ty> = variants
                        .iter()
                        .flat_map(|variant| match &variant.fields {
                            ast::VariantFields::Unit => Vec::new(),
                            ast::VariantFields::Tuple(types) => types
                                .iter()
                                .map(|ty| Ty::from_ast(self.parsed, ty))
                                .collect(),
                            ast::VariantFields::Named(fields) => fields
                                .iter()
                                .map(|f| Ty::from_ast(self.parsed, &f.ty))
                                .collect(),
                        })
                        .collect();
                    self.s.enum_payloads.insert(own.clone(), held);
                    let variants = variants
                        .iter()
                        .map(|v| self.parsed.text(v.name).to_string())
                        .collect();
                    self.s
                        .enums
                        .insert(self.parsed.text(*name).to_string(), variants);
                }
                _ => {}
            }
        }
        self.s.collect_library_enums(self.world);
        for item in &self.parsed.program.items {
            match &item.node {
                Item::Fn { .. } => self.s.bounds_declared_by(self.world, &item.node, None),
                Item::Impl {
                    target, methods, ..
                } => {
                    let target = self.parsed.text(target.name).to_string();
                    for method in methods {
                        self.s
                            .bounds_declared_by(self.world, &method.node, Some(&target));
                    }
                }
                _ => {}
            }
        }
        self.collect_field_walks();
        self.collect_comparisons();
        self.collect_copies();
    }

    /// **A view of a value that copies**, for an argument handed where the
    /// value is wanted (ADR-233 D4): a number, a `bool` or a `char`, and
    /// beside them a type that is one by declaration - the unit's own that
    /// derive `Copy`, and a described Rust type whose ledger says `copies`
    /// (ADR-294 D9.3). `for name in names` over a `ref Vec[Symbol]` binds a
    /// view of each `Symbol`, and handing one to `resolve` was refused as a
    /// mismatch although the handle is a `u32` the ledger says copies.
    fn a_view_that_copies(&self, ty: &Ty) -> bool {
        self.s.a_view_that_copies(self.world, ty)
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
        let mut beside_parts: BTreeMap<String, Vec<Ty>> = BTreeMap::new();
        for other in self.others() {
            for item in &other.program.items {
                let (name, parts): (String, Vec<Ty>) = match &item.node {
                    Item::Struct {
                        name,
                        generics,
                        fields,
                        ..
                    } if generics.is_empty() => (
                        other.text(*name).to_string(),
                        fields.iter().map(|f| Ty::from_ast(other, &f.ty)).collect(),
                    ),
                    Item::Enum { name, variants, .. } => (
                        other.text(*name).to_string(),
                        variants
                            .iter()
                            .flat_map(|v| match &v.fields {
                                ast::VariantFields::Unit => Vec::new(),
                                ast::VariantFields::Tuple(types) => {
                                    types.iter().map(|ty| Ty::from_ast(other, ty)).collect()
                                }
                                ast::VariantFields::Named(fields) => {
                                    fields.iter().map(|f| Ty::from_ast(other, &f.ty)).collect()
                                }
                            })
                            .collect(),
                    ),
                    _ => continue,
                };
                beside_parts.entry(name).or_insert(parts);
            }
        }
        beside_parts
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
        let declared: Vec<String> = self
            .parsed
            .program
            .items
            .iter()
            .filter_map(|item| match &item.node {
                Item::Struct { name, .. } | Item::Enum { name, .. } => {
                    Some(self.parsed.text(*name).to_string())
                }
                _ => None,
            })
            .collect();
        // **And every type another file of the package declares**
        // (ADR-286 D1): whether `Count` from `ty.nika` copies is the same
        // answer in `sharing.nika`, and each file decides it alike because
        // each asks over the whole package. Asked only here; what this unit
        // derives `Copy` for is still its own (found moving the compiler's
        // `sharing` into Nikaia, #125).
        let beside: Vec<String> = self
            .others()
            .flat_map(|other| {
                other
                    .program
                    .items
                    .iter()
                    .filter_map(|item| match &item.node {
                        Item::Struct { name, .. } | Item::Enum { name, .. } => {
                            Some(other.text(*name).to_string())
                        }
                        _ => None,
                    })
            })
            .filter(|name| !declared.contains(name))
            .collect();
        let package: Vec<String> = declared.iter().chain(&beside).cloned().collect();
        // What each type beside is made of, read off its declaration: the
        // fields of a struct, every payload of an enum's variants.
        let beside_parts = self.parts_beside();
        // **A type the program writes a `Drop` for** is not one either, for
        // the reason a cleanup is not: Part I's `FileHandle { fd: i32 }` is a
        // struct of one number with `impl Drop`, and `rustc` refuses `Copy`
        // beside it (E0184).
        let dropped: BTreeSet<String> = std::iter::once(self.parsed)
            .chain(self.beside.iter().copied())
            .flat_map(|unit| {
                unit.program
                    .items
                    .iter()
                    .filter_map(|item| match &item.node {
                        Item::Impl {
                            trait_name: Some(name),
                            target,
                            ..
                        } if unit.text(*name) == "Drop" => Some(unit.text(target.name).to_string()),
                        _ => None,
                    })
            })
            .collect();
        // **A described Rust type that says it copies** is a part that copies
        // ([ADR-294](../../docs/specification/adr/adr-294.md) D9.3): `describe`
        // read its `#[derive(Copy)]`, and the line was reviewed. Nothing else
        // from a crate is.
        let mut copies: BTreeSet<String> = self
            .world
            .library
            .types
            .iter()
            .filter(|(_, contract)| contract.copies)
            .map(|(name, _)| name.clone())
            .collect();
        loop {
            let joining: Vec<String> = package
                .iter()
                .filter(|name| !copies.contains(*name))
                .filter(|name| {
                    let parts: Vec<Ty> = match (
                        declared.contains(*name),
                        self.s.enum_payloads.get(*name),
                        beside_parts.get(*name),
                    ) {
                        (true, Some(held), _) => held.clone(),
                        (true, None, _) => match self.s.structs.get(*name) {
                            Some(fields) => fields.iter().map(|f| f.ty.clone()).collect(),
                            // Neither a struct nor an enum this unit knows
                            // the parts of: nothing is claimed.
                            None => return false,
                        },
                        (false, _, Some(parts)) => parts.clone(),
                        (false, _, None) => return false,
                    };
                    // **Never a type with a cleanup** (ADR-297 D1): its
                    // cleanup is a `Drop` below, and a copied value would run
                    // it twice - `rustc` refuses the pair (E0184), which is
                    // how Part I's own `Connection` found this.
                    let named = Ty::Named {
                        name: (*name).clone(),
                        args: Vec::new(),
                        view: false,
                    };
                    !dropped.contains(*name)
                        && self.cleanup_of(&named).is_none()
                        && parts.iter().all(|ty| a_copied_part(ty, &copies))
                })
                .cloned()
                .collect();
            if joining.is_empty() {
                break;
            }
            copies.extend(joining);
        }
        // What the emitter derives `Copy` for: the program's own types. A
        // described one was a part here, and its derive is its crate's; one
        // beside is its own file's.
        self.s.copies_in_the_package = copies
            .iter()
            .filter(|name| package.contains(name))
            .cloned()
            .collect();
        self.s.facts.copies = copies
            .into_iter()
            .filter(|name| declared.contains(name))
            .collect();
    }

    /// **Which declared types compare**
    /// ([ADR-204](../../docs/specification/adr/adr-204.md) D1), for the derive
    /// the emitter writes and the refusal this file raises.
    ///
    /// After every type is collected, because the answer is structural: a
    /// `struct` compares when its fields do, and a field may name a type
    /// declared further down the file.
    fn collect_comparisons(&mut self) {
        // **And what another file of the package declares** (ADR-286 D1):
        // `Integer` from `integers.nika` compares in `build_values.nika` as it
        // does at home, and a type holding one derives `PartialEq` (found
        // moving the build-time values into Nikaia, #125).
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
        // **Only what this unit declares.** A library's type answers from its
        // own column, and nothing here writes a derive for it.
        let declared: Vec<String> = self
            .parsed
            .program
            .items
            .iter()
            .filter_map(|item| match &item.node {
                Item::Struct { name, .. } | Item::Enum { name, .. } => {
                    Some(self.parsed.text(*name).to_string())
                }
                _ => None,
            })
            .collect();
        let mut compared: Vec<(String, bool)> = Vec::new();
        for name in declared {
            let parts: Vec<Ty> = match self.s.enum_payloads.get(&name) {
                Some(held) => held.clone(),
                None => self
                    .s
                    .structs
                    .get(&name)
                    .map(|fields| fields.iter().map(|f| f.ty.clone()).collect())
                    .unwrap_or_default(),
            };
            if !parts.iter().all(|ty| asking.of(ty)) {
                continue;
            }
            let totally = parts.iter().all(|ty| asking.totally(ty));
            compared.push((name, totally));
        }
        for (name, totally) in compared {
            self.s.facts.compares.insert(name.clone());
            if totally {
                self.s.facts.compares_totally.insert(name);
            }
        }
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

    /// Whether a conversion narrows, recorded for the emitter (ADR-285 D12).
    ///
    /// **Only among the numeric types this compiler knows the range of**, for the
    /// reason `constant_fits` gives: a claim about a type neither the
    /// specification nor the ledger describes would be a claim about a surface
    /// that is not there. Anything this does not recognise is recorded as
    /// widening, which leaves the conversion exactly as it is today.
    ///
    /// The list is the other way round - which conversions **always** fit - and
    /// that is the fail-closed direction (ADR-010 D1): a pair nobody thought
    /// about is checked at run time rather than truncated in silence.
    ///
    /// **The machine-width types are not in it, and no longer need to be**
    /// ([ADR-285](../../../../docs/specification/adr/adr-285.md) D1). They were,
    /// because `len` handed one back and what fits on a large machine does not on
    /// a small one. A length is an `i64` now and `usize` has left the surface a
    /// program can write, so a conversion out of one is a conversion out of a type
    /// nobody can hold - and this list is the writable surface, which is the
    /// reason the entry was there and the reason it is not.
    ///
    /// An integer to `f64` is the one entry that loses something and is still
    /// called fitting (D7): digits go at large values without anything
    /// overflowing, there is no sensible point to stop at, and every language
    /// does it this way. That is a written limit and not a check.
    fn record_cast(&mut self, from: &Ty, into: &Ty, span: &Span) {
        self.s.record_cast(from, into, span);
    }

    /// **`NK1117` for a function nothing declares** (0.0.244).
    ///
    /// `frobnicate(1)` lowered as written and `rustc` said *cannot find
    /// function* about a file nobody wrote (Part III C.1). A call nothing
    /// describes is left alone where it is written through a path - a package
    /// or a crate may have it - but a **bare** name has nowhere else to come
    /// from: a `use` brings no name in (ADR-140 D5), so it is this program's,
    /// a value in scope, the prelude's or nothing. Everything this checker
    /// knows declares one is asked first, so what is left is certain (C.4).
    fn a_function_nothing_declares(&mut self, name: &str, span: &Span) {
        self.s
            .a_function_nothing_declares(&self.world.here, self.world, name, span)
    }

    /// **A lookup's key is lent** (0.0.235, issue #169).
    ///
    /// `m.get(k)`, `m.contains_key(k)` and a set's `contains(k)` and
    /// `remove(k)` take the key by reference in the language below, and a
    /// method's arguments are not lent by the ledger's column
    /// (`contracts::keeps::lends` says why: the declaration would be written
    /// off it, and a call the checker does not walk would miss). These
    /// declarations are Rust's and are never written off anything, so the
    /// reason does not reach them: the `&` is recorded here, for exactly these
    /// entries, where the key is a known value that is not already a view.
    /// `m.get(k)` over a map keyed by numbers was `rustc`'s *mismatched types*.
    fn a_lookup_key_lent(&mut self, key: &str, args: &[Expr], found: &[Ty]) {
        self.s.a_lookup_key_lent(self.world, key, args, found);
    }

    /// `NK1105` for text of its own given to a name bound to a literal
    /// (`tools/check_exprs.nika`).
    fn owned_text_into_a_literal_binding(
        &mut self,
        target: &Expr,
        found: &Ty,
        into: &Ty,
        span: &Span,
    ) -> bool {
        self.s
            .owned_text_into_a_literal_binding(self.world, target, found, into, span)
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

    /// **`assert(cond; message: …)`** ([ADR-269](../../docs/specification/adr/adr-269.md)
    /// D2, D3): one `bool`, one option, and a condition that changes nothing.
    ///
    /// Typed here and not through [`Checker::call`], because what the
    /// condition calls is the question: a frame of [`StoredCode`] around it
    /// collects every method it resolves, and its free calls are read off the
    /// tree. A condition that pauses, fails, touches anything or changes a
    /// value is `NK1194`: the rule that keeps a proof possible (D3), and the
    /// reason evaluating it a second time for the failure message is harmless.
    fn an_assert(&mut self, args: &[Expr], config: &[ast::ConfigArg], span: &Span) -> Ty {
        let unit = Ty::named("()");
        let [condition] = args else {
            self.s.an_assert_of_another_shape(
                format!(
                    "`assert` claims one condition, and this hands it {}",
                    args.len()
                ),
                span,
            );
            for arg in args {
                self.expr(arg, span);
            }
            return unit;
        };
        self.s.stored_frames.push(StoredCode::empty());
        let found = self.expr(condition, span);
        let frame = self.s.stored_frames.pop().unwrap_or_else(StoredCode::empty);
        self.expect_bool(&found, span, "An `assert` checks a `bool`.");
        // A number's type is settled below, so the check above cannot see
        // `assert(5)`; the literal is its own answer.
        if is_a_literal_other_than(condition, "bool") {
            self.s.an_assert_of_another_shape(
                "An `assert` checks a `bool`, but this is a literal of another type.".to_string(),
                span,
            );
        }
        let mut message = None;
        for option in config {
            let name = self.parsed.text(option.name).to_string();
            let found = self.expr(&option.value, span);
            if name != "message" {
                self.s.an_assert_of_another_shape(
                    format!(
                        "`assert` has no option called `{name}`. Its only option is `message:`."
                    ),
                    span,
                );
                continue;
            }
            let not_text = is_a_literal_other_than(&option.value, "text")
                || (!found.is_unknown()
                    && !found.fits(&Ty::named("String"))
                    && !found.fits(&Ty::view("str")));
            if not_text {
                self.s.an_assert_of_another_shape(
                    match found.is_unknown() {
                        true => "An `assert`'s `message:` has to be text, but this is a \
                                 literal of another type."
                            .to_string(),
                        false => format!(
                            "An `assert`'s `message:` has to be text, but this is `{found}`."
                        ),
                    },
                    span,
                );
            }
            message = Some(&option.value);
        }

        let mut not_pure: Vec<String> = Vec::new();
        if frame.unresolved {
            not_pure.push(
                "It calls a method the compiler knows nothing about, so it might change \
                 something."
                    .to_string(),
            );
        }
        for key in &frame.resolved {
            if let Some((_, contract)) = self.method(key)
                && let Some(why) = not_pure_because(&contract)
            {
                not_pure.push(format!("`{key}` {why}"));
            }
        }
        self.a_claim_reaches(condition, &mut not_pure);
        if !not_pure.is_empty() {
            self.s.findings.push(Finding {
                severity: Severity::Error,
                span: *span,
                code: "NK1194".to_string(),
                message: "An `assert` condition can't change anything.".to_string(),
                notes: not_pure
                    .into_iter()
                    .chain(std::iter::once(
                        "An `assert` states something that's true. Its condition may not pause, \
                         fail, touch the outside world or change a value."
                            .to_string(),
                    ))
                    .collect(),
                help: Some(
                    "Compute the value first with `let`, then `assert` something about it."
                        .to_string(),
                ),
                labels: Vec::new(),
            });
        }

        // **What the failure shows** (D2): the operands of a comparison, by
        // the names they were written with, where their value can be printed.
        let (left, right) = match condition {
            Expr::Binary {
                op:
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge,
                lhs,
                rhs,
                ..
            } => (self.shown_operand(lhs), self.shown_operand(rhs)),
            _ => (None, None),
        };
        let mut claimed = format!("{ASSERT}({}", written(self.parsed, condition));
        if let Some(value) = message {
            claimed.push_str(&format!("; message: {}", written(self.parsed, value)));
        }
        claimed.push(')');
        self.s.facts.claims.insert(
            ((span.at() as i64), value_node(condition)),
            Claim {
                held: None,
                written: claimed,
                left,
                right,
            },
        );
        unit
    }

    /// An operand whose value an `assert`'s failure can show: anything but a
    /// literal, which shows nothing it does not already say. Whether its type
    /// can be printed is the language below's to answer
    /// (`nikaia_std::abort::Operand`), because an integer literal's type is
    /// not settled here.
    fn shown_operand(&self, operand: &Expr) -> Option<String> {
        let literal = matches!(
            operand,
            Expr::LitInt { .. }
                | Expr::LitFloat(_, _)
                | Expr::LitStr { .. }
                | Expr::LitChar(_, _)
                | Expr::LitBool(_, _)
                | Expr::LitNull(_)
        ) || matches!(operand, Expr::Unary { op: UnaryOp::Neg, expr, .. }
            if matches!(**expr, Expr::LitInt { .. } | Expr::LitFloat(_, _)));
        (!literal).then(|| written(self.parsed, operand))
    }

    /// The free calls and the forms in a claim that are not a read
    /// ([ADR-269](../../docs/specification/adr/adr-269.md) D3). Methods are
    /// the frame's; this is what the tree says.
    fn a_claim_reaches(&mut self, condition: &Expr, not_pure: &mut Vec<String>) {
        self.nikaia(|state, world, host| state.a_claim_reaches(world, host, condition, not_pure))
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

    /// **`NK1181`: nothing declares the head of this path.**
    ///
    /// [`Checker::nothing_declares_it`] one segment down, and the same
    /// fail-safe direction: `nowhere::wobble` used to lower, and the language
    /// below answered *failed to resolve: use of unresolved module or unlinked
    /// crate* — a sentence with *crate* in it about a file nobody wrote
    /// ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)).
    ///
    /// **What a head may legally be**, asked in order and every one of them
    /// something this compiler has already read: a **type** declared here or
    /// recorded by either ledger, a **package** `use` brings in, a **package**
    /// the manifest declares, a **crate** a description covers, a `std`
    /// module, a **grammar**, an opaque handle, or a type parameter. A ledger
    /// that records *anything* under `head::` answers too, because a crate is
    /// known by its items rather than by a list of its names.
    ///
    /// **Anything this cannot see is a name it must not refuse**
    /// ([Part III C.4](../../docs/specification/30-nikaia-tooling.md)), which
    /// is why the list fails open rather than closed: a head nobody has told
    /// this compiler about is the one case, and every other reading of an
    /// absent key had to be ruled out first.
    fn a_head_nothing_declares(&mut self, head: &str, written: &str, span: &Span) {
        self.s
            .a_head_nothing_declares(&self.world.here, self.world, head, written, span)
    }

    /// **`x?.m(…)` over a `T?`** (Part I 3.5): the call happens only where
    /// there is something to call it on, and the result is flattened for
    /// [`Expr::SafeField`]'s reason - a method that hands back a `T?` would
    /// otherwise give a nullable of a nullable
    /// ([ADR-278](../../../docs/specification/adr/adr-278.md)). Asked from the
    /// method call's own walk once the receiver's type is known, so the
    /// receiver is walked once.
    #[allow(clippy::too_many_arguments)]
    fn a_safe_call(
        &mut self,
        inner: Box<Ty>,
        receiver: &Expr,
        method: &winnow_grammar::Symbol,
        args: &[Expr],
        config: &[ast::ConfigArg],
        witness: Option<usize>,
        span: &Span,
    ) -> Ty {
        let name = self.parsed.text(*method).to_string();
        // **The same call, on the value inside.** Everything a method
        // call is checked for - what it may throw, whether it pauses,
        // what its arguments have to be, what the receiver's own type
        // binds - is unchanged by the reach: a `?.` decides *whether*
        // the call happens and never *what* a call is. The door is one
        // of those things, so it is resolved here exactly as it is
        // there, off the type **inside** the `T?`.
        let door = witness
            .filter(|_| locked_content_of(&inner).is_some())
            .map(|at| &config[at].value);
        if let (Some(at), None) = (witness, door) {
            self.expr(&config[at].value, span);
        }
        let given: Vec<Expr>;
        let (args, written) = match door {
            Some(seen) => {
                self.s.the_witness_takes_no_reference(seen, span);
                self.s.facts.witnessed_sets.insert(span.at() as i64);
                given = args.iter().chain([seen]).cloned().collect();
                (&given[..], "set(after)".to_string())
            }
            None => (args, self.parsed.text(*method).to_string()),
        };
        // **The receiver is lent to the call where the call changes
        // nothing** ([ADR-278](../../docs/specification/adr/adr-278.md)
        // D2, [ADR-278](../../docs/specification/adr/adr-278.md) D16 and
        // D3). What comes out of a reached **method** is the call's own
        // result rather than a view of the receiver, so this half needs
        // no representation and is whole.
        //
        // Asked of the `mutates` column and only where **every**
        // candidate for the name agrees, which is the rule `NK1138`
        // uses one construct over: a name this compiler cannot resolve
        // is claimed nothing about, and the reach lowers exactly as it
        // did (Part III C.4).
        let candidates: Vec<_> = self
            .world
            .own
            .candidates(&name)
            .into_iter()
            .chain(self.world.library.candidates(&name))
            .collect();
        // **A map's read is lent already** (0.0.246): it answers a
        // view of the value the map holds, and lending that again made
        // each element a view of a view - `m[k]?.clone()` copied the
        // reference and not the list, and `rustc` refused the `??`
        // after it. The receiver of a `?.` that is an index can only be
        // a map's, since a list's element is not a `T?` (`NK1121`).
        // **Nor is a nullable view** (issue #171): an
        // `Option<&str>` is a copy already, and lending it made the
        // reached value a `&&str`, whose `.to_owned()` is the `&str`.
        let lent = !candidates.is_empty()
            && candidates.iter().all(|(_, c)| !c.mutates)
            && !matches!(receiver, Expr::Index { .. })
            && !inner.is_a_view();
        if lent {
            self.s.facts.lent_reaches.insert(value_node(receiver));
        }
        // **And a copy of a view is text of its own** (ADR-293 D24),
        // reached or not: `b?.clone()` for a `b: ref String?` is a
        // `String?`, which `.clone()` of the `&str` inside is not.
        if inner.is_a_view() && name == "clone" && args.is_empty() {
            self.s.facts.owned_copies.insert(value_node(receiver));
        }
        // **A field that holds a function is called as one** here too
        // (#533): `x?.apply(1)` is `(x.apply)(1)` on what is inside.
        let inside = self.s.inside_a_door;
        let reached = match self.a_called_field(&inner, receiver, *method, args, inside, span) {
            Some(result) => result,
            None => self.call_on(*inner, receiver, *method, args, &written, span),
        };
        // **A view out of a temporary needs something to point into**
        // (ADR-278 D21): the receiver is held for the rest of the block.
        let a_view = match &reached {
            Ty::Nullable(inner) => inner.is_a_view(),
            other => other.is_a_view(),
        };
        if lent && a_view && !roots_in_a_binding(receiver) {
            self.s.facts.held_reaches.insert(value_node(receiver));
        }
        match reached {
            // The `and_then` case, recorded by name for the emitter
            // exactly as a nullable field is (ADR-288: the emitter has
            // no types and this is a question about one).
            Ty::Nullable(result) => {
                self.s.facts.flattened_reaches.insert(value_node(receiver));
                Ty::Nullable(result)
            }
            Ty::Unknown => Ty::Unknown,
            plain => Ty::Nullable(Box::new(plain)),
        }
    }

    /// A field read through `?.` (`tools/check_fields.nika`).
    fn a_safe_field(
        &mut self,
        inner: &Ty,
        base: &Expr,
        field: String,
        expr: &Expr,
        span: &Span,
    ) -> Ty {
        self.s
            .a_safe_field(self.world, inner, base, &field, expr, span)
    }

    /// `?.` over a value that is never absent (`tools/check_fields.nika`).
    fn reaches_through_a_plain_value(
        &mut self,
        on: &Ty,
        receiver: &Expr,
        member: Reached<'_>,
        reach: &Expr,
        span: &Span,
    ) {
        let (name, method) = match member {
            Reached::Field(name) => (name, false),
            Reached::Method(name) => (name, true),
        };
        self.s
            .reaches_through_a_plain_value(self.world, on, receiver, name, method, reach, span);
    }

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

    /// The entry call itself: the input is an expression like any other, and
    /// what comes back is what the rule declares.
    ///
    /// **Whether it can fail comes from the contract**, which is the whole of
    /// what D2 changed: a `entry` rule is an entry in the ledger with
    /// `throws = ["?"]`, because a rule past a commit point can fail
    /// ([ADR-023](../../docs/specification/adr/adr-023.md) D9). The old form
    /// was not a call and carried no contract, so `NK1134` had to be told in
    /// one line ([ADR-308](../../docs/specification/adr/adr-308.md) D9); that
    /// line is gone with it.
    fn grammar_call(&mut self, key: &str, args: &[Expr], span: &Span) -> Ty {
        self.nikaia(|state, world, host| {
            check_core_pilot::check_core_grammar_call(state, world, host, key, args, span)
        })
    }

    /// **The compiler writes the reference at the call, and a written one is
    /// refused** ([ADR-094](../../docs/specification/adr/adr-094.md) D1).
    ///
    /// The callee's answer is `contracts::keeps::lends`, read here and read
    /// again by the emitter when it writes that callee's *declaration* — one
    /// answer in two positions, because the two disagreeing is a `&&T` or a
    /// moved value in the language below. What this adds is the half the
    /// emitter cannot see: whether the argument is **already** a view, which is
    /// a question about its type (ADR-288).
    ///
    /// **`NK1137` lands exactly where the compiler would have written one**,
    /// and nowhere else. A `&` in a position the callee keeps, or one in front
    /// of a value whose type this checker could not work out, is still the
    /// program's own and is left alone — refusing it would take away the only
    /// way to say what the line means before the inference that replaces it can
    /// answer.
    fn the_compiler_writes_the_reference(
        &mut self,
        contract: &FnContract,
        argument: Argument<'_>,
        written: &str,
        span: &Span,
    ) -> bool {
        let Argument {
            at,
            given,
            found,
            want,
            own,
        } = argument;
        // `arguments()` drops the receiver, and the contract's positions do
        // not, so a method's argument sits one further along.
        let position = at
            + usize::from(
                contract
                    .signature
                    .as_ref()
                    .is_some_and(|s| s.takes_a_receiver()),
            );
        // **`mut` is asked first, and it is a declaration** (D3): the
        // parameter lowers to `&mut T` and the argument gains a `&mut`, both
        // off the word the author wrote. `lends` withholds its claim on such a
        // position, so exactly one of the two answers.
        let changes = contract
            .signature
            .as_ref()
            .and_then(|s| s.params.get(position).map(|(name, _)| (s, name)))
            .is_some_and(|(s, name)| s.mutable.iter().any(|m| m == name));
        if changes {
            let Some(given) = given else {
                return false;
            };
            // **The `&mut` is the compiler's, the type is still the
            // argument's.** This answered *it fits* for whatever stood there,
            // so a `Walked` handed to a `mut found: Vec[Seen]` was accepted
            // and `rustc` refused the file (found moving the walk's names
            // into Nikaia, #125). An argument that is not the parameter's type
            // falls through to `NK1102` as it does at any other position.
            if !(found.fits(want) || self.fits_through_deref(found, want)) {
                return false;
            }
            // **A reference that is already there is not written twice.** Where
            // the argument is a `mut` parameter of the function this call stands
            // in, the binding *is* a `&mut T`, so the call hands it straight on
            // and the language below reborrows it. Writing the `&mut` anyway is
            // not a `&mut &mut T` that works by deref: a `&mut` may only be
            // taken of a binding that is itself `mut`, and a parameter is not
            // one, so `rustc` refuses it — *cannot borrow `connection` as
            // mutable, as it is not declared as mutable* — about a file nobody
            // wrote ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)).
            //
            // `true` and not `false`: nothing is left for the positions below to
            // decide. This **is** the fit — the argument already has the shape
            // the parameter asked for — and falling through would measure a
            // `Connection` against a `Connection` and then look for a wrap.
            //
            // Only a bare name. `&mut c.field` for a `mut c` is the reference
            // the call wants, and `rooted_at` would have said `c` for it.
            if matches!(given, Expr::Variable(name, _)
                if self.s.binding(self.parsed.text(*name)).is_some_and(|l| l.changing))
            {
                return true;
            }
            self.s.facts.mut_args.insert(value_node(given));
            return true;
        }
        let copying = [self.world.library];
        let lends = match own {
            true => crate::contracts::keeps::lends_in_own(contract, position, &copying),
            false => crate::contracts::keeps::lends_in(contract, position, &copying),
        };
        if !lends {
            return false;
        }
        let Some(given) = given else {
            return false;
        };
        // **A lambda written in place for a `std` entry is handed over, not
        // lent**: the entry's counterpart takes an `impl FnOnce` by value, and
        // the language below infers the closure's parameter types only from a
        // closure passed as it is - `task::scope(&|s| …)` left `s` untyped.
        if matches!(given, Expr::Closure { .. })
            && !self.world.own.functions.contains_key(written)
            && !self
                .world
                .own
                .functions
                .contains_key(&format!("{written}::new"))
            && self.world.library.functions.contains_key(written)
        {
            return false;
        }
        let wrote_a_reference = matches!(
            given,
            Expr::Unary {
                op: crate::ast::UnaryOp::Ref,
                ..
            }
        );
        // **Only where the call would be right with the reference in it.** A
        // `&i64` handed to a `&Request` is `NK1102` and stays one: saying *the
        // `&` here is the compiler's* about an argument that is the wrong value
        // would send the reader to fix the punctuation of a line whose type is
        // wrong.
        //
        // The hull's path counts as a fit: a `Shared[Conn]` reaches a `&Conn`
        // through its deref ([ADR-312](../../docs/specification/adr/adr-312.md)
        // D5), and that fit was the *source's* `&` before this.
        //
        // **Which fit to ask depends on which of the two kinds this is.** A
        // parameter written `&str` is a view in the declaration already, so
        // what has to fit it is the argument *with* the reference the compiler
        // writes — a `String` becomes a `&String` and reaches `&str` the way
        // the language below reaches it. A parameter written `Stats` and
        // inferred lent gains its `&` in the declaration too, so both sides
        // move together and the fit is the one it always was.
        let fits = match want.is_a_view() {
            true => {
                // **`view_of` and not a `&` pasted on the front**, because
                // a view of a `String` is a `&str` — the one rewrite Part I 6.5
                // makes, and the same one this checker performs for a `&`
                // somebody writes. Keeping the name refused `count(dna)` for a
                // `dna` every caller had been writing `count(&dna)` for.
                let lent = view_of(found);
                lent.fits(want) || self.fits_through_deref(&lent, want)
            }
            false => {
                found.fits(want)
                    || self.fits_through_deref(found, want)
                    // **A `&` the source wrote is already the view**, and this
                    // parameter is not one: what has to fit `want` is what the
                    // `&` was put in front of, because that is the expression
                    // the compiler would have written its own `&` before. Asked
                    // the other way, `width(&t)` for a `text: String` that
                    // `width` only reads came back `&str` against `String` and
                    // was answered with `NK1102`'s *write `.to_string()`* — a
                    // help that sends the reader the wrong way about a line
                    // whose only fault is a character the compiler writes.
                    || (wrote_a_reference
                        && viewed_from(found).iter().any(|owned| {
                            owned.fits(want) || self.fits_through_deref(owned, want)
                        }))
            }
        };
        if !fits {
            return false;
        }
        if wrote_a_reference {
            // **A type this compiler could not work out is not one to refuse
            // punctuation over.** `Ty::Unknown` fits everything, which is the
            // right answer for an absent claim
            // ([ADR-024](../../docs/specification/adr/adr-024.md) D1) and no
            // ground to stand on here: `route(&n)` for an `n` whose type was
            // never pinned would be refused on the strength of a fit that means
            // *nothing was checked*, which is [Part III
            // C.4](../../docs/specification/30-nikaia-tooling.md)'s correct
            // program refused. Left alone, the `&` the source wrote is the same
            // character the compiler would have written, so the lowering is the
            // one either way.
            //
            // **Which is why the test sits here and not above the fit.** The
            // declaration is written off `lends` alone, so anything this
            // refuses to record is a call whose callee takes a `&T` and whose
            // argument does not have one - `rustc` about a file nobody wrote.
            // A condition that skips the *recording* may only be one the
            // emitter shares; this one skips the *refusal*, and the source's
            // own `&` stands in for what would have been recorded.
            if matches!(found, Ty::Unknown) {
                return false;
            }
            // **A `&` at a method call is left alone** (Part III's `NK1137`):
            // it is the reference the compiler would have written, and the
            // source's own stands in for it.
            if contract
                .signature
                .as_ref()
                .is_some_and(|s| s.takes_a_receiver())
            {
                return true;
            }
            self.s.findings.push(Finding {
                severity: Severity::Error,
                span: *span,
                code: "NK1137".to_string(),
                message: "You don't need the `ref` here: the compiler lends where it's needed."
                    .to_string(),
                notes: vec![format!(
                    "`{written}` only reads this argument, so it's lent already. Whether an \
                     argument is lent or handed over is decided by the function, not the call."
                )],
                help: Some("Remove the `ref`.".to_string()),
                labels: Vec::new(),
            });
            return true;
        }
        // **A view of a nullable handed to a nullable view** is recorded, and
        // no `&` is what the emitter writes for it: a part a `match` was lent
        // (`Local::lent`) of type `T?` is an `&Option<T>` below, and a
        // `ref T?` parameter an `Option<&T>`. The emitter opens the option
        // (`.as_deref()`, `.as_ref()`), which is what it writes where it lends
        // a `T?` of its own.
        let a_lent_nullable = matches!(found, Ty::Nullable(_))
            && matches!(want, Ty::Nullable(inner) if inner.is_a_view())
            && matches!(given, Expr::Variable(name, _)
                if self.s.binding(self.parsed.text(*name)).is_some_and(|local| local.lent));
        // A value that is already a view needs nothing: the declaration and the
        // argument agree without a second `&`.
        if found.is_a_view() && !a_lent_nullable {
            return false;
        }
        // **Nor a name that is a view below whatever its type says**: a `let`
        // bound to a map read beside a jump (`let at = m[k] ?? continue`) is
        // a `&String` already, and a second `&` was a `&&String` that
        // `contains_key` has no `Borrow` for (found moving `describe`'s draft
        // into Nikaia, #125). A number read that way is the emitter's `*`.
        if !matches!(found, Ty::Nullable(_))
            && crate::contracts::keeps::moves(found)
            && matches!(given, Expr::Variable(name, _)
                if self.s.binding(self.parsed.text(*name)).is_some_and(|local| local.lent))
        {
            return true;
        }
        // **Nor does a function this function was lent**: a code parameter
        // the body only calls is an `&impl Fn` below already, and a second `&`
        // on a call that hands it on is a new type at every level of a
        // recursion - `rustc`'s *overflow evaluating the requirement*, found
        // moving `fold` into Nikaia (ADR-294 D11).
        if matches!(found, Ty::Fn { .. }) && self.a_lent_parameter(given) {
            return false;
        }
        // **A `??` handed to a position that only reads it lends its left
        // side** (ADR-279 D5): the answer is a view already, of the name or of
        // the fallback, and needs no `&` of its own.
        if self.a_pending_coalesce_is_lent(given) {
            return true;
        }
        // **A plain value lent to a `ref T?` is put in the option too** (Part
        // I 2.3's wrap at an argument): `name_of(Kind::B)` for a `k: ref
        // Kind?` is `Some(&Kind::B)`. Settled here, the wrap below was never
        // recorded, and the emitter opened the value as if it were an option
        // already - `(Kind::B).as_ref()` (found moving `sync`'s walk into
        // Nikaia, #125).
        if matches!(want, Ty::Nullable(inner) if inner.is_a_view())
            && !matches!(found, Ty::Nullable(_))
            && !found.is_unknown()
        {
            self.s
                .facts
                .nullable_args
                .insert(value_node(given), Wrap::Constructor);
        }
        self.s.facts.lent_args.insert(value_node(given));
        true
    }

    /// The name a free call names, unaliased — or nothing where this is not one.
    fn free_callee(&self, func: &Expr) -> Option<String> {
        let name = match func {
            Expr::Variable(name, _) => self.parsed.text(*name).to_string(),
            Expr::Path(segments, _) => segments
                .iter()
                .map(|s| self.parsed.text(*s))
                .collect::<Vec<_>>()
                .join("::"),
            _ => return None,
        };
        Some(self.parsed.unaliased(&name))
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
        let Some(current) = self.s.current.clone() else {
            return;
        };
        // **Whether the function's settle point can fail it**: only where a
        // cleanup can fail and the function says `throws`. Its channel is
        // otherwise whatever its body raises, and the settle point hands the
        // body's result back untouched.
        let failing = fails && self.s.flow.throwing;
        *self.s.facts.settle_fns.entry(current).or_insert(false) |= failing;
        let key = match failing {
            true => "cleanup::settle_failing",
            false => "cleanup::settle",
        };
        self.reached_method(Some(key));
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
        let is = match ty.unseen() {
            Ty::Named { name, .. }
                if crate::contracts::ty::base(&name) == crate::contracts::ty::base(&owner) =>
            {
                "is"
            }
            _ => "holds",
        };
        if fails && !self.s.flow.throwing && !self.s.flow.caught && self.s.current.is_some() {
            self.s.findings.push(Finding {
                severity: Severity::Error,
                span: *span,
                code: "NK2605".to_string(),
                message: format!("This function can fail, because cleaning up `{name}` can fail."),
                notes: vec![format!(
                    "`{name}` {is} a `{owner}`. Its cleanup runs at the end of this block \
                     and can fail, so the function can fail with it."
                )],
                help: Some(match is {
                    "is" => format!(
                        "Declare the function with `throws`, or close it yourself where you \
                         want to handle the failure: `{name}.close() catch {{ … }}`."
                    ),
                    _ => format!(
                        "Declare the function with `throws`, or close the `{owner}` it holds \
                         yourself, with `close() catch {{ … }}`."
                    ),
                }),
                labels: Vec::new(),
            });
        }
        Some(fails)
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
        let Some(ty) = [left, right].into_iter().find(|ty| !asking.of(ty)) else {
            return;
        };
        let ty = ty.text();
        self.s.findings.push(Finding {
            severity: Severity::Error,
            span: *span,
            code: "NK1188".to_string(),
            message: format!("You can't compare two `{ty}` values with `==`."),
            notes: vec![
                "A `struct` or an `enum` can be compared when all of its parts can. Locks, \
                 mapped files, sockets and task handles can't be compared."
                    .to_string(),
            ],
            help: Some(
                "Compare the parts that matter, or give the type a method that says what \
                 being equal means for it."
                    .to_string(),
            ),
            labels: Vec::new(),
        });
    }

    fn a_field_of_a_borrowed_subject(&mut self, value: &Expr, span: &Span, what: &str) {
        if !self.s.borrowing_self {
            return;
        }
        let Expr::Field { base, name, .. } = value else {
            return;
        };
        let Expr::Variable(subject, _) = base.as_ref() else {
            return;
        };
        if self.parsed.text(*subject) != "self" {
            return;
        }
        let field = self.parsed.text(*name).to_string();
        let ty = self.expr(value, span);
        if ty.is_unknown() || copies(&ty) {
            return;
        }
        self.s.findings.push(Finding {
            severity: Severity::Error,
            span: *span,
            code: "NK1131".to_string(),
            message: format!(
                "You can't give away `self.{field}` here: this method only borrows `self`."
            ),
            notes: vec![format!(
                "`ref self` borrows the value, and `{field}` is a `{}`. Giving it away \
                 would take a piece out of something this method doesn't own.",
                ty.text()
            )],
            // **[ADR-083](../../docs/specification/adr/adr-083.md) D2's two ways
            // out — and, where this is a `return`, the third that is now real.**
            //
            // The third stood here once and could not be taken: it said *declare
            // the result `&str` and write `return &self.field`*, and `&str`
            // stopped being a spelling at
            // [ADR-184](../../docs/specification/adr/adr-184.md) D4 while a `&` a
            // program writes is `NK1137` since
            // [ADR-094](../../docs/specification/adr/adr-094.md) D1. What it was
            // reaching for is built now: **declaring the result a view is
            // enough**, and the `&` is the compiler's
            // ([`Checked::lent_returns`], D1's third position). So the sentence
            // is back, with the half the author writes and without the half the
            // author cannot.
            //
            // **Only where the value is handed back.** A field *bound* to a name
            // or *passed* to a call is not a result, so a view of it has nowhere
            // declared to point — the two ways out are the whole answer there.
            help: Some(match what {
                "handed back" => format!(
                    "Declare the result `ref {}` to return a view of it, write \
                     `self.{field}.clone()` for a copy, or take `self` instead of \
                     `ref self` if the method is meant to use up its value.",
                    ty.text()
                ),
                _ => format!(
                    "Write `self.{field}.clone()` for a copy, or take `self` instead of \
                     `ref self` if the method is meant to use up its value."
                ),
            }),
            labels: Vec::new(),
        });
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
        if self.s.expected.as_ref().is_some_and(Ty::is_a_view) {
            return;
        }
        let Expr::Field { .. } = value else {
            return;
        };
        let Some(path) = self.s.place_path(self.world, value) else {
            return;
        };
        let Some((root, _)) = path.split_once('.') else {
            return;
        };
        if root == "self" || !self.s.lent_here(root) {
            return;
        }
        let ty = self.expr(value, span);
        if ty.is_unknown() || copies(&ty) {
            return;
        }
        let root = root.to_string();
        self.s.findings.push(Finding {
            severity: Severity::Error,
            span: *span,
            code: "NK1131".to_string(),
            message: format!("You can't give away `{path}` here: `{root}` is only borrowed."),
            notes: vec![format!(
                "`{root}` belongs to whoever lent it (a `ref` parameter, a `for` over a list, \
                 a `let` over a place), and `{path}` is a `{}`. Giving it away would take a \
                 piece out of something this function doesn't own.",
                ty.text()
            )],
            help: Some(format!(
                "Declare the result `ref {}` to return a view of it, or write `{path}.clone()` \
                 for a copy.",
                ty.text()
            )),
            labels: Vec::new(),
        });
    }

    /// **A `mut` parameter given away whole** (issue #161, Part I
    /// 6.6): it is passed as `ref mut`, so the function borrows it as a method
    /// borrows `ref self`, and giving it away is [`Self::a_field_of_a_borrowed_subject`]'s
    /// shape one position over. `store(mut out: Vec[String])` then
    /// `Box { items: out }` was `rustc`'s *mismatched types* about a file
    /// nobody wrote.
    fn a_mut_parameter_given_away(&mut self, value: &Expr, span: &Span, what: &str) {
        self.nikaia(|state, world, host| {
            check_core_pilot::check_core_a_mut_parameter_given_away(
                state, world, host, value, span, what,
            )
        })
    }

    /// **What a method call is**, asked of a receiver whose type is already in
    /// hand.
    ///
    /// One function and not two, because there are two ways to write the call
    /// and only one thing a call *is*: `x.m()` and `x?.m()` differ in whether
    /// the call happens, never in what it throws, whether it pauses, what its
    /// arguments have to be, or what the receiver's type binds in its signature
    /// ([ADR-278](../../../docs/specification/adr/adr-278.md)). Two copies of
    /// these rules would be that many chances for the two spellings to drift.
    ///
    /// `on` is the receiver's type - for a `?.` the type **inside** the `T?`,
    /// which is the whole of what that operator changes here.
    ///
    /// `entry` is the name the **ledger** knows this call by, which is the
    /// written one everywhere but at
    /// [ADR-281](../../docs/specification/adr/adr-281.md) D26's witness door:
    /// `kasse.set(neu; after: stand)` is `set(after)`, a different operation
    /// from `set` and a different entry. `method` stays the name the *program*
    /// wrote, and everything the **emitter** is handed stays keyed by that —
    /// an argument it has to put a `&` in front of is found under `set`,
    /// because `set` is what is written on the line.
    fn call_on(
        &mut self,
        on: Ty,
        receiver: &Expr,
        method: Ident,
        args: &[Expr],
        entry: &str,
        span: &Span,
    ) -> Ty {
        // **`full()` is `Error`'s own, on every type that implements it**
        // (Part I 7.1, #497): the `impl` writes `message` and the trait
        // provides the long form, which no entry of the type's names.
        if entry == "full"
            && args.is_empty()
            && let Ty::Named { name, .. } = &on
            && self.implements_error(name)
        {
            self.reached_method(None);
            self.method_pauses(method, false, span);
            self.method_propagates(method, false, span);
            return Ty::named("String");
        }
        // **An error, one of several, answers `Error`'s methods** (ADR-280
        // D30); what only one member has is reached through a `match`.
        if let Some(members) = several_errors(&on) {
            args.iter().for_each(|a| {
                self.expr(a, span);
            });
            self.reached_method(None);
            self.method_pauses(method, false, span);
            self.method_propagates(method, false, span);
            if matches!(entry, "message" | "full") && args.is_empty() {
                return Ty::named("String");
            }
            let members = members_named(members);
            self.s.findings.push(Finding {
                severity: Severity::Error,
                span: *span,
                code: "NK1171".to_string(),
                message: format!(
                    "This is an error, one of {members}, and an error has no method called `{entry}`."
                ),
                notes: vec![
                    "What every error has is `message()` and `full()`; what one of them has \
                     is reached once a `match` has said which it is."
                        .to_string(),
                ],
                help: Some(format!(
                    "Take it apart first: `match error {{ … }}`, and call `{entry}` in the arm \
                     of the type that has it."
                )),
                labels: Vec::new(),
            });
            return Ty::Unknown;
        }
        // **A produced sequence answers under its own word**
        // ([ADR-105](../../docs/specification/adr/adr-105.md) D1): `Seq` and
        // `Par` are what the ledger calls the receiver, so `Seq::collect` is
        // found exactly as `Vec::push` is. A `Par[T]` falls back to `Seq`'s
        // entries where it has none of its own, which is D3's *otherwise
        // `Par[T]` has `Seq[T]`'s surface* - one sentence rather than a second
        // copy of twelve entries.
        let named;
        let name = match &on {
            Ty::Seq { parallel, .. } => {
                named = match parallel {
                    true => ty::PAR,
                    false => ty::SEQ,
                };
                Some(named)
            }
            Ty::Named { name, .. } => Some(name.as_str()),
            // **A run is an `Array` whose length the caller knows**
            // ([ADR-184](../../docs/specification/adr/adr-184.md) D3), and
            // `std` writes its surface under `Array`: `xs.len()` for an
            // `xs: ref Array[R]` resolved to nothing, so the function around
            // it was inferred to pause - found moving `contracts::trust` into
            // Nikaia.
            Ty::Pointed { slice: true, .. } => Some(ty::ARRAY),
            _ => None,
        };
        // **A copy of a number, a `bool`, a `char` or a tuple is the derived
        // one** too, for the reason a declared record's is below: no ledger
        // describes it because nothing has to. Answered as *nothing describes
        // this method*, `at.clone()` on an `i64` and `id.clone()` on an
        // `(i64, String)` made every function around them `async`, found
        // moving `contracts::keep`'s walk into Nikaia (ADR-294).
        if entry == "clone" && args.is_empty() {
            match &on {
                Ty::Tuple(_) => return on.clone(),
                Ty::Named { name, args, .. }
                    if args.is_empty()
                        && (is_number(name)
                            || name == "bool"
                            || name == "char"
                            || name == "scalar") =>
                {
                    return Ty::Named {
                        name: name.clone(),
                        args: Vec::new(),
                        view: false,
                    };
                }
                _ => {}
            }
        }
        let Some(name) = name else {
            // The receiver's type is not known, so neither is what this
            // calls. Recorded, because "I could not find out" is an
            // answer somebody downstream has to act on.
            args.iter().for_each(|a| {
                self.expr(a, span);
            });
            self.reached_method(None);
            self.method_propagates(method, false, span);
            self.method_pauses(method, false, span);
            return Ty::Unknown;
        };
        let key = format!("{name}::{entry}");
        let found = self.method(&key).or_else(|| match &on {
            // D3's *otherwise `Par[T]` has `Seq[T]`'s surface*.
            Ty::Seq { parallel: true, .. } => self.method(&format!("{}::{entry}", ty::SEQ)),
            // **Text of its own lends itself as a view**
            // ([ADR-282](../../docs/specification/adr/adr-282.md) D18), so a
            // method of a view is a method of a `String` too - which is how
            // the language below has it.
            Ty::Named { name, args, .. } if name == "String" && args.is_empty() => {
                self.method(&format!("str::{entry}"))
            }
            // **A list's own `map` is `std`'s `ListExt`** (ADR-233 D2): the
            // trait is what the language below calls on a `Vec`, and the entry
            // is written under its name.
            Ty::Named { name, .. } if ty::base(name) == "Vec" => {
                self.method(&format!("list::ListExt::{entry}"))
            }
            // **A value with a cleanup closes under std's one `close`**
            // ([ADR-297](../../docs/specification/adr/adr-297.md) D4): it
            // runs the cleanup now and hands its failure back at the call.
            Ty::Named { name, .. }
                if entry == "close"
                    && self.cleanup_of(&on).is_some_and(|(owner, _)| {
                        crate::contracts::ty::base(&owner) == crate::contracts::ty::base(name)
                    }) =>
            {
                self.method("cleanup::close")
            }
            _ => None,
        });
        // **A copy of a type this program declares is the derived one**: it
        // does not pause, cannot fail, and hands back the type itself, owned.
        // No ledger describes it because nothing has to - and answered as
        // *nothing describes this method*, `r.clone()` on a declared `struct`
        // made the function around it `async`, found moving the ledger's
        // records into Nikaia (ADR-294), where `Signature::text` became a
        // future.
        if found.is_none()
            && entry == "clone"
            && args.is_empty()
            && (self.s.structs.contains_key(name)
                || self.s.enums.contains_key(name)
                // ...or another unit of the package declares, which is where
                // a tool module's records stand (`ledger.nika` beside
                // `ty.nika`).
                || self.world.own.types.contains_key(name))
        {
            return match &on {
                Ty::Named { name, args, .. } => Ty::Named {
                    name: name.clone(),
                    args: args.clone(),
                    view: false,
                },
                other => other.clone(),
            };
        }
        let Some((key, contract)) = found else {
            // The type is known and no ledger describes this method of
            // it - `HashMap::entry` until something writes it down.
            // **A bound is looked up before anything is refused**
            // ([ADR-295](../../docs/specification/adr/adr-295.md) D8): where
            // `T: Summarize` and the trait declares this method, the call is
            // asked again on the trait and everything about it - arity,
            // argument types, `sync`, `throws` - is answered from the
            // declaration, exactly as it would be from a written-down receiver.
            if let Some(bound) = self.s.bound_that_answers(self.world, name, entry) {
                return self.call_on(bound, receiver, method, args, entry, span);
            }
            // **A `Seen[T]` is the `T` it stamps** (ADR-281 D26, #497): the
            // stamp is erased below, so `seen.len()` is the list's `len`.
            if let Ty::Named {
                name, args: held, ..
            } = &on
                && name == "Seen"
                && let [held] = held.as_slice()
            {
                return self.call_on(held.clone(), receiver, method, args, entry, span);
            }
            // **And a type seen through answers with what it holds** (#497),
            // as a field read on one does: `data.lines()` on an `fs::Mapped`
            // is the text's.
            if let Some(held) = self
                .seen_through(&on)
                .filter(|held| held.unseen() != on.unseen())
            {
                return self.call_on(held, receiver, method, args, entry, span);
            }
            // Unless the type is a **parameter**, where nothing will ever
            // describe it and saying so now is the whole of `NK1126`.
            self.s
                .nothing_says_what_a_parameter_can_do(name, "method", entry, span);
            self.s.no_number_has_this_method(name, entry, span);
            self.s.a_list_collected(name, entry, args, span);
            self.a_member_nothing_writes_down(name, entry, true, span);
            args.iter().for_each(|a| {
                self.expr(a, span);
            });
            self.reached_method(None);
            self.method_pauses(method, false, span);
            self.method_propagates(method, false, span);
            return Ty::Unknown;
        };
        // **`to_text` on a name made from text cannot fail** (ADR-319 D5):
        // it reaches the entry that throws nothing, and the emitter writes it.
        let known_text =
            key == "fs::Path::to_text" && std::mem::take(&mut self.s.flow.receiver_from_text);
        if known_text {
            self.s.facts.path_methods.insert(value_node(receiver), true);
        }
        self.reached_method(Some(match known_text {
            true => "fs::Path::text_of",
            false => &key,
        }));
        self.s.flow.last_resolved = Some(key.clone());
        // **A receiver the method changes or takes is not only read**: a
        // `match` arm that does this to a part it bound needs the part itself.
        if let Some(name) = self.s.flow.receiver_name.clone() {
            let takes = contract
                .signature
                .as_ref()
                .and_then(|s| s.params.first())
                .is_some_and(|(_, ty)| {
                    matches!(ty, Ty::Seq { .. }) || self.s.takes_away(self.world, ty)
                });
            if contract.mutates || takes {
                self.s.changed.push(name);
            }
        }
        // **A method that walks a produced sequence consumes it**
        // ([ADR-105](../../docs/specification/adr/adr-105.md) D2): every `Seq`
        // entry writes its receiver `(Seq[$T], …)` and not `&Seq[$T]`, so the
        // signature is what says so rather than a list of method names. A
        // container's methods take a view and are untouched.
        if matches!(&on, Ty::Seq { shape, .. } if !shape.replays)
            && walks_by_value(&contract)
            && let Some(name) = self.s.flow.receiver_name.clone()
        {
            let taken = self.s.taken(name, on.clone(), self.s.read_seq, "", span);
            self.s.walked.push(taken);
        }
        // **And a list's own `map` takes the list** (ADR-233 D2): the mapped
        // list replaces it, so a read of it afterwards is one of a list that
        // is gone - `NK2105`, where it was `rustc`'s *borrow of moved value*.
        if key == "list::ListExt::map"
            && let Some(name) = self.s.flow.receiver_name.clone()
        {
            self.s
                .hands_over_name(self.world, &name, "taken by `map`", span);
        }
        // **And `close` takes the value**: nothing of it is left to clean up
        // afterwards, and nothing of it to read.
        if key == "cleanup::close"
            && let Some(name) = self.s.flow.receiver_name.clone()
        {
            self.s
                .hands_over_name(self.world, &name, "closed by `close`", span);
        }
        // ADR-023 D8: the failure leaves at the call, and the emitter
        // is what writes that. Recorded whether or not the function
        // around it declares `throws` - where it does not, `NK2605`
        // below refuses the program and nothing is emitted at all.
        self.method_options(method, &contract, span);
        self.method_propagates(
            method,
            !known_text
                && (!contract.fails_with.is_empty()
                    || self.walks_a_failing_sequence(&on, &contract)),
            span,
        );
        // ADR-055 D2, the method half. Either ledger since §6 step 3
        // made `std`'s own pausing entries `async fn`: before it, a
        // `std` entry blocked its thread and awaiting one would have
        // been awaiting a value rather than a future.
        // **A walk of a sequence whose step pauses, pauses**
        // ([ADR-172](../../docs/specification/adr/adr-172.md) D5). The entry's
        // own `sync` is about the walk — `count` adds one per element and does
        // nothing else — and what suspends is the **step** it asks for. So the
        // receiver's word decides it at the site, which is the same word the
        // `for` one construct over reads.
        //
        // **Except a `map` or a `filter` of a sequence this compiler made
        // pause** ([ADR-233](../../docs/specification/adr/adr-233.md) D1): that
        // one has both, as adapters of its own, and hands back another like it.
        let chained = a_paused_chain(&key, &on);
        let walks_a_pausing_step =
            matches!(&on, Ty::Seq { pauses: true, .. }) && walks_by_value(&contract) && !chained;
        self.method_pauses(
            method,
            !contract.sync_claim.is_sync() || walks_a_pausing_step,
            span,
        );
        // …and a **lazy** walk of one is another such sequence, as an adapter
        // of its own ([ADR-233](../../docs/specification/adr/adr-233.md) D1,
        // [ADR-234](../../docs/specification/adr/adr-234.md) D1) - every one
        // `Seq` has, which is what `chained` says. What is left here is an entry
        // that hands back a sequence and has no pausing form: none today, since
        // `rev` is `NK2703` first. The net stays for the next one.
        //
        // Recorded rather than refused here, because *this compiler cannot
        // build that yet* is a refusal from the lowering and not a rule of the
        // language ([ADR-171](../../docs/specification/adr/adr-171.md) §4).
        let hands_back_a_sequence = matches!(
            contract.signature.as_ref().and_then(|s| s.result.as_ref()),
            Some(Ty::Seq { .. })
        );
        if walks_a_pausing_step && hands_back_a_sequence {
            self.s
                .facts
                .pausing_walks
                .insert(((span.at() as i64), self.parsed.text(method).to_string()));
        }
        // **A walk of a sequence whose step can fail, can fail**
        // ([ADR-025](../../docs/specification/adr/adr-025.md) D1, one construct
        // over from the `for` it was written about). The entry's own `throws`
        // is about the walk; what fails is the **step** it asks for, so the
        // receiver's word decides it at the site.
        //
        // **The eager walks and not the lazy ones**, for the reason above read
        // the other way: a `map` has produced nothing, so nothing of it has
        // failed yet, and the failure belongs to whatever walks the result.
        let walks_a_failing_step = matches!(
            &on,
            Ty::Seq {
                can_throw: true,
                ..
            }
        ) && walks_by_value(&contract)
            && !hands_back_a_sequence;
        if walks_a_failing_step {
            self.a_walk_of_a_failing_sequence(method, span);
        }
        self.s
            .a_pausing_method_in_a_sync_body(&key, &contract, span);
        self.s.a_call_that_may_pause(
            self.parsed.text(method),
            !contract.sync_claim.is_sync() || walks_a_pausing_step,
            unpromised(&contract) && !walks_a_pausing_step,
            span,
        );
        self.s
            .a_pausing_call_in_an_action(self.parsed.text(method), &contract, span);
        // A method call is a written call, so the rule reaches it too
        // (`NK2605`) - and here the receiver's type was known and a
        // ledger described the method, which is the only case this
        // compiler can answer at all.
        if !known_text {
            self.may_fail_here(&key, &contract, span);
        }

        // What the receiver's own type tells the signature (ADR-288).
        // `HashMap[&str, Stats]` against `&HashMap[$K, $V]` binds `$V`
        // to `Stats`, so `-> Entry[$V]` is an `Entry[Stats]` and the
        // next call in the chain has something to bind from in turn.
        let mut bound = bindings(&contract, &on);
        // **And a generic type of the program's own binds its parameters by
        // position** (#497): `h.seen()` on a `Holder[i64]`, for `impl
        // Holder[T] { fn seen(ref self) -> ref T }`, is a `ref i64`, as the
        // field `h.value` is.
        for (name, ty) in self.arguments_of(&on) {
            bound.entry(name).or_insert(ty);
        }

        // The arguments are walked **after** the contract is in hand,
        // which is what lets a lambda's parameters have types (ADR-288).
        // The old order walked them first and could not: `a` in
        // `.and_modify fn { a.add(t) }` is named nowhere and typed by
        // nothing but the callee's signature.
        let expected: Vec<Ty> = expected_arguments(&contract)
            .iter()
            .map(|ty| ty::substitute(ty, &bound))
            .collect();
        let outer_paused = std::mem::take(&mut self.s.flow.paused_args);
        // **A lambda handed to a walk of a `Par[T]` runs on several cores at
        // once** (Part II 12.6, ADR-235 D2): what it may do is asked while it
        // is walked, and a lambda inside it is inside it too.
        let outer_parallel = self.s.in_parallel;
        let outer_scoped = self.s.scoped_task;
        if matches!(&on, Ty::Seq { parallel: true, .. }) {
            self.s.in_parallel = outer_parallel.or(Some(self.s.scope.len() as i64));
            self.s.scoped_task = false;
        }
        // **A task of `task::scope` runs beside the others** (Part II 12.7):
        // asked as a walk's lambda is, at both settings, so the program means
        // the same at each.
        if key == "task::Scope::spawn" {
            self.s.in_parallel = outer_parallel.or(Some(self.s.scope.len() as i64));
            self.s.scoped_task = true;
        }
        let found = self.arguments_given(
            args,
            &expected,
            self.world.own.functions.contains_key(&key),
            Some(&contract),
            span,
        );
        self.s.in_parallel = outer_parallel;
        self.s.scoped_task = outer_scoped;
        // **The element type an empty list's first `push` gives it** (#523):
        // `let mut rows = Vec()` then `rows.push((fragment, tally.n))` makes
        // `rows` a list of what was pushed, for every read after.
        if let Some(name) = self.s.flow.pushed_into.take() {
            // **A `push` onto a list of open elements is a use of them**
            // (ADR-285 D34): a typed value asks for its type, and a number is
            // given to them and held to the type they come to.
            let open = self
                .s
                .binding(&name)
                .and_then(|local| {
                    self.s
                        .flow
                        .open_elements
                        .get(&local.id)
                        .map(|at| *at as usize)
                })
                .map(|at| self.open_root(at));
            // **An empty list's first `push` of a number makes its elements
            // one open number** (#497, ADR-285 D34), as a list written in
            // numbers has: `let mut xs = Vec()` then `xs.push(10)` is a list
            // of whatever integer the uses of `xs[0]` ask for.
            if open.is_none()
                && let ("push", [pushed], [value]) = (entry, found.as_slice(), args)
                && pushed.is_unknown()
                && self.number_shaped(value)
                && let Some(local) = self.s.binding(&name)
                && matches!(&local.ty, Ty::Named { name, args, .. }
                    if name == "Vec" && matches!(args.as_slice(), [] | [Ty::Unknown]))
            {
                let id = local.id;
                self.s.flow.open_numbers.insert(
                    span.at() as i64,
                    OpenNumber {
                        name: name.clone(),
                        element: true,
                        at: *span,
                        joined: span.at() as i64,
                        asks: Vec::new(),
                        given: Vec::new(),
                        derived: Vec::new(),
                        written: Vec::new(),
                    },
                );
                self.open_number_given(span.at(), value, span);
                self.s.flow.open_elements.insert(id, span.at() as i64);
            } else if let (Some(number), "push", [pushed], [value]) =
                (open, entry, found.as_slice(), args)
            {
                if let Some(ty) = integer_named(&value_of_a_copy(pushed.unseen())) {
                    if let Some(number) = self.s.flow.open_numbers.get_mut(&(number as i64)) {
                        number.asks.push((ty, *span, true));
                    }
                } else if self.number_shaped(value) {
                    self.open_number_given(number, value, span);
                }
            }
            let given: Option<(Option<String>, Vec<Ty>)> = match (entry, found.as_slice()) {
                ("push", [pushed]) if !pushed.is_unknown() => Some((
                    Some("Vec".to_string()),
                    vec![value_of_a_copy(pushed.clone())],
                )),
                // `m.insert("a", 7)`: the key and the value, each as the map
                // holds it; the map keeps the name it was made with.
                ("insert", [key, value]) if !key.is_unknown() && !value.is_unknown() => Some((
                    None,
                    vec![value_of_a_copy(key.clone()), value_of_a_copy(value.clone())],
                )),
                _ => None,
            };
            if let Some((kind, elements)) = given
                && let Some(local) = self
                    .s
                    .scope
                    .iter_mut()
                    .rev()
                    .find_map(|frame| frame.iter_mut().rev().find(|local| local.name == name))
                && let Ty::Named { name, args, .. } = &mut local.ty
            {
                if let Some(kind) = kind {
                    *name = kind;
                }
                *args = elements;
            }
        }
        let paused_here = std::mem::replace(&mut self.s.flow.paused_args, outer_paused);
        let pausing_lambda = args.iter().find(|arg| {
            matches!(arg, Expr::Closure { .. }) && paused_here.contains(&value_node(arg))
        });
        self.s
            .a_set_that_reads_what_it_writes(&on, entry, &found, span);
        // **A literal where the receiver says text is wanted** (ADR-293 D21):
        // `m.insert("a", 1)` on a map of `String` keys wants `$K`, which the
        // receiver binds and the per-argument check below does not see. A
        // method's argument is handed over, so the literal is built there.
        for (at, given) in args.iter().enumerate() {
            if let Some(want) = expected.get(at) {
                self.text_literal(want, given, true);
            }
        }
        self.a_sequence_that_cannot_do_this(&on, method, &contract, &found, span);
        self.counts_in_usize(&key, args);
        if key.starts_with(PATH_METHODS) {
            self.s
                .facts
                .path_methods
                .entry(value_node(receiver))
                .or_insert(false);
        }
        // **The source's own name and not the ledger's**, because what
        // `arguments` records is read back by the emitter off the line as it is
        // written: a `&` the compiler owes the witness is looked up under
        // `set`, which is the word on the page.
        let written = self.parsed.text(method).to_string();
        let result = self.arguments(&key, &written, &contract, args, &found, &[], span);
        self.a_lookup_key_lent(&key, args, &found);
        self.a_view_kept_where_the_receiver_says_text(
            &key, &contract, args, &found, &expected, span,
        );
        // **A plain value where the receiver says `T?`** (Part I 2.3):
        // `m.insert(1, Kind::B)` on a map of `Kind?` values wants `$V`, which
        // the receiver binds to a `T?` and `arguments` does not see - the
        // `Some(…)` was never written, and `rustc` said *mismatched types*
        // (found moving `sharing`'s classes into Nikaia, #125).
        for (at, given) in args.iter().enumerate() {
            let (Some(want), Some(found)) = (expected.get(at), found.get(at)) else {
                continue;
            };
            self.a_map_read_kept(found, want, given, span);
            if let Some(how) = self.widened_slot(found, want, Some(given), span) {
                self.s.facts.nullable_args.insert(value_node(given), how);
                continue;
            }
            // **An integer the receiver's type does not hold without loss**
            // (ADR-285 D32): `v.push(b)` with `b: u64` on a `Vec[i64]` was
            // passed here and refused by `rustc`. A literal takes the type it
            // is handed to and is not asked.
            if !is_literal(given) && an_integer_that_does_not_widen(found, want) {
                self.s.findings.push(Finding {
                    severity: Severity::Error,
                    span: *span,
                    code: "NK1102".to_string(),
                    message: format!(
                        "`{key}` expects `{}` here, but you're passing `{}`.",
                        want.text(),
                        found.text()
                    ),
                    notes: Vec::new(),
                    help: Some(convert(found, want)),
                    labels: Vec::new(),
                });
            }
        }
        self.s.a_pattern_of_owned_text(&key, args, &found, span);
        // The receiver first (ADR-288), then whatever the arguments can still
        // say (ADR-295 D2) - `or_insert` on a map that bound `$V` already has
        // its answer, and `bind` does not overwrite one.
        for (name, ty) in from_arguments(&contract, &found) {
            bound.entry(name).or_insert(ty);
        }
        // A method's own parameters carry bounds exactly as a free function's
        // do, and one written call is one rule (ADR-278).
        self.s
            .a_bound_the_argument_does_not_meet(self.world, &key, &bound, span);
        let result = ty::substitute(&result, &bound);
        let result = shape_through(&contract, &on, &found, result);
        let result =
            self.a_lambda_that_pauses_at_an_entry(&key, method, &on, pausing_lambda, result, span);
        self.stamped_through(&contract, &found, result)
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
        method: Ident,
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
        let written = (span.at() as i64, self.parsed.text(method).to_string());
        self.s.flow.settled_methods.remove(&written);
        self.s.flow.pausing_methods.insert(written);
        let callee = key.to_string();
        self.s.a_call_that_may_pause(&callee, true, false, span);
        result
    }

    /// **`NK2703`: a sequence asked for something it is not**
    /// ([ADR-293](../../docs/specification/adr/adr-293.md) D17).
    ///
    /// `rev` writes its receiver `Seq[$T] ends`, and in a receiver's position
    /// the word is a demand: `io::lines().rev()` is refused here, in the
    /// program's words, where it used to be refused by `rustc` in the language
    /// below's (`the trait bound `Lines: DoubleEndedIterator` is not
    /// satisfied`, about a file nobody wrote).
    fn a_sequence_that_cannot_do_this(
        &mut self,
        on: &Ty,
        method: Ident,
        contract: &FnContract,
        found: &[Ty],
        span: &Span,
    ) {
        self.s
            .a_sequence_that_cannot_do_this(self.world, on, method, contract, found, span);
    }

    /// **A count the language below takes in `usize`**
    /// ([ADR-293](../../docs/specification/adr/adr-293.md) D20), recorded for
    /// the emitter by the **entry** the call resolved to - so a method of the
    /// program's own that happens to be called `take` is not converted.
    fn counts_in_usize(&mut self, key: &str, args: &[Expr]) {
        self.s.counts_in_usize(key, args);
    }

    /// **A map's read kept where a `T?` of its own is wanted**
    /// ([ADR-293](../../docs/specification/adr/adr-293.md) D15): the read is
    /// a view of what the map keeps, an `Option<&T>` below, so a value that
    /// copies is copied out - `let k: Kind? = m[1]` - and one that does not
    /// is refused with the copy to write, where it reached `rustc` as
    /// *mismatched types* about a file nobody wrote.
    fn a_map_read_kept(&mut self, found: &Ty, want: &Ty, value: &Expr, span: &Span) {
        self.s.a_map_read_kept(self.world, found, want, value, span);
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
        // **A signed value into an unsigned slot waits for the walk** (D32).
        if let Some(value) = value
            && !is_literal(value)
            && let Some(into) = unsigned_from_signed(found, want)
        {
            self.s
                .flow
                .unsigned_slots
                .entry(span.at() as i64)
                .or_default()
                .push(UnsignedSlot {
                    node: ((value_node(value) as usize) as i64),
                    found: found.clone(),
                    want: want.clone(),
                    into: into.to_string(),
                    written: written(self.parsed, value),
                    span: *span,
                    refusal: None,
                });
            return Some(Wrap::Unsigned(into.to_string()));
        }
        let how = wrap_for(found, want, value.is_some_and(is_literal))?;
        if let (Wrap::Widen(into), Some(value)) = (&how, value)
            && (self.narrow_by_width(value, into, span) || self.widen_operands(value, into, span))
        {
            return None;
        }
        Some(how)
    }

    /// The operands of a computation widen to the type it is computed in
    /// (`tools/check_numbers2.nika`).
    fn widen_operands(&mut self, value: &Expr, into: &str, span: &Span) -> bool {
        self.s.widen_operands(self.world, value, into, span)
    }

    /// `NK1215` for a bit operation computed in a narrower type
    /// (`tools/check_exprs.nika`).
    fn narrow_by_width(&mut self, value: &Expr, into: &str, span: &Span) -> bool {
        self.s.narrow_by_width(self.world, value, into, span)
    }

    /// A list literal whose elements widen (`tools/check_numbers2.nika`).
    fn widened_elements(&mut self, found: &Ty, want: &Ty, value: &Expr, span: &Span) -> Option<Ty> {
        self.s
            .widened_elements(self.world, found, want, value, span)
    }

    /// **A map's read, past a jump, assigned to a place that owns its value**
    /// ([ADR-293](../../docs/specification/adr/adr-293.md) D15, #492):
    /// `x = m[k] ?? return …` reads a view of what the map keeps, as
    /// `let found = m[k] ?? return …` binds one. A value that copies is copied
    /// out, and one that does not is refused with the copy to write - the rule
    /// `let k: Kind? = m[1]` follows - where it reached `rustc` as *mismatched
    /// types*.
    fn a_map_read_assigned(&mut self, into: &Ty, value: &Expr, span: &Span) {
        self.s.a_map_read_assigned(self.world, into, value, span);
    }

    fn wraps_into_nullable(&mut self, found: &Ty, want: &Ty, value: &Expr, span: &Span) {
        self.a_map_read_kept(found, want, value, span);
        let Some(how) = self.widened_slot(found, want, Some(value), span) else {
            return;
        };
        self.s.facts.nullable_sites.insert(span.at() as i64, how);
    }

    /// Part I 2.2: a constant that does not fit the type it is given is a
    /// compile error rather than something the program finds out about at run
    /// time.
    ///
    /// **`NK1116`, and it exists to take a message back rather than to prevent an
    /// abort.** This was already refused where it was written - but by `rustc`, in
    /// Rust's words, down to the lint name `overflowing_literals` and the advice
    /// to use a `u32`, about a file nobody wrote (Part III, C.1). An out-of-range
    /// constant never reached run time and never will; what changes is who says so.
    ///
    /// **A literal alone is asked only where a type stands beside it**, because a
    /// literal has no type of its own on purpose: `Expr::LitInt` answers
    /// `Unknown`, so `add(3)` is right wherever the parameter is numeric, and
    /// `let m = 3000000000` is a correct program where the next line passes `m`
    /// to an `i64` (Part I 2.4, and issue #142's out-of-range literal).
    /// So the places are an
    /// annotated `let`, a `return` against a declared result, and an argument
    /// whose parameter says what it takes.
    ///
    /// **A sum reaches further, and that is ADR-285's gap closing.** Where an
    /// operand's *declaration* pins the type - `let a: i32 = …`, then `a + 1` -
    /// the arithmetic has a type whatever stands beside it, so the bare `let` is
    /// asked too. That is the one case `rustc` refused about the generated file
    /// with *"attempt to compute `i32::MAX + 1_i32`"*.
    ///
    /// **Only the integer types Part I 2.2 offers** — `i64`, `i32`, `u8`, and
    /// since [ADR-285](../../docs/specification/adr/adr-285.md) `u64` and
    /// `u32`. A range that is not checked is `rustc` about the generated file
    /// the first time somebody writes one.
    fn constant_fits(&mut self, value: &Expr, want: Option<&Ty>, span: &Span) {
        self.s.constant_fits(self.world, value, want, span);
    }

    /// `NK1116` for a number and the type it has to fit, where it does not;
    /// nothing where it does (`tools/check_numbers.nika`). `why` is a note
    /// saying where the type came from, for one the uses decided
    /// ([ADR-285](../../docs/specification/adr/adr-285.md)).
    fn a_number_that_does_not_fit(
        &mut self,
        number: &Constant,
        bare: bool,
        ty: &str,
        span: &Span,
        why: Option<String>,
    ) {
        if let Some(found) = nikaia_std::tools::check_numbers::a_number_that_does_not_fit(
            &number.value,
            number.beyond,
            bare,
            ty,
            *span,
            why,
        ) {
            self.s.findings.push(crate::traits::from_nikaia(found));
        }
    }

    /// What a constant integer expression comes to, and the type an operand's
    /// declaration pinned.
    ///
    /// **The arithmetic is [`crate::fold`]'s**, and what this adds is the half
    /// the emitter cannot have: a name resolved to what it is worth
    /// ([ADR-285](../../docs/specification/adr/adr-285.md) D26). Only an
    /// **immutable** `let` carries a value forward to be found here, which
    /// `bind_with` decides; a declaration on the way pins the type, and that is
    /// what makes `let a: i32 = 2` then `a + a` arithmetic in an `i32`.
    fn constant_of(&self, expr: &Expr) -> Option<Constant> {
        self.s.constant_of(self.world, expr).map(Constant::from)
    }

    /// The entry an open number was joined into
    /// ([ADR-285](../../docs/specification/adr/adr-285.md) D23).
    fn open_root(&self, at: usize) -> usize {
        self.s.open_root(at as i64) as usize
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

    /// **An element read out of a list whose elements are open** (D34):
    /// `xs[i]`, and not a slice of it.
    fn open_element_of(&self, expr: &Expr) -> Option<usize> {
        self.s
            .open_element_of(self.world, expr)
            .map(|at| at as usize)
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

    /// **A number given to an open one**: its own value at the `let`, or one
    /// assigned to it later (D3).
    fn open_number_given(&mut self, number: usize, value: &Expr, span: &Span) {
        self.s
            .open_number_given(self.world, number as i64, value, span);
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
        if !matches!(
            op,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
        ) || self.s.flow.overflowed.contains(&(span.at() as i64))
        {
            return;
        }
        let (Some(left), Some(right)) = (self.constant_of(lhs), self.constant_of(rhs)) else {
            return;
        };
        // An operand past the 65 bits has said so where it was folded.
        if left.beyond || right.beyond {
            return;
        }
        // `integers.nika`'s arithmetic. A sum or a difference that leaves the
        // 65 bits heads the way the left operand does, a product the way the
        // signs together do; a division by zero is `NK1118`'s, not this.
        use nikaia_std::tools::integers as int;
        let (a, b) = (&left.value, &right.value);
        let (value, heading) = match op {
            BinaryOp::Add => (int::integer_sum(a, b), a.negative),
            BinaryOp::Sub => (int::integer_difference(a, b), a.negative),
            BinaryOp::Mul => (int::integer_product(a, b), a.negative != b.negative),
            BinaryOp::Div => (int::integer_quotient(a, b, false), false),
            _ => (int::integer_quotient(a, b, true), false),
        };
        let value = match value {
            Some(value) => Constant::of(value),
            None if matches!(op, BinaryOp::Div | BinaryOp::Rem) => return,
            None => Constant {
                beyond: true,
                ..Constant::of(int::integer(u64::MAX, heading))
            },
        };
        // **An open number in it is measured with that number**, once its
        // uses have decided what it is (ADR-285 D25) - and a refusal there
        // names the cause. **Asked before the pinned type**, which an open
        // number never has: `a + a - a` for `let a = 2000000000` recorded the
        // result and not the step, and the step is what the language below
        // refuses (issue #170).
        let mut open = Vec::new();
        self.open_numbers_in(lhs, &mut open);
        self.open_numbers_in(rhs, &mut open);
        if !open.is_empty() {
            for number in open {
                if let Some(number) = self.s.flow.open_numbers.get_mut(&(number as i64)) {
                    number.derived.push((value.clone().into(), *span));
                }
            }
            return;
        }
        let Some(ty) = left.pinned.clone().or(right.pinned.clone()) else {
            return;
        };
        // **Computed in the type a `let` states, where it holds the operands'**
        // (D32): `let t: i64 = a * a` over an `i32` constant is an `i64`.
        let ty = match reach.and_then(|into| common_integer(&ty, into)) {
            Some(common) if Some(common) == reach => common.to_string(),
            _ => ty,
        };
        let before = self.s.findings.len();
        self.a_number_that_does_not_fit(&value, false, &ty, span, None);
        if self.s.findings.len() > before {
            self.s.flow.overflowed.insert(span.at() as i64);
        }
    }

    fn divisor_is_not_zero(&mut self, op: BinaryOp, rhs: &Expr, span: &Span) {
        self.s.divisor_is_not_zero(self.world, op, rhs, span);
    }

    // --- statements ---------------------------------------------------------

    /// Walk a block and hand back the type of its tail.
    fn block(&mut self, block: &Block) -> Ty {
        self.s.scope.push(Vec::new());
        let mut tail = Ty::Tuple(Vec::new());
        let last = block.stmts.len().saturating_sub(1);
        for (at, stmt) in block.stmts.iter().enumerate() {
            let ty = self.stmt(&stmt.node, &stmt.span);
            self.coalesces_not_lent_are_taken();
            // **`break i` is two statements, and that is the point** - a jump
            // takes no value, so the value becomes a statement of its own and
            // the program written is not the program compiled. `while_stmt`
            // has the identical note about the identical failure: three
            // statements, no error, and `rustc` complaining about a file
            // nobody wrote. Asked here rather than in the grammar because the
            // grammar cannot see what follows without a lookahead over every
            // expression there is - and asked of the **next** statement's span,
            // because that is what the author would have to delete.
            if at < last && matches!(stmt.node, Stmt::Break | Stmt::Continue) {
                self.s
                    .nothing_follows_a_jump(&stmt.node, &block.stmts[at + 1].span);
            }
            if at == last {
                tail = ty;
            }
        }
        self.s.scope.pop();
        tail
    }

    fn stmt(&mut self, stmt: &Stmt, span: &Span) -> Ty {
        self.s.flow.literal_receivers.clear();
        match stmt {
            Stmt::Let {
                names,
                mutable,
                ty,
                value,
            } => {
                self.a_field_of_a_borrowed_subject(value, span, "bound");
                self.a_mut_parameter_given_away(value, span, "bound");
                let outer_read = std::mem::replace(&mut self.s.flow.read_a_map, false);
                self.s.flow.reaching =
                    ty.as_ref()
                        .and_then(|t| match Ty::from_ast(self.parsed, t) {
                            Ty::Named {
                                name,
                                args,
                                view: false,
                            } if args.is_empty() => {
                                INTEGERS.iter().find(|n| **n == name).map(|n| n.to_string())
                            }
                            _ => None,
                        });
                if let (Some(t), Expr::Closure { .. } | Expr::ListLit { .. }) = (ty, value) {
                    self.s.flow.lambda_expected = Some(Ty::from_ast(self.parsed, t));
                }
                let found = self.expr(value, span);
                self.s.flow.lambda_expected = None;
                // **A name bound to a map read is a view of the map** (#297):
                // the read itself, not one somewhere inside the value.
                let of_a_map = std::mem::replace(&mut self.s.flow.read_a_map, outer_read)
                    && matches!(value, Expr::Index { .. } | Expr::MethodCall { .. });
                // **A value with a cleanup, bound here, dies where this block
                // ends** (ADR-297 D2).
                if let [single] = names.as_slice() {
                    let name = self.parsed.text(*single).to_string();
                    self.a_value_with_a_cleanup_bound(&name, &found, span);
                }
                // **`let t = s` is a rename, and a rename moves** (ADR-094 D4,
                // ADR-293 D29) - except `let _ = s`, which binds nothing and so
                // takes nothing.
                let bound: Vec<&str> = names.iter().map(|n| self.parsed.text(*n)).collect();
                // **A container a `let` declares a view of what it holds is
                // lent, not handed over** (#452): `let c: ref String = data`
                // over a mapped file is a view of its text, so `data` stays
                // where it is and the line is written `&data`.
                let seen_through = matches!(value, Expr::Variable(_, _) | Expr::Field { .. })
                    && ty.as_ref().is_some_and(|t| {
                        let want = Ty::from_ast(self.parsed, t);
                        !found.fits(&want) && self.fits_through_deref(&found, &want)
                    });
                if seen_through {
                    self.s.facts.lent_lets.insert(span.at() as i64);
                    for name in names {
                        let name = self.parsed.text(*name).to_string();
                        let depth = self.s.scope.len().saturating_sub(1) as i64;
                        self.s.lent_lets.insert((name, depth));
                    }
                } else if bound.iter().any(|n| *n != "_") {
                    let to = format!("bound to `{}` by a `let`", bound.join("`, `"));
                    self.hands_over(value, &found, &to, span);
                }
                // **And a `let` gives the name a value again**, as an
                // assignment does: a name handed over and then bound anew is a
                // new binding, so a read after it is not a read of what was
                // taken. Two loops that each bind `more` and hand it to
                // `take` - `examples/http` - were one name to the path walk.
                let rebound: Vec<String> = bound
                    .iter()
                    .filter(|n| **n != "_")
                    .map(|n| n.to_string())
                    .collect();
                for name in rebound {
                    self.s.written_at.push((name, span.at() as i64));
                }
                // **A `let` over a place is a view of it** (ADR-094 D4), and
                // the emitter needs to know before it writes the line. A bare
                // name is deliberately not a place here: `let y = x` is a
                // rename and stays a move, which is the one shape that
                // separates this rule from `for`'s.
                if matches!(value, Expr::Field { .. } | Expr::Index { .. })
                    && self.s.takes_away(self.world, &found)
                {
                    self.s.facts.lent_lets.insert(span.at() as i64);
                    for name in names {
                        let name = self.parsed.text(*name).to_string();
                        let depth = self.s.scope.len().saturating_sub(1) as i64;
                        self.s.lent_lets.insert((name, depth));
                    }
                }
                // **A tuple of names takes the value apart**
                // ([ADR-291](../../../docs/specification/adr/adr-291.md)). The
                // parts come from the value's own type where it is a tuple of
                // the right width, and are `?` otherwise - the same silence
                // every other unanswered question here keeps, and never a guess
                // that the widths match.
                if let [_, _, ..] = names.as_slice() {
                    return self.tuple_let(names, ty.as_ref(), &found, value, span);
                }
                let name = self.parsed.text(names[0]).to_string();
                // **`let _ = expr` is a statement wearing a `let`**
                // ([ADR-291](../../docs/specification/adr/adr-291.md) D7). A
                // binding that ignores its whole value binds nothing, so the
                // word `let` says something that does not happen - and below it
                // is Rust's `let _ =`, which **discards** the value where the
                // source said *bound*. For a file handle or a lock guard those
                // are different programs, which is why the one thing Rust's form
                // is used for is the one thing this refuses.
                //
                // Only the single-name form: `let (a, _) = pair()` is D1's tuple
                // position and binds `a`.
                if name == "_" {
                    self.s.a_let_that_binds_nothing(span);
                }
                self.s.nameable(&name, span, "a `let`");

                let bound = match ty {
                    Some(ty) => {
                        let want = self.declared(ty, span);
                        // **The annotation is no longer the constructor**
                        // ([ADR-281](../../docs/specification/adr/adr-281.md) D2):
                        // a hull is made by a call, so a plain value standing here
                        // is an ordinary mismatch - and `convert` is what names the
                        // way out.
                        self.constant_fits(value, Some(&want), span);
                        self.wraps_into_nullable(&found, &want, value, span);
                        // **The annotation is a use, and a use answers the
                        // literal** (ADR-152 D4).
                        let found = self
                            .literal_by_use(&found, &want, value, span)
                            .unwrap_or(found);
                        let found = self
                            .widened_elements(&found, &want, value, span)
                            .unwrap_or(found);
                        self.expect_kept(
                            &found,
                            &want,
                            Some(value),
                            "the `let` declares a `String`, which is text of its own",
                            *span,
                            "let",
                            |found, want| {
                                format!("This value is `{found}`, but the `let` declares `{want}`.")
                            },
                        );
                        self.a_number_read_through_a_lent_binding(&want, value, span);
                        want
                    }
                    // **A `let mut` of a literal that the body later grows
                    // holds text of its own** (0.0.232). `let mut s = "a"`
                    // then `s = s + "b"` was `NK1105` - *this is `String`, and
                    // what it is assigned to is `ref String`* - with a help to
                    // take a view of the sum, which is no way out at all. A
                    // literal is built into text of its own wherever it is kept
                    // ([ADR-282](../../docs/specification/adr/adr-282.md) D4),
                    // and a binding the program appends to keeps it: the `let`
                    // reads as `let mut s: String = "a"` would.
                    //
                    // **Only where the growing is written as `+` or an
                    // f-string**, which can only be text of its own. A bare
                    // name assigned later may be a view - `best = w` over a
                    // list of views is the ordinary shape - and one read that
                    // way would refuse it.
                    None if *mutable
                        && self.s.flow.grown_text.contains(&name)
                        && found.is_a_view()
                        && self
                            .text_literal(&Ty::named("String"), value, true)
                            .is_some() =>
                    {
                        Ty::named("String")
                    }
                    None => {
                        // **A number nothing typed is open**
                        // ([ADR-285](../../docs/specification/adr/adr-285.md)
                        // D1): its uses decide, once the body has been walked,
                        // and what it is given is held to that type then.
                        //
                        // **No annotation, and a type all the same** where an
                        // operand's declaration pinned one: `let a: i32 = …`
                        // then `let b = a + 1` is arithmetic in an `i32`, and
                        // that is the sum ADR-285 left to `rustc`.
                        if !(found == Ty::Unknown && self.number_shaped(value)) {
                            self.constant_fits(value, None, span);
                        }
                        found
                    }
                };
                let open = match ty {
                    None if bound == Ty::Unknown && self.number_shaped(value) => {
                        self.s.flow.open_numbers.insert(
                            span.at() as i64,
                            OpenNumber {
                                name: name.clone(),
                                element: false,
                                at: *span,
                                joined: span.at() as i64,
                                asks: Vec::new(),
                                given: Vec::new(),
                                derived: Vec::new(),
                                written: Vec::new(),
                            },
                        );
                        self.open_number_given(span.at(), value, span);
                        Some(span.at())
                    }
                    _ => None,
                };
                // **A list written in numbers has one open number for its
                // elements** ([ADR-285](../../docs/specification/adr/adr-285.md)
                // D34), decided as a `let`'s is: by the uses, else the first
                // type that holds every element.
                let element = match (ty, value) {
                    (None, Expr::ListLit { items, .. })
                        if !items.is_empty()
                            && matches!(&bound, Ty::Named { name, args, .. }
                                if name == "Vec" && matches!(args.as_slice(), [Ty::Unknown]))
                            && items.iter().all(|item| self.number_shaped(item)) =>
                    {
                        self.s.flow.open_numbers.insert(
                            span.at() as i64,
                            OpenNumber {
                                name: name.clone(),
                                element: true,
                                at: *span,
                                joined: span.at() as i64,
                                asks: Vec::new(),
                                given: Vec::new(),
                                derived: Vec::new(),
                                written: Vec::new(),
                            },
                        );
                        for item in items {
                            self.open_number_given(span.at(), item, span);
                        }
                        let elements = match mutable {
                            false => items
                                .iter()
                                .map(|item| self.constant_of(item).map(CheckConstant::from))
                                .collect(),
                            true => None,
                        };
                        Some((span.at(), elements))
                    }
                    _ => None,
                };
                // **An empty list has no element type, and D2 says where one
                // comes from**
                // ([ADR-135](../../docs/specification/adr/adr-135.md)): the
                // first use that needs one. An annotation is that use and
                // answers it here; without one the question is left open and
                // asked again when the body has been walked, because the use
                // that answers it stands *after* this line.
                let pending = match (ty, value) {
                    (None, Expr::ListLit { items, .. }) if items.is_empty() => {
                        self.s
                            .flow
                            .empty_lists
                            .insert(span.at() as i64, (name.clone(), *span));
                        Some(span.at())
                    }
                    _ => None,
                };
                // **Only an immutable `let` carries its value forward.** A
                // `mut` one may be given another before the name is read again,
                // and this checker does not follow assignments - so the value
                // it started with would be a claim about a program that no
                // longer holds (Part III, C.4).
                let constant = match mutable {
                    false => self.constant_of(value).map(|c| c.value),
                    true => None,
                };
                // **What a task binds for itself** (ADR-055 D6): recorded
                // where the type is known, because the frame goes when the body
                // does.
                if let Some(task) = self.s.task_bindings.last_mut() {
                    task.push((name.clone(), bound.clone(), span.at() as i64));
                }
                let id = self.a_new_binding();
                if let Some((at, elements)) = element {
                    self.s.flow.open_elements.insert(id, at as i64);
                    if let Some(elements) = elements {
                        self.s.flow.list_constants.insert(id, elements);
                    }
                }
                // **And typed as one**: a `V?` read out of a map is an option
                // of a view of the `V` the map keeps, which is what a `ref V?`
                // is - so `found?.clone()` copies the value and not the view.
                let bound = match (of_a_map, bound) {
                    (true, Ty::Nullable(inner)) if !inner.is_a_view() => {
                        Ty::Nullable(Box::new(view_of(&inner)))
                    }
                    (_, bound) => bound,
                };
                if of_a_map {
                    self.s.flow.map_views.insert(id);
                }
                // **A read through the brackets that jumps where nothing is
                // there binds what the container holds, not a copy of it**:
                // `let first = seen[name] ?? continue` is the `Some` of a
                // `get` below, a `&String`, which a comparison with a `String`
                // has no impl for (found moving `types` into Nikaia, #125). A
                // view below whatever its type says, as a `for` binding is.
                let read_through_a_jump = matches!(value, Expr::Coalesce { value: read, fallback, .. }
                    if matches!(**read, Expr::Index { .. })
                        && self.s.jumps(self.world, fallback));
                // **Unless what it reads is a number, a `bool` or a `char`**
                // (#456): then the value is read out of the view where it is
                // bound, as a loop binding's is, and the name is the value in
                // every position below - a `get` included, whose answer is
                // the same view; `num::value` reads a value as itself, so an
                // owned answer is read as well. Bound as a view, `values.insert(name, v)`
                // handed the map a `&i64`.
                let copied_read = a_copy_by_value(&bound)
                    && matches!(value, Expr::Coalesce { value: read, fallback, .. }
                        if (matches!(**read, Expr::Index { .. })
                            || matches!(&**read, Expr::MethodCall { method, .. }
                                if self.parsed.text(*method) == "get"))
                            && self.s.leaves(self.world, fallback));
                if copied_read {
                    self.s.facts.copied_jump_reads.insert(value_node(value));
                }
                let read_through_a_jump = read_through_a_jump && !copied_read;
                if matches!(&bound, Ty::Named { name, .. } if name == "fs::Path")
                    && self.made_from_text(value)
                {
                    self.s.flow.text_paths.insert(id);
                }
                if bound.is_unknown() || self.s.a_drivers_row(self.world, value) {
                    self.s.flow.unknown_bound.insert(id);
                }
                self.bind_local(Local {
                    id,
                    name,
                    ty: bound,
                    constant,
                    // **A `let` is not lent here** even where D4 lends its
                    // initialiser: what the emitter writes there is a `&` in
                    // front of a place, and a cast over the name that comes
                    // out of one is the language below's own deref.
                    lent: read_through_a_jump,
                    changing: false,
                    // A `let` binds a number where one folded; the whole value
                    // is a `comptime`'s, which is the one that *must* fold.
                    built: None,
                    empty_list: pending.map(|n| n as i64),
                    open_number: open.map(|n| n as i64),
                    literal: match (ty, value) {
                        (None, Expr::LitStr { text, .. }) => Some(text.clone()),
                        _ => None,
                    },
                    // **Part I 2.1**: without the word, a change to this name
                    // is `NK1139`. The span is the statement's, which is the
                    // `let` itself — a binding has no narrower one.
                    immutable: (!mutable).then_some(Immutable {
                        at: *span,
                        kind: Kind::Let,
                    }),
                    written_mut: *mutable,
                });
                Ty::Tuple(Vec::new())
            }

            // **`comptime` is a `let` that has to fold**
            // ([ADR-287](../../docs/specification/adr/adr-287.md) D4). The fold
            // is the one [ADR-285](../../docs/specification/adr/adr-285.md)
            // already shares, so nothing new evaluates anything: what this arm
            // adds is the refusal where it comes back empty, and the type the
            // emitter will need (D4).
            //
            // **Refused by name rather than run at program time** (D5). Falling
            // back would break the promise the word is for, and quietly - the
            // program would still work and the guarantee would be gone.
            Stmt::Comptime {
                name,
                ty,
                value,
                bounds,
            } => self.comptime_binding(*name, ty.as_ref(), value, bounds, span),

            Stmt::Assign {
                target, op, value, ..
            } => {
                // **Code assigned into a function field** (ADR-230 D1): the
                // lock column does not read it, so the field is undecided.
                if let Expr::Field { base, name, .. } = target {
                    let field = self.parsed.text(*name).to_string();
                    // The owner where the base is a name; every struct with a
                    // function field of that name where it is not.
                    let owners: Vec<String> = match base.as_ref() {
                        Expr::Variable(bound, _) => match self.s.lookup(self.parsed.text(*bound)) {
                            Some(Ty::Named { name: owner, .. }) => vec![owner],
                            _ => Vec::new(),
                        },
                        _ => self
                            .parsed
                            .program
                            .items
                            .iter()
                            .filter_map(|item| match &item.node {
                                Item::Struct { name, .. } => {
                                    Some(self.parsed.text(*name).to_string())
                                }
                                _ => None,
                            })
                            .collect(),
                    };
                    for owner in owners {
                        let code = self.s.a_field_holds_code(self.world, &owner, &field);
                        if code {
                            self.a_field_whose_code_is_not_seen(&owner, &field);
                        }
                    }
                }
                // `NK2101`: an assignment **revives** a name a task took with
                // it - `message = "other"` after the `spawn` is a correct
                // program - so the target is recorded before it is walked, and
                // walking it records a read this must not be confused with.
                // **And a part of one** (ADR-293 D32): `p.name = …` gives back
                // what `xs.push(p.name)` took.
                if let Some(path) = self.s.place_path(self.world, target) {
                    let choices = self.s.branch.clone();
                    self.s.gives.push(Give {
                        path: path.clone(),
                        at: span.at() as i64,
                        choices,
                    });
                    self.s.written_at.push((path, span.at() as i64));
                }
                // **D3**: an assignment into a parameter, or into a place
                // rooted at one, changes the caller's value and says `mut`.
                // A name that holds shared mutable state is not asked for
                // `mut`: writing it directly is `NK2204`'s, one message for one
                // mistake, and `mut` would not be the way out.
                let shared = matches!(target, Expr::Variable(_, _))
                    && self.s.rooted_at(self.world, target).and_then(|root| self.s.binding(&root)).is_some_and(
                        |local| matches!(&local.ty, Ty::Named { name, .. } if name == SHARED_MUT),
                    );
                if let Some(root) = self.s.rooted_at(self.world, target).filter(|_| !shared) {
                    let how = match target {
                        Expr::Variable(_, _) => "changed here",
                        _ => "one of its parts is changed here",
                    };
                    self.s.a_changed_binding_says_mut(&root, how, *span);
                    self.s.a_shared_name_changed_in_parallel(&root, span);
                }
                // **A write through the brackets is unchanged**
                // ([ADR-293](../../docs/specification/adr/adr-293.md) D2):
                // `m[k] = v` inserts or replaces, and what it inserts is a `V`.
                // Only the **read** answers a `T?`, so the slot being written
                // is the value type with that answer taken back off — without
                // this, the write would be handed a `Some(v)` for a map whose
                // values are plain.
                // The target is **written**, not taken: `s = xs.iter()` gives a
                // walked sequence a new one (ADR-293 D19). What is inside an
                // index is still read, and is walked by its own arm.
                let outer = std::mem::replace(
                    &mut self.s.reading_only,
                    matches!(target, Expr::Variable(_, _)),
                );
                // **A key written is handed to the map** (ADR-293 D27): the
                // `Index` arm reads this for the one index it is walking.
                self.s.flow.writing_index = op.is_none() && matches!(target, Expr::Index { .. });
                let into = match target {
                    Expr::Index { .. } => match self.expr(target, span) {
                        Ty::Nullable(inner) => *inner,
                        other => other,
                    },
                    _ => self.expr(target, span),
                };
                self.s.reading_only = outer;
                self.s.flow.writing_index = false;
                let found = self.expr(value, span);
                if op.is_none() {
                    self.a_map_read_assigned(&into, value, span);
                    self.a_map_first_written(target, &found);
                }
                // **An assignment keeps what it is given**, so a text literal
                // assigned to a `String` place - a name, a field, a map's or a
                // list's slot - is built into text of its own there, as it is at
                // every other place that keeps it (ADR-282 D3, ADR-293 D28).
                if op.is_none() {
                    self.a_kept_lambda(&into, value, span);
                }
                let found = match op {
                    None => self.text_literal(&into, value, true).unwrap_or(found),
                    Some(_) => found,
                };
                // **An assignment is a use of both sides, and a number given
                // to the one on the left**
                // ([ADR-285](../../docs/specification/adr/adr-285.md) D24, D25):
                // `n = xs.len()` makes an open `n` an `i64`, `n = 3000000000`
                // is held to whatever `n` is, and `total = n` asks `n` for
                // `total`'s type. A shift's count is any integer and asks
                // nothing.
                if !matches!(op, Some(BinaryOp::Shl | BinaryOp::Shr)) {
                    let open = match target {
                        Expr::Variable(name, _) => self
                            .s
                            .binding(self.parsed.text(*name))
                            .and_then(|local| local.open_number.map(|n| n as usize))
                            .map(|at| self.open_root(at)),
                        Expr::Index { .. } => {
                            self.open_element_of(target).map(|at| self.open_root(at))
                        }
                        _ => None,
                    };
                    match open {
                        Some(_) if integer_named(&value_of_a_copy(found.unseen())).is_some() => {
                            self.number_asked(target, &found, span, true);
                        }
                        Some(number) if self.number_shaped(value) => {
                            self.open_number_given(number, value, span);
                        }
                        _ => {}
                    }
                    self.number_asked(value, &into, span, true);
                }
                // **What is assigned is given to the place** (ADR-293 D29).
                if op.is_none() {
                    let to = match target {
                        Expr::Index { .. } => "written into a container",
                        _ => "assigned to another name",
                    };
                    self.hands_over(value, &found, to, span);
                }
                // **A write to shared mutable state goes through a door**
                // ([ADR-281](../../../docs/specification/adr/adr-281.md)).
                self.a_write_that_skips_the_door(target, &into, value, span);
                // **A compound assignment on a map slot is written out**
                // ([ADR-293](../../docs/specification/adr/adr-293.md) D2).
                if op.is_some() {
                    self.a_compound_write_to_a_map_slot(target, span);
                }
                // **`x += a` over two integer types is `x = x + a`**
                // ([ADR-285](../../docs/specification/adr/adr-285.md) D32): it
                // computes in their common type, which has to be `x`'s.
                if let Some(op) = op
                    && !matches!(op, BinaryOp::Shl | BinaryOp::Shr)
                {
                    self.a_compound_over_two_integer_types(*op, target, &into, value, &found, span);
                }
                // **`x += d` is `x = x + d`, and may be proved as one** (#387):
                // a name of one whole-number type and a value of it, or a
                // literal, recorded by the statement's first byte - where no
                // operator of an expression can stand - for
                // `remove-overflow-checks` (ADR-306 D10).
                if let (
                    Some(BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul),
                    Expr::Variable(_, _),
                    Ty::Named {
                        name,
                        args,
                        view: false,
                    },
                ) = (op, target, into.unseen())
                    && args.is_empty()
                    && is_a_whole_number(&name)
                    && (matches!(value, Expr::LitInt { .. })
                        || value_of_a_copy(found.unseen()) == into.unseen())
                {
                    self.s.facts.arithmetic.insert(span.at() as i64, name);
                }
                // **`xs[i] += d` on a list of numbers** (#387): the slot is a
                // copy, so it can be read once and written back.
                if let (
                    Some(
                        BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Rem
                        | BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::Shl
                        | BinaryOp::Shr,
                    ),
                    Expr::Index { .. },
                    Ty::Named {
                        name,
                        args,
                        view: false,
                    },
                ) = (op, target, into.unseen())
                    && args.is_empty()
                    && (is_number(&name) || name == "bool")
                {
                    self.s.facts.copied_slots.insert(span.at() as i64);
                }
                // **`NK1191` for `-=` as for `-`**: `a -= 30` on an
                // `Account` reached `rustc` as E0368, and `xs += [3]` on a list
                // as the same. `+=` onto text is a concatenation and is left
                // alone, as `+` is.
                if let Some(
                    arithmetic @ (BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem),
                ) = op
                    && !(matches!(arithmetic, BinaryOp::Add) && is_text(&into))
                    && (self.has_no_arithmetic(&into) || self.has_no_arithmetic(&found))
                {
                    self.arithmetic_on_something_that_has_none(
                        *arithmetic,
                        [(target, &into), (value, &found)],
                        true,
                        span,
                    );
                }
                // Only a plain assignment: `n += 1` is whatever the operator
                // makes of the two, and Stage 0 does not model operators.
                if op.is_none() {
                    self.wraps_into_nullable(&found, &into, value, span);
                    if self.owned_text_into_a_literal_binding(target, &found, &into, span) {
                        return Ty::Tuple(Vec::new());
                    }
                    if self.a_name_in_text(&into, Some(value), span) {
                        return Ty::Tuple(Vec::new());
                    }
                    self.expect(&found, &into, *span, "assign", |found, want| {
                        format!("You're assigning `{found}` to something that holds `{want}`.")
                    });
                }
                Ty::Tuple(Vec::new())
            }

            Stmt::While { cond, body } => {
                let cond_ty = self.expr(cond, span);
                self.expect_bool(
                    &cond_ty,
                    span,
                    "A `while` loop repeats while a `bool` is `true`.",
                );
                let depth = self.s.scope.len() as i64;
                self.s
                    .repeats
                    .push(Repeats::a_loop(&self.parsed.interner, depth, body));
                self.s.scope.push(Vec::new());
                // **The body and not the condition.** A `break` written in the
                // condition is bound to this very loop in the language below,
                // which is legal there and is a program nobody writes; refusing
                // it is the direction that can be taken back later, and
                // accepting it is not.
                self.s.flow.loops += 1;
                self.block(body);
                self.s.flow.loops -= 1;
                self.s.scope.pop();
                self.s.repeats.pop();
                Ty::Tuple(Vec::new())
            }

            Stmt::For {
                bindings,
                iter,
                body,
            } => {
                self.s
                    .the_caller_writes_no_reference(iter, span, "a `for` lends what it iterates");
                let over = self.expr(iter, span);
                self.a_loop_over_a_nullable(&over, iter, span);
                self.fallible_step(&over, bindings.len(), span);
                self.pausing_step(&over, span);
                // **A `for` walks a produced sequence, and that consumes it**
                // ([ADR-105](../../docs/specification/adr/adr-105.md) D2). A
                // container is walked by view and as often as one likes, which
                // is why the record is keyed on the type being a `Seq`.
                self.s.a_sequence_is_walked(iter, &over, span, self.world);
                let elements = elements_of(&over, bindings.len());
                // **A name that holds a sequence is handed over, not lent**
                // (ADR-293 D19): the emitter writes no `.iter()` for it, and the
                // binding is each element itself.
                let owned = crate::emit::is_a_place(iter) && matches!(over, Ty::Seq { .. });
                if owned {
                    self.s.facts.owned_loops.insert(span.at() as i64);
                }
                // **A `for` over a place lends** (ADR-094 D4), which the
                // emitter writes as `.iter()` — so the binding is a *view* of
                // each element. Recorded on the binding because the one thing
                // that has to know is a **cast** over it, and Rust's `as` does
                // not see through a reference. The predicate is the emitter's
                // own, so the two cannot answer differently.
                let lent = crate::emit::is_a_place(iter) && !owned;
                let mut frame: Vec<Local> = Vec::new();
                for (b, element) in bindings.iter().zip(elements) {
                    let name = self.parsed.text(*b).to_string();
                    // `_` is read nowhere, so nothing is read out of it.
                    let copied = lent && name != "_" && a_copy_by_value(&element);
                    if copied {
                        self.s
                            .facts
                            .copied_loop_bindings
                            .insert(((span.at() as i64), name.clone()));
                    }
                    // A view is copied as a number is: it is a pointer.
                    let a_view = lent && name != "_" && !copied && element.is_a_view();
                    if a_view {
                        self.s
                            .facts
                            .copied_view_bindings
                            .insert(((span.at() as i64), name.clone()));
                    }
                    // **A binding lent something that does not copy is a
                    // view of it**, and typed as one: what keeps it needs its
                    // own, and says so where the view would be kept (`NK1105`
                    // and its neighbours), rather than `rustc` saying
                    // *expected `String`, found `&String`*.
                    let element = match lent && !copied && matches!(element, Ty::Named { .. }) {
                        true => view_of(&element),
                        false => element,
                    };
                    frame.push(Local {
                        written_mut: false,
                        lent: lent && !copied && !a_view,
                        ..self.local_free(name, element)
                    });
                }
                // **A range's binding is a use of the numbers it counts
                // between** ([ADR-285](../../docs/specification/adr/adr-285.md)
                // D1, [ADR-270](../../docs/specification/adr/adr-270.md) D8
                // step 1): in `for lit in 0..<(2 * variables) { lists[lit] }`
                // the index asks `variables` for its type through `lit`. Left
                // unjoined, nothing asked, the number was written with no type,
                // and the language below could not infer what the index read.
                if let ([_], Expr::Range { start, end, .. }) = (bindings.as_slice(), iter) {
                    let mut open = Vec::new();
                    self.open_numbers_in(start, &mut open);
                    self.open_numbers_in(end, &mut open);
                    self.open_numbers_joined(&open);
                    if let (Some(first), Some(local)) = (open.first(), frame.first_mut()) {
                        local.open_number = Some(self.open_root(*first) as i64);
                    }
                    // **Between two literals the binding is a number of its
                    // own** (D1, D3): `for k in 0..<55` counts in the type its
                    // uses ask for, and the ends are what it is given.
                    if open.is_empty()
                        && self.number_shaped(start)
                        && self.number_shaped(end)
                        && let Some(local) = frame.first_mut()
                        && local.ty.is_unknown()
                    {
                        self.s.flow.open_numbers.insert(
                            span.at() as i64,
                            OpenNumber {
                                name: local.name.clone(),
                                element: false,
                                at: *span,
                                joined: span.at() as i64,
                                asks: Vec::new(),
                                given: Vec::new(),
                                derived: Vec::new(),
                                written: Vec::new(),
                            },
                        );
                        local.open_number = Some(span.at() as i64);
                        self.open_number_given(span.at(), start, span);
                        self.open_number_given(span.at(), end, span);
                    }
                }
                // **A walk over a list written in numbers binds its open
                // element** (ADR-285 D34): `for n in [1, 2, 3]`, and `for x in
                // xs` over such a list's name, count in the type the uses ask.
                if let [_] = bindings.as_slice()
                    && let Some(local) = frame.first_mut()
                    && local.ty.is_unknown()
                {
                    match iter {
                        Expr::Variable(name, _) => {
                            if let Some(at) =
                                self.s.binding(self.parsed.text(*name)).and_then(|list| {
                                    self.s
                                        .flow
                                        .open_elements
                                        .get(&list.id)
                                        .map(|at| *at as usize)
                                })
                            {
                                local.open_number = Some(self.open_root(at) as i64);
                            }
                        }
                        Expr::ListLit { items, .. }
                            if !items.is_empty() && items.iter().all(|i| self.number_shaped(i)) =>
                        {
                            self.s.flow.open_numbers.insert(
                                span.at() as i64,
                                OpenNumber {
                                    name: local.name.clone(),
                                    element: false,
                                    at: *span,
                                    joined: span.at() as i64,
                                    asks: Vec::new(),
                                    given: Vec::new(),
                                    derived: Vec::new(),
                                    written: Vec::new(),
                                },
                            );
                            local.open_number = Some(span.at() as i64);
                            for item in items {
                                self.open_number_given(span.at(), item, span);
                            }
                        }
                        _ => {}
                    }
                }
                for local in &frame {
                    self.s
                        .nameable(&local.name.clone(), span, "a `for` binding");
                }
                let depth = self.s.scope.len() as i64;
                self.s
                    .repeats
                    .push(Repeats::a_loop(&self.parsed.interner, depth, body));
                self.s.scope.push(frame);
                self.s.flow.loops += 1;
                self.block(body);
                self.s.flow.loops -= 1;
                self.s.scope.pop();
                self.s.repeats.pop();
                Ty::Tuple(Vec::new())
            }

            Stmt::Return(value) => {
                self.returns(value.as_ref(), span, false);
                self.s.a_way_out("return", "", span);
                Ty::Unknown
            }

            // Part I 3.3. **`Ty::Unknown` and not `()`**, for the reason a
            // `return` hands back one: a statement that jumps is not a value,
            // and a block that ends in one is a block nothing arrives at the
            // end of. `if c { break } else { 1 }` would otherwise be an `if`
            // whose halves disagree, which is a refusal about a program that is
            // right - the language below reads the jumping half as the `!` it
            // is and coerces it to the other.
            Stmt::Break | Stmt::Continue => {
                let word = match stmt {
                    Stmt::Break => "break",
                    _ => "continue",
                };
                if self.s.flow.loops == 0 {
                    self.a_jump_with_nowhere_to_go(word, span);
                }
                Ty::Unknown
            }

            // **One site, not two.** A statement that is one name is an
            // expression statement, so the walk below reaches it - and the
            // refusal now lives where every name is read rather than only where
            // one stands alone.
            Stmt::Expr(expr) => self.expr(expr, span),
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
                                self.stmt(other, &stmt.span);
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
                self.expect_bool(&cond_ty, span, "An `if` decides on a `bool`.");
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
                        if then == other || (leaves(otherwise) && !leaves(then_branch)) {
                            then
                        } else if leaves(then_branch) && !leaves(otherwise) {
                            other
                        } else if let Some(typed) = self.a_number_beside_a_number(
                            (tail_of(then_branch), &then),
                            (tail_of(otherwise), &other),
                            span,
                        ) {
                            typed
                        } else {
                            let arms = [(tail_of(then_branch), then), (tail_of(otherwise), other)];
                            self.arms_meet_at_text(&arms, span)
                                .or_else(|| self.arms_meet_at_null(&arms))
                                .unwrap_or(Ty::Unknown)
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
                let several: Option<Vec<Ty>> = several_errors(&on).map(<[Ty]>::to_vec);
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
                        self.expect_bool(&found, span, "A `match` guard is a condition.");
                    }
                    let ty = self.expr(&arm.body, span);
                    self.a_part_lent_where_it_is_not_kept(
                        &arm.body,
                        &parts,
                        from,
                        changed_from,
                        through_the_reference,
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
                    _ => self
                        .arms_meet_at_text(&answered, span)
                        .or_else(|| self.arms_meet_at_null(&answered))
                        .unwrap_or(Ty::Unknown),
                }
            }

            Expr::Call {
                func, args, config, ..
            } => match self.the_prelude_assert(func) {
                true => self.an_assert(args, config, span),
                false => self.call(func, args, config, span),
            },

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
                // **`after:` on a `set` is a witness and not an option**
                // ([ADR-281](../../docs/specification/adr/adr-281.md) D26): it
                // is walked below as an *argument*, where the door's signature
                // is what says it has to be a `$T`. Found here by name, and
                // confirmed to be the door once the receiver's type is in hand
                // - a `set` on something that is not a lock takes this back.
                // **An empty list's first `push`** (#523, ADR-135 D2): the
                // name is held for `call_on`, which has the argument's type.
                // **And an empty map's first `insert`** (#543): its key and
                // value types, the same way.
                self.s.flow.pushed_into = match (&**receiver, self.parsed.text(*method), args.len())
                {
                    (Expr::Variable(name, _), "push", 1)
                        if self
                            .s
                            .binding(self.parsed.text(*name))
                            .is_some_and(|local| {
                                matches!(&local.ty, Ty::Named { name, args, .. }
                                if name == "Vec" && matches!(args.as_slice(), [] | [Ty::Unknown]))
                            }) =>
                    {
                        Some(self.parsed.text(*name).to_string())
                    }
                    (Expr::Variable(name, _), "insert", 2)
                        if self
                            .s
                            .binding(self.parsed.text(*name))
                            .is_some_and(|local| {
                                matches!(&local.ty, Ty::Named { name, args, .. }
                                if a_map(name)
                                    && matches!(args.as_slice(), [] | [Ty::Unknown, Ty::Unknown]))
                            }) =>
                    {
                        Some(self.parsed.text(*name).to_string())
                    }
                    _ => None,
                };
                let witness = match self.parsed.text(*method) == "set" {
                    true => config
                        .iter()
                        .position(|a| self.parsed.text(a.name) == "after"),
                    false => None,
                };
                // ADR-296 D5: a DSL's deferred parameters stand here. They are
                // expressions like any other, so they are walked - what checks
                // that they are the *right* names is `dsl::check`, which knows
                // which statement the receiver came from.
                config.iter().enumerate().for_each(|(at, a)| {
                    if Some(at) != witness {
                        self.expr(&a.value, span);
                    }
                });
                // **A grammar is entered by an ordinary call**
                // ([ADR-296](../../docs/specification/adr/adr-296.md) D24), so
                // this is that call: a receiver naming a grammar of this file
                // and a method naming one of its `entry` rules. Answered before
                // the receiver is typed, because a grammar name is not a value
                // and typing it would be asking the wrong question.
                // **Both ways out of here owe the witness its walk.** It was
                // held back above so that the door can walk it as an argument,
                // and neither of these reaches the door: a grammar rule is not
                // a lock, and a `T?` receiver is answered before the call is.
                // A value that nothing walks is a mistake nothing reports.
                if let Some(entered) = self.s.grammar_entry(self.world, receiver, *method) {
                    if let Some(at) = witness {
                        self.expr(&config[at].value, span);
                    }
                    // **The dot is gone** (ADR-140 D3), and the refusal is
                    // here rather than in the parser because only this side
                    // knows the receiver names a grammar: `Json.value(x)` and
                    // `text.value(x)` are the same five tokens.
                    let (grammar, rule) = entered.split_once("::").unwrap_or((&entered, ""));
                    self.s.a_grammar_reached_through_a_dot(grammar, rule, span);
                    return self.grammar_call(&entered, args, span);
                }
                let on = self.expr(receiver, span);
                // **A number written as a receiver takes the first type that
                // holds it** (Part I 2.4, #417): nothing stands beside it to
                // ask for another, and a literal has no type of its own here,
                // so `(-7).wrapping_add(2)` was a method on nothing known - the
                // function was taken to pause, and the language below could
                // not tell which integer it was. The type is written on the
                // literal ([`Checked::unsigned_literals`]).
                let on = match (&**receiver, &on) {
                    (
                        Expr::LitInt {
                            value, negative, ..
                        },
                        Ty::Unknown,
                    ) => {
                        let number =
                            Constant::of(nikaia_std::tools::integers::integer(*value, *negative));
                        let ty = match number.fits("i64") {
                            true => number.first_type(),
                            false => "u64",
                        };
                        self.s
                            .facts
                            .unsigned_literals
                            .insert(value_node(receiver), ty.to_string());
                        Ty::named(ty)
                    }
                    _ => on,
                };
                // **`x?.m(…)`** (Part I 3.5): over a `T?` the call is made on
                // what is inside; over a type nothing worked out it says
                // nothing; over a plain value it is the plain call, with
                // `NK1217` ([ADR-279](../../docs/specification/adr/adr-279.md)
                // D13).
                if matches!(expr, Expr::SafeMethod { .. }) {
                    let name = self.parsed.text(*method).to_string();
                    match on {
                        Ty::Nullable(inner) => {
                            return self
                                .a_safe_call(inner, receiver, method, args, config, witness, span);
                        }
                        Ty::Unknown => {
                            args.iter().for_each(|a| {
                                self.expr(a, span);
                            });
                            if let Some(at) = witness {
                                self.expr(&config[at].value, span);
                            }
                            return Ty::Unknown;
                        }
                        _ => self.reaches_through_a_plain_value(
                            &on,
                            receiver,
                            Reached::Method(&name),
                            expr,
                            span,
                        ),
                    }
                }
                // **A length `std` counts**: of a list, a run, text or a map,
                // whose `len()` reads the value and changes nothing - what
                // `crate::bounds` may take as a number of a proof (ADR-306 D4).
                if args.is_empty() && self.parsed.text(*method) == "len" {
                    let counted = match &on {
                        Ty::Named { name, .. } => matches!(
                            crate::contracts::ty::base(name),
                            "Vec"
                                | "List"
                                | "Array"
                                | "String"
                                | "str"
                                | "HashMap"
                                | "Map"
                                | "BTreeMap"
                                | "HashSet"
                                | "Set"
                                | "BTreeSet"
                        ),
                        Ty::Pointed { slice: true, .. } => true,
                        _ => false,
                    };
                    if counted {
                        self.s.facts.std_lengths.insert(value_node(receiver));
                        if self.s.elements_take_space(&on) {
                            self.s.facts.sized_lengths.insert(value_node(receiver));
                        }
                    }
                }
                // **`m.get(k)` reads the map as `m[k]` does** (#297): what it
                // hands back is a view of the value the map keeps.
                let reads_a_map = self.parsed.text(*method) == "get"
                    && matches!(&on, Ty::Named { name, .. }
                        if matches!(crate::contracts::ty::base(name), "HashMap" | "Map" | "BTreeMap"));
                if reads_a_map {
                    self.s.flow.read_a_map = true;
                }
                // **A receiver the method only reads is lent** (ADR-279 D5):
                // `(name ?? "guest").len()` asks the answer and keeps nothing
                // of it, where every candidate for the method takes its
                // receiver as a view and changes nothing.
                if matches!(receiver.as_ref(), Expr::Coalesce { .. })
                    && self
                        .s
                        .only_reads_its_receiver(self.world, self.parsed.text(*method))
                {
                    self.a_pending_coalesce_is_lent(receiver);
                }
                // **The tier pass's own conversion**
                // ([ADR-282](../../docs/specification/adr/adr-282.md) D5):
                // `value.into_either()` is written into the program where a
                // value goes into text both kinds flow into, and it hands the
                // value on as it is - absent where it may be absent - so it
                // reads as the value's own type, and what the position accepts
                // is decided exactly as it was before the pass wrote it.
                // Into a `String?` it is `value.into_either_maybe()`, which is
                // absent-or-present whatever the value was, so no wrap is
                // written around it; before a `collect`, `either_items()`,
                // which is the same items.
                // What goes in is `EitherText`, which reads as text of its
                // own (ADR-282 D15): a view handed over this way is not a view
                // kept.
                let own = |ty: Ty| match ty {
                    ty if ty == Ty::view("str") => Ty::named("String"),
                    Ty::Nullable(inner) if *inner == Ty::view("str") => {
                        Ty::Nullable(Box::new(Ty::named("String")))
                    }
                    ty => ty,
                };
                if args.is_empty() && config.is_empty() {
                    match self.parsed.text(*method) {
                        "into_either" => return own(on),
                        "into_either_maybe" => {
                            return match own(on) {
                                Ty::Nullable(inner) => Ty::Nullable(inner),
                                other => Ty::Nullable(Box::new(other)),
                            };
                        }
                        "either_items" | "either_keys" | "either_values" | "either_pairs" => {
                            let which = self.parsed.text(*method);
                            // Of a pair, only the half the name says.
                            let own = |item: Ty| match (which, item) {
                                ("either_items", item) => own(item),
                                (_, Ty::Tuple(parts)) => Ty::Tuple(
                                    parts
                                        .into_iter()
                                        .enumerate()
                                        .map(|(at, part)| match (which, at) {
                                            ("either_keys", 0)
                                            | ("either_values", 1)
                                            | ("either_pairs", _) => own(part),
                                            _ => part,
                                        })
                                        .collect(),
                                ),
                                (_, item) => item,
                            };
                            return match on {
                                Ty::Seq {
                                    item,
                                    is_sync,
                                    pauses,
                                    can_throw: throws,
                                    parallel,
                                    shape,
                                } => Ty::Seq {
                                    item: Box::new(own(*item)),
                                    is_sync,
                                    pauses,
                                    can_throw: throws,
                                    parallel,
                                    shape,
                                },
                                other => other,
                            };
                        }
                        _ => {}
                    }
                }
                // **`field.of(value)`, the one method a reflected field has**
                // ([ADR-304](../../docs/specification/adr/adr-304.md) D2).
                //
                // What it answers is the field's type **on this unrolling**, and
                // `?` where there is none — which is the generic body, walked
                // once with nothing known. That walk must refuse nothing about a
                // field it cannot see, because a body wrong for one field is
                // wrong at one unrolled copy and right at the others (D5), and
                // saying so from the generic walk would be saying it about all
                // of them.
                // **`variant.is(value)`, the one method a reflected variant
                // has**: whether the value is that variant. A `bool` on every
                // turn, so the generic walk and the unrolled one agree.
                if matches!(&on, Ty::Named { name, .. } if name == ty::VARIANT) {
                    // A variant's attributes are read as a field's are (D6).
                    let named = self.parsed.text(*method).to_string();
                    if matches!(named.as_str(), "attribute" | "attributes") {
                        let Some(read) = self.an_attribute_read(args, span) else {
                            return Ty::Unknown;
                        };
                        return match named.as_str() {
                            "attribute" => Ty::Nullable(Box::new(Ty::named(&read))),
                            _ => Ty::Named {
                                name: "Vec".to_string(),
                                args: vec![Ty::named(&read)],
                                view: false,
                            },
                        };
                    }
                    args.iter().for_each(|a| {
                        self.expr(a, span);
                    });
                    if let Some(at) = witness {
                        self.expr(&config[at].value, span);
                    }
                    if named != "is" || args.len() != 1 {
                        self.s
                            .a_reflected_variant_has_two_members(&format!("{named}(…)"), span);
                        return Ty::Unknown;
                    }
                    return Ty::named("bool");
                }
                if matches!(&on, Ty::Named { name, .. } if name == ty::FIELD) {
                    // **`field.attribute(X)` and `field.attributes(X)`**
                    // (ADR-331 D6): `X` is a type, not a value, so it is not
                    // walked as one.
                    let named = self.parsed.text(*method).to_string();
                    if matches!(named.as_str(), "attribute" | "attributes") {
                        let Some(read) = self.an_attribute_read(args, span) else {
                            return Ty::Unknown;
                        };
                        return match named.as_str() {
                            "attribute" => Ty::Nullable(Box::new(Ty::named(&read))),
                            _ => Ty::Named {
                                name: "Vec".to_string(),
                                args: vec![Ty::named(&read)],
                                view: false,
                            },
                        };
                    }
                    args.iter().for_each(|a| {
                        self.expr(a, span);
                    });
                    if let Some(at) = witness {
                        self.expr(&config[at].value, span);
                    }
                    let named = self.parsed.text(*method).to_string();
                    if named != "of" {
                        self.s
                            .a_reflected_field_has_two_members(&format!("{named}(…)"), span);
                        return Ty::Unknown;
                    }
                    return match &self.s.flow.unrolling {
                        Some(unrolling) => unrolling.field.ty.clone(),
                        None => Ty::Unknown,
                    };
                }
                if let Ty::Nullable(_) = &on {
                    let name = self.parsed.text(*method).to_string();
                    self.s.reaches_into_a_nullable(&on, "method", &name, span);
                    // The arguments are still walked: a mistake inside one is a
                    // mistake whatever is wrong with the receiver.
                    args.iter().for_each(|a| {
                        self.expr(a, span);
                    });
                    if let Some(at) = witness {
                        self.expr(&config[at].value, span);
                    }
                    return Ty::Unknown;
                }
                // **The witness door, now that the receiver is typed**
                // ([ADR-281](../../docs/specification/adr/adr-281.md) D26).
                // `kasse.set(neu; after: stand)` is `Locked::set(after)` in the
                // ledger — a key no program can write as a method name, which
                // is what keeps D5's *one door* one (`NK2208` is what a program
                // that tries meets). The witness joins the arguments, so
                // everything a call is checked for reaches it through the one
                // path: the fit against `$T`, the lend, the stamp.
                //
                // **Before `NK2203`**, which names the entry it refuses and
                // would otherwise name `set` for a call that is not one.
                let door = witness
                    .filter(|_| locked_content_of(&on).is_some())
                    .map(|at| &config[at].value);
                if let (Some(at), None) = (witness, door) {
                    // Not a lock after all, so it was an option like any other
                    // and the walk above owed it a visit.
                    self.expr(&config[at].value, span);
                }
                let given: Vec<Expr>;
                let (args, written) = match door {
                    Some(seen) => {
                        self.s.the_witness_takes_no_reference(seen, span);
                        self.s.facts.witnessed_sets.insert(span.at() as i64);
                        given = args.iter().chain([seen]).cloned().collect();
                        (&given[..], "set(after)".to_string())
                    }
                    None => (&args[..], self.parsed.text(*method).to_string()),
                };
                // **A `set` given a stamped value** (D4). The
                // rule wants the receiver's **name** for its message and the
                // argument's **type** for its answer, and those are known in
                // two different places — so the name is carried the way the
                // door flags above are, and the rule itself runs in `call_on`
                // where the arguments have been typed.
                let outer_receiver = std::mem::replace(
                    &mut self.s.set_receiver,
                    match receiver.as_ref() {
                        Expr::Variable(name, _) => Some(self.parsed.text(*name).to_string()),
                        _ => Some("this lock".to_string()),
                    },
                );
                // The same carrying, for D2's once-only rule, and `None` where
                // the receiver is a temporary.
                let outer_named = std::mem::replace(
                    &mut self.s.flow.receiver_name,
                    match receiver.as_ref() {
                        Expr::Variable(name, _) => Some(self.parsed.text(*name).to_string()),
                        _ => None,
                    },
                );
                self.s.a_door_that_is_not_written(&on, &written, span);
                // **`NK2203`, the method half**: which entry `other.get()` goes
                // to is the type checker's answer (ADR-288), so it is asked
                // here where the receiver's type is in hand.
                if self.s.inside_a_door
                    && let Ty::Named { name, .. } = &on
                {
                    let key = format!("{name}::{written}");
                    if let Some((key, contract)) = self.method(&key) {
                        let (key, holds) = (key.clone(), contract.touches_a_lock);
                        self.s.a_lock_inside_a_lock(&key, &holds, span);
                    }
                }
                // **And a method on a mapping reads the file**
                // ([ADR-281](../../docs/specification/adr/adr-281.md) D35):
                // `mapped.lines()` walks pages that are not there yet.
                self.s
                    .io_inside_a_door(self.world, &on, &format!("`{written}`"), span);
                // **`update`'s block is a write door's**
                // ([ADR-281](../../docs/specification/adr/adr-281.md) D12), and
                // that is where a parameter without `mut` is refused. Set
                // around the call and restored after it, so a lambda written
                // inside the block is not one.
                let at_a_door =
                    self.parsed.text(*method) == "update" && locked_content_of(&on).is_some();
                if at_a_door && let Some(Expr::Closure { params, body, .. }) = args.last() {
                    self.s.an_update_block_returns_nothing(body, span);
                    self.an_update_block_reads_what_it_writes(params, body, span);
                }
                let outer_door = std::mem::replace(&mut self.s.flow.at_a_write_door, at_a_door);
                // **And `access` holds one open too** (ADR-281 D10): `get` and
                // `set` do not, because no code of the program's runs while the
                // lock is open in either.
                let holding = matches!(self.parsed.text(*method), "update" | "access")
                    && locked_content_of(&on).is_some();
                // Kept for the stamp below, since `call_on` takes `on` by value.
                let on_for_the_stamp = on.clone();
                let outer_inside = std::mem::replace(&mut self.s.inside_a_door, holding);
                // **D3**: a method that changes its subject, called on a
                // parameter. Asked of the `mutates` column (D3's own, recorded
                // because the ledger's `&T` cannot spell `&mut`), and only
                // where **every** candidate for the name agrees — one that does
                // not is a name this compiler cannot resolve, and refusing on
                // it would refuse a correct program (C.4).
                if let Some(root) = self.s.rooted_at(self.world, receiver) {
                    let name = self.parsed.text(*method);
                    let candidates: Vec<_> = self
                        .world
                        .own
                        .candidates(name)
                        .into_iter()
                        .chain(self.world.library.candidates(name))
                        .collect();
                    let changes = !candidates.is_empty()
                        && candidates.iter().all(|(_, contract)| contract.mutates);
                    if changes {
                        self.s.a_changed_binding_says_mut(
                            &root,
                            &format!("changed here, by `{name}`"),
                            *span,
                        );
                        self.s.a_shared_name_changed_in_parallel(&root, span);
                        // **An element a call changes is a place** (0.0.250):
                        // `by_bucket[b].push(x)` writes through the index, and
                        // the read `*get(…)` it was written as cannot be
                        // changed - `rustc` about the generated file.
                        // **And so is a field of one** (#526):
                        // `items[1].tags.push(x)` changes the element too.
                        if an_element_beneath(receiver) {
                            self.s.facts.changed_elements.insert(value_node(receiver));
                        }
                    }
                }
                self.s
                    .a_copy_under_another_name(self.world, receiver, *method, args, span);
                // **The text form of text is the text** (ADR-282 D8): a view
                // stays a view and a literal stays a literal, and whoever keeps
                // it asks for text of its own as anywhere else - by building a
                // literal there, or by a `.clone()` the program writes.
                let text = matches!(&on, Ty::Named { name, args: none, .. }
                    if none.is_empty() && matches!(name.as_str(), "String" | "str"));
                if text && self.parsed.text(*method) == "find" {
                    self.s.facts.text_finds.insert(value_node(receiver));
                }
                if text
                    && args.is_empty()
                    && self.parsed.text(*method) == "to_string"
                    && self.world.own.candidates("to_string").is_empty()
                {
                    self.s.facts.text_as_is.insert(value_node(receiver));
                    self.s.flow.receiver_name = outer_named;
                    self.s.flow.at_a_write_door = outer_door;
                    self.s.inside_a_door = outer_inside;
                    return on;
                }
                // **A field that holds a function is called as one**:
                // `button.on_click(4)`, where `on_click` is a field and no
                // method has the name.
                if let Some(result) =
                    self.a_called_field(&on, receiver, *method, args, outer_inside, span)
                {
                    self.s.flow.receiver_name = outer_named;
                    self.s.flow.at_a_write_door = outer_door;
                    self.s.inside_a_door = outer_inside;
                    return result;
                }
                // **A copy of a slice is a list** (ADR-282 D9): `to_owned`
                // below, where `.clone()` would copy the reference.
                let a_run = match &on {
                    // What a parameter declared `ref Array[T]` is (ADR-184 D3).
                    Ty::Pointed {
                        item, slice: true, ..
                    } => Some((**item).clone()),
                    Ty::Named {
                        name,
                        args: run,
                        view: true,
                    } if name == ty::ARRAY && run.len() == 1 => Some(run[0].clone()),
                    _ => None,
                };
                if let (Some(item), "clone", true) =
                    (&a_run, self.parsed.text(*method), args.is_empty())
                {
                    self.s.facts.owned_copies.insert(value_node(receiver));
                    self.s.flow.receiver_name = outer_named;
                    self.s.flow.at_a_write_door = outer_door;
                    self.s.inside_a_door = outer_inside;
                    return Ty::Named {
                        name: "Vec".to_string(),
                        args: vec![item.clone()],
                        view: false,
                    };
                }
                self.s.flow.last_resolved = None;
                self.s.flow.receiver_from_text = self.made_from_text(receiver);
                let value = self.call_on(on, receiver, *method, args, &written, span);
                self.s.flow.receiver_from_text = false;
                // **A copy of `std`'s is a copy whatever the receiver is**
                // (ADR-293 D24): below, `.clone()` of a view is the view.
                if let Some(key) = self.s.flow.last_resolved.take() {
                    let library = key.ends_with("::clone")
                        && self.world.library.functions.contains_key(&key)
                        && !self.world.own.functions.contains_key(&key);
                    if library && args.is_empty() {
                        self.s.facts.owned_copies.insert(value_node(receiver));
                    }
                    // **A list of text asked for a view of text** (#125): the
                    // name is a `&str` below, and what the list's own
                    // `contains` takes is a `&String`, so `rustc` refused the
                    // file - and the way round it written in Nikaia was a copy
                    // of the name (`ref name.clone()`), which ADR-283 D3 says
                    // is never the program's to need.
                    //
                    // **And an owned text in a list of views** (#375):
                    // `NAMES.contains(name)` for a `comptime NAMES:
                    // Array[ref String, N]` and a `name: String`, where the
                    // array's own `contains` takes a `&&str`. Element by element
                    // answers every pairing of `String` and `str`.
                    if (key == "Vec::contains" || key == "Array::contains")
                        && let [Expr::Variable(name, _)] = args
                        && self
                            .s
                            .binding(self.parsed.text(*name))
                            .is_some_and(|local| {
                                local.ty == Ty::view("str") || local.ty == Ty::named("String")
                            })
                    {
                        self.s.facts.text_in_lists.insert(value_node(receiver));
                    }
                    // **A sequence's count is an `i64`** (Part I 2.2): the
                    // emitter writes the conversion a length has.
                    let counts = [ty::SEQ, ty::PAR]
                        .iter()
                        .any(|of| key == format!("{of}::count"));
                    if counts && !self.world.own.functions.contains_key(&key) {
                        self.s.facts.counted.insert(value_node(receiver));
                    }
                }
                self.s.flow.receiver_name = outer_named;
                self.s.flow.at_a_write_door = outer_door;
                self.s.inside_a_door = outer_inside;
                self.s.set_receiver = outer_receiver;
                // **A walk over views of values that copy yields the values**
                // ([ADR-231](../../docs/specification/adr/adr-231.md) D1): a
                // view of an `i64` is a reference below, and a number read
                // through one compared with a number was `rustc`'s error.
                // Copying one costs less than pointing at it.
                let value = match value {
                    Ty::Seq {
                        item,
                        is_sync,
                        pauses,
                        can_throw: throws,
                        parallel,
                        shape,
                    } if copies_as_a_view(&item) => {
                        self.s.facts.copied_walks.insert(value_node(expr));
                        Ty::Seq {
                            item: Box::new(unviewed(&item)),
                            is_sync,
                            pauses,
                            can_throw: throws,
                            parallel,
                            shape,
                        }
                    }
                    other => other,
                };
                // **`filter` is handed a reference to the item** below
                // (ADR-231 D2); where the item copies, the parameter takes it
                // by pattern and is the item.
                if self.parsed.text(*method) == "filter"
                    && let (Ty::Seq { item, .. }, [lambda @ Expr::Closure { params, .. }]) =
                        (&on_for_the_stamp, args)
                    && params.len() == 1
                    && !item.is_unknown()
                    && (item.is_a_view() || !crate::contracts::keeps::moves(item))
                {
                    self.s.facts.filter_patterns.insert(value_node(lambda));
                }
                // **And what `access` hands back came out of a lock** (D1).

                match self.parsed.text(*method) == "access"
                    && locked_content_of(&on_for_the_stamp).is_some()
                {
                    true => Ty::seen(value.unseen()),
                    false => value,
                }
            }

            Expr::Field { base, name, .. } | Expr::SafeField { base, name, .. } => {
                let on = self.expr(base, span);
                let field = self.parsed.text(*name).to_string();
                // **`x?.field`** (Part I 3.5): over a `T?` the field of what is
                // inside, as a `T?`; over a plain value the plain read, with
                // `NK1217` ([ADR-279](../../docs/specification/adr/adr-279.md)
                // D13).
                if matches!(expr, Expr::SafeField { .. }) {
                    match &on {
                        Ty::Nullable(inner) => {
                            return self.a_safe_field(inner, base, field, expr, span);
                        }
                        Ty::Unknown => return Ty::Unknown,
                        _ => self.reaches_through_a_plain_value(
                            &on,
                            base,
                            Reached::Field(&field),
                            expr,
                            span,
                        ),
                    }
                }
                self.a_read_of_a_part(base, expr, &field, span);
                if let Ty::Nullable(_) = &on {
                    self.s.reaches_into_a_nullable(&on, "field", &field, span);
                    return Ty::Unknown;
                }
                // **A field of one member is reached through the `match`**
                // (ADR-280 D30).
                if let Some(members) = several_errors(&on) {
                    let members = members_named(members);
                    self.s.findings.push(Finding {
                        severity: Severity::Error,
                        span: *span,
                        code: "NK1107".to_string(),
                        message: format!(
                            "This is an error, one of {members}, and an error has no field `{field}`."
                        ),
                        notes: vec![
                            "A field belongs to one of them, and which one it is is said by a \
                             `match`."
                                .to_string(),
                        ],
                        help: Some(format!(
                            "Take it apart first: `match error {{ … }}`, and read `{field}` in \
                             the arm of the type that has it."
                        )),
                        labels: Vec::new(),
                    });
                    return Ty::Unknown;
                }
                // **A tuple's part is the type written at its position**:
                // `params[i].0` over a `Vec[(String, i64)]` is a `String`.
                // It was nothing, so a comparison with a lent `String` could
                // not tell which side was the value, and `rustc` answered
                // *can't compare `String` with `&String`* about a generated
                // file - found moving the compiler's `views` into Nikaia
                // (#125).
                if let Ty::Tuple(parts) = &on {
                    // **A position the tuple does not have is `NK1107`** (#515):
                    // `t.2` on a pair reached `rustc` as *no field `2`*.
                    if let Ok(at) = field.parse::<usize>()
                        && at >= parts.len()
                    {
                        let last = parts.len().saturating_sub(1);
                        self.s.findings.push(Finding {
                            severity: Severity::Error,
                            span: *span,
                            code: "NK1107".to_string(),
                            message: format!("This tuple has no part `.{at}`."),
                            notes: vec![format!(
                                "A tuple's parts are counted from 0, and this one has {}.",
                                parts.len()
                            )],
                            help: Some(match parts.is_empty() {
                                true => "There's nothing in it to read.".to_string(),
                                false => format!("Use one of `.0` to `.{last}`."),
                            }),
                            labels: Vec::new(),
                        });
                        return Ty::Unknown;
                    }
                    return field
                        .parse::<usize>()
                        .ok()
                        .and_then(|at| parts.get(at).cloned())
                        .unwrap_or(Ty::Unknown);
                }
                // **A type that is seen through answers with what it holds**
                // (#497): `db.host` on a `Shared[Connection]` is the
                // connection's field, as a method on it already is.
                let on = match &on {
                    Ty::Named { name, .. } if self.s.fields_of(self.world, name).is_none() => {
                        self.seen_through(&on).unwrap_or(on)
                    }
                    _ => on,
                };
                let Ty::Named { name: ty, .. } = &on else {
                    return Ty::Unknown;
                };
                // **A boxed field is read through its box**
                // ([ADR-246](../../docs/specification/adr/adr-246.md) D2).
                if self
                    .s
                    .facts
                    .boxed_members
                    .get(ty)
                    .is_some_and(|members| members.contains(&field))
                {
                    self.s.facts.boxed_reads.insert(value_node(expr));
                }
                // **A handle has no fields** (ADR-302 D7): it is an address
                // this language never dereferences, so there is nothing inside
                // it to name. Before `fields_of`, which would answer `None` and
                // send the reader to `NK1126`'s *no bound* — a sentence about a
                // type parameter, which this is not.
                if self.s.opaque_handles.contains(ty) {
                    let ty = ty.clone();
                    self.s
                        .a_handle_has_nothing_inside(&ty, &format!("the field `{field}`"), span);
                    return Ty::Unknown;
                }
                // **A reflected field answers two members and no others**
                // ([ADR-304](../../docs/specification/adr/adr-304.md) D2):
                // `.name` here, and `.of(value)` where a method is called.
                if ty == ty::VARIANT {
                    if field == "name" {
                        return Ty::view("str");
                    }
                    let held = field.clone();
                    self.s.a_reflected_variant_has_two_members(&held, span);
                    return Ty::Unknown;
                }
                if ty == ty::FIELD {
                    if field == "name" {
                        return Ty::Named {
                            name: "str".to_string(),
                            args: Vec::new(),
                            view: true,
                        };
                    }
                    // **`field.default`, its default as a `T?`** (ADR-331
                    // D5): `null` where it has none.
                    if field == "default" {
                        return match &self.s.flow.unrolling {
                            Some(unrolling) => Ty::Nullable(Box::new(unrolling.field.ty.clone())),
                            None => Ty::Unknown,
                        };
                    }
                    let held = field.clone();
                    self.s.a_reflected_field_has_two_members(&held, span);
                    return Ty::Unknown;
                }
                let Some(fields) = self.s.fields_of(self.world, ty) else {
                    // `NK1126` where the type is a parameter: nothing will ever
                    // describe `T`, so a field on one is refused here rather
                    // than by `rustc` about the generated file.
                    let ty = ty.clone();
                    self.s
                        .nothing_says_what_a_parameter_can_do(&ty, "field", &field, span);
                    return Ty::Unknown;
                };
                match fields.iter().find(|f| f.name == field) {
                    Some(found) => {
                        let (ty, declared) = (ty.clone(), found.clone());
                        self.s.field_is_reachable(&ty, &declared, span);
                        // `Pair[i64].first` is an `i64`: the receiver's own
                        // arguments bind the declaration's parameters, exactly
                        // as a method's receiver binds its signature's
                        // (ADR-288, and ADR-295 D2 for a `.nika` declaration).
                        ty::substitute(&declared.ty, &self.arguments_of(&on))
                    }
                    None => {
                        let ty = ty.clone();
                        self.s.no_such_field(&ty, &field, &fields, span);
                        Ty::Unknown
                    }
                }
            }

            Expr::StructLit { name, fields, .. } => {
                // `unaliased`, the same as a type: `h::Request(path: …)` builds
                // `http::Request` (ADR-286 D12).
                let name = self.parsed.unaliased(self.parsed.text(*name));
                let declared = self.s.fields_of(self.world, &name);
                // **A struct literal naming nothing this compiler declares**
                // ([ADR-096](../../docs/specification/adr/adr-096.md), `NK1135`),
                // and it is the silence that let
                // [ADR-133](../../docs/specification/adr/adr-133.md)'s collision
                // through: `Stats(min: first)` and `execute(target_age: 30)` are
                // one spelling, `ctor_lit` reads both as a literal, and a literal
                // for a struct nothing declares walked out of here on the
                // `continue` below and lowered verbatim. `rustc` answered
                // *cannot find struct `execute`* about a file nobody wrote, which
                // is Part III C.1's class and the very hole `NK1135` was built
                // to close for a written annotation.
                //
                // **Unqualified only**, which is `NK1135`'s own convention:
                // `pool::Conn(id: 1)` names a package's type, and whether this
                // build can see that package is a question with a message of its
                // own (ADR-286 D11).
                if declared.is_none() && !name.contains("::") && !self.declares_a_type(&name) {
                    self.s.a_struct_nothing_declares(self.world, &name, span);
                }
                // **What a generic struct's literal binds**
                // ([ADR-295](../../docs/specification/adr/adr-295.md) D2).
                // `Pair { first: 1, second: 2 }` is a `Pair[i64]` and nothing
                // else says so: the declaration writes `first: $T` and the
                // value is an `i64`, which is one `bind` per field.
                let mut bound: BTreeMap<String, Ty> = BTreeMap::new();
                // **A field named twice** is `NK1172`, here and in a `with` for
                // one reason ([ADR-118](../../docs/specification/adr/adr-118.md)
                // D1 restates the literal's rule): `Point { x: 1, x: 2 }` used
                // to lower, and `rustc` answered about the generated file.
                let mut seen: BTreeSet<String> = BTreeSet::new();
                let mut misspelled = false;
                for init in fields {
                    let field = self.parsed.text(init.name).to_string();
                    if !seen.insert(field.clone()) {
                        self.s.a_field_written_twice(&name, &field, span);
                    }
                    // **Code put into a function field is remembered with the
                    // field** (ADR-230 D1): what it calls is what a call
                    // through the field reaches, wherever that call is.
                    let code_field = declared.as_ref().is_some_and(|d| {
                        d.iter()
                            .any(|f| f.name == field && matches!(f.ty, Ty::Fn { .. }))
                    });
                    if code_field {
                        self.s.stored_frames.push(StoredCode::empty());
                    }
                    // `Reading { name, temp }` is shorthand for `name: name`.
                    if let Some(value) = &init.value {
                        self.a_mut_parameter_given_away(value, span, "put into a field");
                    }
                    if let Some(Expr::Closure { .. }) = &init.value {
                        self.s.flow.lambda_expected = declared
                            .as_ref()
                            .and_then(|d| d.iter().find(|f| f.name == field))
                            .map(|f| f.ty.clone());
                    }
                    let found = match &init.value {
                        Some(value) => self.expr(value, span),
                        None => self.s.lookup(&field).unwrap_or(Ty::Unknown),
                    };
                    self.s.flow.lambda_expected = None;
                    if code_field {
                        let mut stored =
                            self.s.stored_frames.pop().unwrap_or_else(StoredCode::empty);
                        match &init.value {
                            Some(Expr::Closure { body, .. }) => {
                                stored
                                    .free
                                    .extend(crate::contracts::locks::free_calls(self.parsed, body));
                            }
                            // A function of this program, named: it is what
                            // runs.
                            Some(Expr::Variable(named, _)) => {
                                let named = self.parsed.text(*named).to_string();
                                match self.s.lookup(&named).is_none() {
                                    true => {
                                        stored.free.insert(named);
                                    }
                                    false => stored.unresolved = true,
                                }
                            }
                            // Anything else - a value computed somewhere else -
                            // is code this walk did not see.
                            _ => stored.unresolved = true,
                        }
                        let entry = self
                            .s
                            .stored_code
                            .entry(format!("{name}.{field}"))
                            .or_insert_with(StoredCode::empty);
                        entry.resolved.extend(stored.resolved);
                        entry.unresolved |= stored.unresolved;
                        entry.free.extend(stored.free);
                    }
                    let Some(declared) = &declared else { continue };
                    match declared.iter().find(|f| f.name == field) {
                        Some(found_field) => {
                            let want = found_field.ty.clone();
                            ty::bind(&want, &found, &mut bound);
                            // **A field holds what it is given** (ADR-293 D29),
                            // unless it is declared a view.
                            if !want.is_a_view() {
                                let to = format!("stored in `{name}.{field}`");
                                match &init.value {
                                    Some(value) => self.hands_over(value, &found, &to, span),
                                    None => self.s.hands_over_name(self.world, &field, &to, span),
                                }
                            }
                            let owner = name.clone();
                            self.s.field_is_reachable(&name, found_field, span);
                            // **No longer a constructor either**
                            // ([ADR-281](../../docs/specification/adr/adr-281.md)
                            // D2). It was the second of the two positions, and the
                            // two positions are what stopped being a list.
                            let _ = &owner;
                            // **A declared field is a use** (ADR-152 D4), and
                            // it is the position [ADR-284](../../docs/specification/adr/adr-284.md)
                            // §4's C value field will be written through.
                            let found = match init.value.as_ref() {
                                Some(value) => self
                                    .literal_by_use(&found, &want, value, span)
                                    .unwrap_or(found),
                                None => found,
                            };
                            // Part I 2.3's fourth position: a plain value in a
                            // field the struct declares nullable. The same rule
                            // as the other three (`wraps_into_nullable`), keyed
                            // by the field as well, because a struct literal
                            // has one of these per field and a statement only
                            // one span.
                            // **A run this body owns, in a field that views
                            // one** ([ADR-179](../../docs/specification/adr/adr-179.md)
                            // D2). `Vec[T]` *fits* `&[T]` — that is the C
                            // boundary's rule, where the call lends for its own
                            // duration ([ADR-302](../../docs/specification/adr/adr-302.md)
                            // D1) — and a **struct outlives the expression that
                            // fills it**, so the same fit here is a view of
                            // something already gone. Asked before `expect`,
                            // because `expect` is silent where the fit holds.
                            //
                            // **Inside a grammar action anything that is not
                            // already a view is refused**, and that is not
                            // caution: an action's bindings come from the
                            // parse, which **owns** the runs it built, and what
                            // this checker knows about one of them is nothing
                            // — so a `Section { name, settings }` shorthand
                            // would slip past a test on the value's type and
                            // reach `rustc` as *expected `&[Setting]`, found
                            // `Vec<Setting>`*, about a grammar line whose way
                            // out is not *write a `&`* at all.
                            //
                            // **And not inside a `comptime`**, which is the
                            // position this type was added for: there the run
                            // is **the build's**, the crossing writes the `&`
                            // and the array literal behind it is the program's
                            // own text ([ADR-311](../../docs/specification/adr/adr-311.md)
                            // D1). Refusing there would refuse the one line
                            // that is right, which is [Part III
                            // C.4](../../docs/specification/30-nikaia-tooling.md).
                            let owns_it = matches!(&found, Ty::Named { name, view: false, .. }
                                if name == "Vec" || name == "List")
                                || (self.s.inside_an_action.is_some() && !found.is_a_view());
                            if slice_element(&want).is_some()
                                && owns_it
                                && !self.s.flow.inside_a_comptime
                            {
                                self.s
                                    .a_run_this_body_owns(&name, &field, &found.text(), span);
                                continue;
                            }
                            let value = init.value.as_ref();
                            let keeper = format!("`{owner}` keeps its `{field}` after this line");
                            if let Some(how) = self.widened_slot(&found, &want, value, span) {
                                self.s
                                    .facts
                                    .nullable_fields
                                    .insert((value_node(expr), field.clone()), how);
                                // A view into a `String?` is still a view kept
                                // (ADR-282 D12): asked as it would be without
                                // the `?`.
                                if views_into_nullable_text(&found, &want) {
                                    self.s.flow.field_either = self.either_field(&owner, &field);
                                    let (o, f) = (owner.clone(), field.clone());
                                    self.expect_kept(
                                        &found,
                                        &want,
                                        value,
                                        &keeper,
                                        *span,
                                        "field",
                                        move |found, want| {
                                            format!("`{o}.{f}` holds `{want}`, but you're giving it `{found}`.")
                                        },
                                    );
                                }
                                continue;
                            }
                            self.s.flow.field_either = self.either_field(&owner, &field);
                            self.expect_kept(
                                &found,
                                &want,
                                value,
                                &keeper,
                                *span,
                                "field",
                                move |found, want| {
                                    format!("`{owner}.{field}` holds `{want}`, but you're giving it `{found}`.")
                                },
                            );
                        }
                        None => {
                            misspelled = true;
                            self.s.no_such_field(&name, &field, declared, span)
                        }
                    }
                }
                // **A field left out takes its default, and one without a
                // default may not be left out** (ADR-331 D5, `NK1234`).
                // Not after a field it does not have: that is most likely the
                // one it misspelled, and said once.
                if let Some(declared) = declared.as_ref().filter(|_| !misspelled) {
                    let missing: Vec<String> = declared
                        .iter()
                        .filter(|f| f.default.is_empty() && !seen.contains(&f.name))
                        .map(|f| f.name.clone())
                        .collect();
                    if !missing.is_empty() {
                        self.s.fields_left_out(&name, &missing, span);
                    }
                }
                // **A variant's literal is its `enum`** and not the variant:
                // `Shape` is a type a declaration can be written as and
                // `Shape::Spot` is not, so answering the latter refused a
                // correct program with a way out nobody can take (Part III
                // C.4). The map is built where the `enum` is read, so the
                // literal and the pattern agree by construction.
                if let Some(owner) = self.s.variant_owner.get(&name) {
                    return Ty::named(owner.clone());
                }
                match self.s.struct_parameters.get(&name) {
                    Some(order) => {
                        let args = order
                            .iter()
                            .map(|p| bound.get(p).cloned().unwrap_or(Ty::Unknown))
                            .collect();
                        Ty::Named {
                            name,
                            args,
                            view: false,
                        }
                    }
                    None => Ty::named(name),
                }
            }

            // **`value with { field: … }`**
            // ([ADR-118](../../docs/specification/adr/adr-118.md) D1): a copy of
            // a value with named fields changed, of the same type. Every rule
            // about *naming* a field is the literal's — the braces are the
            // literal's — so what is new here is the **operand**: it has to be
            // a struct this compiler can name, because what it lowers to is
            // Rust's `Point { x: 1, ..p }` and the type is written there.
            Expr::With { base, fields, .. } => {
                let found = self.expr(base, span);
                let Ty::Named {
                    name,
                    view,
                    args: _,
                } = &found
                else {
                    self.s
                        .a_with_over_something_else(&found.text(), &Copyable::Unnamed, span);
                    return Ty::Unknown;
                };
                let name = self.parsed.unaliased(name);
                // **An enum is refused with a message naming `match`**
                // (ADR-118 §4): `m with { x: 1 }` cannot be typed without
                // knowing the variant, and inside a `match` arm it is known —
                // which is a decision that record deliberately left open.
                if self.s.enums.contains_key(&name) {
                    self.s
                        .a_with_over_something_else(&name, &Copyable::AnEnum, span);
                    return found;
                }
                // **A view is not something to move from** (D3): what `with`
                // does not name it takes from the operand *by move*, and no
                // copy is inserted that the program did not write (ADR-282 D7).
                if *view {
                    self.s
                        .a_with_over_something_else(&name, &Copyable::AView, span);
                    return Ty::named(name);
                }
                let Some(declared) = self.s.fields_of(self.world, &name) else {
                    self.s
                        .a_with_over_something_else(&name, &Copyable::NotAStruct, span);
                    return found;
                };
                // **A copy that changes nothing is the value** (D1).
                if fields.is_empty() {
                    self.s.a_with_that_changes_nothing(&name, span);
                }
                let mut seen: BTreeSet<String> = BTreeSet::new();
                for init in fields {
                    let field = self.parsed.text(init.name).to_string();
                    if !seen.insert(field.clone()) {
                        self.s.a_field_written_twice(&name, &field, span);
                    }
                    // `user with { name }` — the shorthand is the literal's.
                    let given = match &init.value {
                        Some(value) => self.expr(value, span),
                        None => self.s.lookup(&field).unwrap_or(Ty::Unknown),
                    };
                    // **Code put into a function field here is code the lock
                    // column does not read** (ADR-230 D1): the field is
                    // undecided rather than quiet.
                    if declared
                        .iter()
                        .any(|f| f.name == field && matches!(f.ty, Ty::Fn { .. }))
                    {
                        self.a_field_whose_code_is_not_seen(&name, &field);
                    }
                    match declared.iter().find(|f| f.name == field) {
                        Some(held) => {
                            // **D4: a field may be named only where a literal
                            // could name it.** The private field a copy merely
                            // *carries* is never named, so it never reaches
                            // this.
                            self.s.field_is_reachable(&name, held, span);
                            let want = held.ty.clone();
                            let owner = name.clone();
                            let field = field.clone();
                            self.expect(&given, &want, *span, "field", move |given, want| {
                                format!("`{owner}.{field}` holds `{want}`, but you're giving it `{given}`.")
                            });
                        }
                        None => self.s.no_such_field(&name, &field, &declared, span),
                    }
                }
                // **The type, for the emitter** (ADR-296 D17): Rust writes the
                // struct's name in a functional update and this node does not
                // carry one, so the answer travels under the byte the `with`
                // stands at rather than being worked out twice.
                self.s
                    .facts
                    .with_types
                    .insert(value_node(expr), name.clone());
                found
            }

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
                        self.expect_bool(&left, span, "`&&` and `||` join two `bool`s.");
                        self.expect_bool(&right, span, "`&&` and `||` join two `bool`s.");
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

            Expr::Cast { expr, ty, .. } => {
                let from = self.expr(expr, span);
                let into = self.declared(ty, span);
                self.a_cast_over_a_lent_binding(expr, span);
                if let Ty::Named { name, .. } = &into
                    && !OFFERED.contains(&name.as_str())
                {
                    self.s.cast_names_a_foreign_type(name, span);
                }
                self.record_cast(&from, &into, span);
                if matches!(&from, Ty::Named { name, .. } if name == "char" || name == "scalar")
                    && matches!(&into, Ty::Named { name, .. } if INTEGERS.contains(&name.as_str()))
                {
                    self.s.facts.char_codes.insert(value_node(expr));
                }
                into
            }

            Expr::Tuple(parts, _) => {
                let found: Vec<Ty> = parts.iter().map(|p| self.expr(p, span)).collect();
                for (part, ty) in parts.iter().zip(&found) {
                    self.hands_over(part, ty, "put into a tuple", span);
                }
                Ty::Tuple(found)
            }

            // **`[1, 2, 3]` is a `Vec[T]`**
            // ([ADR-135](../../docs/specification/adr/adr-135.md) D1), and `T`
            // is what the elements agree on.
            Expr::ListLit { items, .. } => self.list_literal(items, span),

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
                        .is_some_and(|root| self.s.lent_here(&root));
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
            Expr::Index { base, index, .. } => {
                let writing = std::mem::replace(&mut self.s.flow.writing_index, false);
                let on = self.expr(base, span);
                let key = self.expr(index, span);
                // **An index of a mapping is a page fault**
                // ([ADR-281](../../docs/specification/adr/adr-281.md) D35).
                self.s.io_inside_a_door(self.world, &on, "this index", span);
                // A type seen through is indexed as what it holds (#497):
                // `data[0..<7]` on an `fs::Mapped` is a slice of its text.
                let on = match self.seen_through(&on) {
                    Some(held) if held.unseen() != on.unseen() => held,
                    _ => on,
                };
                // **A view of a run is indexed as the run is** (ADR-179 D1,
                // ADR-293 D23): `xs[i]` for an `xs: ref Array[T]` is a `T`, and a
                // range in the brackets is a run of it again. It was `?`, so a
                // method called on an element was one no ledger describes, and
                // the function around it became a future (found moving `Ty::fits`
                // into Nikaia, ADR-294).
                if let Ty::Pointed {
                    item,
                    slice: true,
                    mutable,
                } = &on
                {
                    if matches!(&**index, Expr::Range { .. }) {
                        return Ty::Pointed {
                            item: item.clone(),
                            slice: true,
                            mutable: *mutable,
                        };
                    }
                    self.number_asked(index, &Ty::named("i64"), span, false);
                    self.s.facts.list_indices.insert(value_node(base));
                    return (**item).clone();
                }
                let Ty::Named { name, args, .. } = &on else {
                    return Ty::Unknown;
                };
                // **And no indexing** (ADR-302 D7), for the same reason: a
                // handle is one address and not a run of anything.
                if self.s.opaque_handles.contains(name) {
                    let name = name.clone();
                    self.s.a_handle_has_nothing_inside(&name, "an index", span);
                    return Ty::Unknown;
                }
                // The **last segment**, because a `std` type carries its module
                // since [ADR-313](../../docs/specification/adr/adr-313.md) D3 and
                // what is indexed is the type rather than where it is reached
                // from.
                // **A range in the brackets is a slice** (ADR-293 D23): a run of
                // the list, `ref Array[T]` - the type a function writes to take
                // one - or a view of the text. Written there, or kept in a name.
                let a_range = matches!(&**index, Expr::Range { .. })
                    || matches!(&key, Ty::Seq { shape, .. } if shape.replays);
                if a_range && !matches!(&**index, Expr::Range { .. }) {
                    self.s.facts.slice_indices.insert(value_node(index));
                }
                // **An index is a use that prefers an `i64`**
                // ([ADR-285](../../docs/specification/adr/adr-285.md) D24): a
                // length and an index are one (ADR-285 D1), and the language
                // below takes any integer through `index::at` - so it is asked
                // only where nothing else asks.
                if !a_range && matches!(crate::contracts::ty::base(name), "Vec" | "List" | "Array")
                {
                    self.number_asked(index, &Ty::named("i64"), span, false);
                }
                if a_range {
                    return match (crate::contracts::ty::base(name), args.as_slice()) {
                        ("Vec" | "List" | "Array", [item, ..]) => Ty::Named {
                            name: ty::ARRAY.to_string(),
                            args: vec![item.clone()],
                            view: true,
                        },
                        ("String" | "str", []) => Ty::view("str"),
                        // **A cut of a `Bytes` is a `Bytes`** (ADR-320 D7).
                        ("Bytes", []) => Ty::named("Bytes"),
                        _ => Ty::Unknown,
                    };
                }
                match (crate::contracts::ty::base(name), args.as_slice()) {
                    // **A `Bytes` is read as a run of bytes** (ADR-320 D7,
                    // Part I 2.6): `b[i]` is a `u8`, and an index outside
                    // stops the program, as a list's does.
                    ("Bytes", []) => Ty::named("u8"),
                    // **A sequence keeps its `T` and its abort**
                    // ([ADR-293](../../docs/specification/adr/adr-293.md) D3):
                    // it has a `T` at every index it has at all, and an index
                    // it does not have is the program's own arithmetic gone
                    // wrong ([Part III A.2](../../../docs/specification/30-nikaia-tooling.md)).
                    ("Vec" | "List" | "Array", [item, ..]) => {
                        self.s.facts.list_indices.insert(value_node(base));
                        item.clone()
                    }
                    // **A map answers a `T?`** (D1). It has a value only where
                    // the key is, and *there is nothing there* is data about
                    // the world rather than a bug in the program — so the
                    // bracket says what `get` says, and `??` is how a program
                    // that knows better says so.
                    ("HashMap" | "Map" | "BTreeMap", [keys, value]) => {
                        let (keys, value) = (keys.clone(), value.clone());
                        self.s.flow.read_a_map = !writing;
                        self.a_map_key(&keys, &key, index, writing, span);
                        // **A map of `T?` reads one `T?`**
                        // ([ADR-293](../../docs/specification/adr/adr-293.md)
                        // D1): a stored `null` and an absent key are both
                        // `null`, and `contains_key` tells them apart. A write
                        // keeps the slot's own type, so `m[k] = null` stores
                        // (D2).
                        match (value, writing) {
                            (Ty::Nullable(held), false) => {
                                self.s.facts.flat_reads.insert(value_node(expr));
                                Ty::Nullable(held)
                            }
                            (value, _) => Ty::Nullable(Box::new(value)),
                        }
                    }
                    _ => Ty::Unknown,
                }
            }
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
            Expr::Range { start, end, .. } => {
                let from = self.expr(start, span);
                let to = self.expr(end, span);
                let item = match from.is_unknown() {
                    true => to,
                    false => from,
                };
                Ty::Seq {
                    item: Box::new(item),
                    is_sync: true,
                    pauses: false,
                    can_throw: false,
                    parallel: false,
                    shape: ty::Shape {
                        ends: true,
                        sized: true,
                        replays: true,
                    },
                }
            }

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

    /// `NK1102` for a type variable two integer types are handed to
    /// (`tools/check_widen.nika`).
    fn one_type_variable_two_integer_types(
        &mut self,
        key: &str,
        wanted: &[(String, Ty)],
        given: &[Expr],
        found: &[Ty],
        span: &Span,
    ) {
        self.s
            .one_type_variable_two_integer_types(self.world, key, wanted, given, found, span);
    }

    /// `x op= value` over two integer types (`tools/check_numbers2.nika`).
    fn a_compound_over_two_integer_types(
        &mut self,
        op: BinaryOp,
        target: &Expr,
        into: &Ty,
        value: &Expr,
        found: &Ty,
        span: &Span,
    ) {
        self.s
            .a_compound_over_two_integer_types(self.world, op, target, into, value, found, span);
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
        // **A template's `<for>` declares a name**, and the holes inside it use
        // it: `<for r in :rows>{r.name}</for>` (ADR-017). That is the fifth
        // thing that counts as declaring one, after the four
        // `nothing_declares_it` lists - and the one a reader of a hole cannot
        // see, because the binding is in the markup around it.
        //
        // Measured: widening `NK1117` to a name in an expression refused
        // `examples/escaping/src/main.nika` for its `{r.shade}` until this frame existed.
        //
        // What each of an f-string's holes says after its `:`, beside the holes
        // that parse - the ones the walk below is handed, in the same order.
        let specs: Vec<Option<String>> = match literal {
            // As the grammar parsed them (ADR-309 D6): every hole parses.
            Expr::LitInterpolated { parts, .. } => parts
                .iter()
                .filter_map(|part| match part {
                    crate::ast::FPart::Hole { spec, .. } => Some(spec.clone()),
                    crate::ast::FPart::Text(_) => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        // **An f-string's hole is read where it stands in the tree**, unless
        // the tier pass wraps something inside it: what is recorded against a
        // node - an unsigned literal's suffix, an index into a list - is keyed
        // by its address ([`value_node`]), and a copy made here and another
        // made by the emitter met only where the allocator happened to hand
        // back the same address (found moving `sharing`'s walk into Nikaia,
        // #125: `double(3000000000)` in a hole lost its `u32`).
        let walked: Vec<(std::borrow::Cow<'_, Expr>, LoopNames)> = match literal {
            Expr::LitInterpolated { parts, .. } => crate::emit::interpolated_holes(parts)
                .into_iter()
                .map(|(at, hole)| (crate::emit::hole_as_read(self.parsed, at, hole), Vec::new()))
                .collect(),
            _ => crate::emit::literal_expressions_bound(self.parsed, literal)
                .into_iter()
                .map(|(hole, bound)| (std::borrow::Cow::Owned(hole), bound))
                .collect(),
        };
        let mut names: Vec<bool> = Vec::new();
        // **What each collection a `<for>` walks came to** (#524): it is a hole
        // of its own, walked before the body that binds its element, so the
        // name is a view of that element, as a `for` over a list binds it.
        let mut collections: BTreeMap<String, Ty> = BTreeMap::new();
        for (index, (hole, bound)) in walked.into_iter().enumerate() {
            let hole: &Expr = &hole;
            let frame: Vec<Local> = bound
                .into_iter()
                .map(|(name, collection)| {
                    let element = match collections.get(collection.trim()) {
                        Some(Ty::Named { name, args, .. })
                            if matches!(name.as_str(), "Vec" | "Array") && args.len() == 1 =>
                        {
                            view_of(&args[0])
                        }
                        _ => Ty::Unknown,
                    };
                    self.local_free(name, element)
                })
                .collect();
            self.s.scope.push(frame);
            let held = self.expr(hole, span);
            if let Expr::Variable(name, _) = hole {
                collections.insert(self.parsed.text(*name).to_string(), held.clone());
            }
            // **A hole is formatted by reference** (ADR-279 D5): a `??` that
            // is the hole lends its left side, as a print call's argument does.
            if matches!(literal, Expr::LitInterpolated { .. }) {
                self.a_pending_coalesce_is_lent(hole);
            }
            self.s.scope.pop();
            if let Some(spec) = specs.get(index) {
                self.a_collection_in_a_hole(hole, &held, spec.as_deref(), span);
            }
            names.push(matches!(&held, Ty::Named { name, .. } if name == "fs::Path"));
        }
        // **An `f"…"` with a file's name in it is joined in the platform's
        // encoding** (ADR-319 D4), so a name that is not text keeps its bytes.
        if matches!(literal, Expr::LitInterpolated { .. }) && names.contains(&true) {
            self.s.facts.path_holes.insert(value_node(literal), names);
        }
    }

    /// **Whether a file's name is made from text** (ADR-319 D5), followed as
    /// far as this can see: a literal, a name that holds text or a name made
    /// from text, an `f"…"` whose names are, and `join` or `with_extension`
    /// over them. Anything else - a parameter, a field, what `walk` handed
    /// over - may be the operating system's, and is not.
    fn made_from_text(&self, expr: &Expr) -> bool {
        let text =
            |ty: &Ty| matches!(ty, Ty::Named { name, .. } if name == "String" || name == "str");
        match expr {
            Expr::LitStr { .. } => true,
            Expr::Variable(name, _) => {
                self.s
                    .binding(self.parsed.text(*name))
                    .is_some_and(|local| {
                        text(&local.ty)
                            || self
                                .s
                                .flow
                                .text_paths
                                .contains(&((local.id as usize) as i64))
                    })
            }
            Expr::LitInterpolated { parts, .. } => crate::emit::interpolated_holes(parts)
                .iter()
                .all(|(_, hole)| match hole {
                    Expr::Variable(name, _) => {
                        self.s
                            .binding(self.parsed.text(*name))
                            .is_some_and(|local| {
                                !matches!(&local.ty, Ty::Named { name, .. } if name == "fs::Path")
                                    || self
                                        .s
                                        .flow
                                        .text_paths
                                        .contains(&((local.id as usize) as i64))
                            })
                    }
                    Expr::LitInt { .. } | Expr::LitStr { .. } => true,
                    other => self.made_from_text(other),
                }),
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                matches!(self.parsed.text(*method), "join" | "with_extension")
                    && self.made_from_text(receiver)
                    && args.iter().all(|arg| self.made_from_text(arg))
            }
            _ => false,
        }
    }

    /// **`NK1201`, a file's name in an `f"…"` used as text** (ADR-319 D4):
    /// a name holds the platform's bytes, which need not be text, so it has no
    /// text form where the `f"…"` is text. Refused at the hole, the same at
    /// every position; whether the name is text is the program's to say,
    /// with `to_text()` or `display()`.
    fn a_name_in_text(&mut self, want: &Ty, value: Option<&Expr>, span: &Span) -> bool {
        let Some(literal @ Expr::LitInterpolated { parts, .. }) = value else {
            return false;
        };
        let wants_text = match want {
            Ty::Nullable(inner) => {
                matches!(&**inner, Ty::Named { name, .. } if name == "String" || name == "str")
            }
            Ty::Named { name, .. } => name == "String" || name == "str",
            _ => false,
        };
        if !wants_text {
            return false;
        }
        let Some(names) = self.s.facts.path_holes.get(&value_node(literal)).cloned() else {
            return false;
        };
        let holes = crate::emit::interpolated_holes(parts);
        for ((_, hole), is_a_name) in holes.iter().zip(names) {
            if !is_a_name {
                continue;
            }
            let written = written(self.parsed, hole);
            let shown = match hole {
                Expr::Variable(_, _) | Expr::Field { .. } => written.clone(),
                _ => format!("({written})"),
            };
            self.s.findings.push(Finding {
                severity: Severity::Error,
                span: *span,
                code: "NK1201".to_string(),
                message: format!(
                    "You can't put the file name `{written}` in an `f\"…\"` that is text."
                ),
                notes: vec![
                    "A hole is written as its value's text, and a file name holds the \
                     platform's bytes, which need not be text. Here the `f\"…\"` is used \
                     as text."
                        .to_string(),
                ],
                help: Some(format!(
                    "Write `{{{shown}.to_text()}}` to use it as text where it is text, or \
                     `{{{shown}.display()}}` to show it."
                )),
                labels: Vec::new(),
            });
        }
        true
    }

    /// `NK1201` (`tools/check_exprs.nika`).
    fn a_collection_in_a_hole(&mut self, hole: &Expr, held: &Ty, spec: Option<&str>, span: &Span) {
        self.s
            .a_collection_in_a_hole(self.world, hole, held, spec, span);
    }

    /// `f(a, b)`, `Stats(first)`, `io::read_to_string()`, `write(p, d; append: true)`.
    /// A door over several locks: `access_all` reads them, `update_all` writes
    /// them ([ADR-281](../../docs/specification/adr/adr-281.md)).
    ///
    /// **It is typed here and not in the ledger**, for the reason the hull
    /// constructors are: a signature binds its type variables from the
    /// **receiver** (ADR-288), and a door over several locks has none - the
    /// locks are arguments, they hold different types, and how many there are is
    /// open. What a written signature cannot say, this says.
    ///
    /// `access_all` hands each value as a **view**, the way `access` does;
    /// `update_all` hands each in **by value** and takes a new one back per lock
    /// (D2). Both refuse anything that is not a lock, because a door that let a
    /// plain value through would be a door about nothing.
    fn locks(&mut self, door: MultiLock, args: &[Expr], span: &Span) -> Ty {
        let Some((last, locks)) = args.split_last() else {
            self.no_door(door, "It takes the locks and then the block.", span);
            return Ty::Unknown;
        };
        let Expr::Closure {
            params,
            mutable,
            body,
            ..
        } = last
        else {
            self.no_door(door, "The block comes last: `fn(a, b) { … }`.", span);
            return Ty::Unknown;
        };
        if locks.len() < 2 {
            self.no_door(door, "It's for several locks, so pass at least two.", span);
            return Ty::Unknown;
        }

        let mut held = Vec::with_capacity(locks.len());
        for lock in locks {
            let found = self.expr(lock, span);
            match locked_content_of(&found) {
                Some(inside) => held.push(match (door, &inside) {
                    // A view of what it holds, the way `access` hands one over.
                    (MultiLock::Reading, Ty::Named { name, args, .. }) => Ty::Named {
                        name: name.clone(),
                        args: args.clone(),
                        view: true,
                    },
                    (MultiLock::Reading, _) => inside,
                    (MultiLock::Writing, _) => inside,
                }),
                // Nothing written down says this is a lock. A type nothing
                // describes is not claimed about at all (Part III, C.4) - but a
                // **number** is not such a type: a literal carries no type on
                // purpose (Part I 2.4), and reading that absence as "might be a
                // lock" would hand `rustc` a program about a trait bound.
                None if found.is_unknown() && !self.s.is_a_number(self.world, lock) => {
                    held.push(Ty::Unknown)
                }
                None => {
                    self.s.findings.push(Finding {
                        severity: Severity::Error,
                        span: *span,
                        code: "NK1124".to_string(),
                        message: format!(
                            "`{}` takes locks, but this is a `{}`.",
                            door.written(),
                            found.text()
                        ),
                        notes: vec![
                            "Taking several locks together keeps them in one order, and only a \
                             lock can be taken."
                                .to_string(),
                        ],
                        help: Some(format!(
                            "Pass `{SHARED_MUT}[T]` values or `{LOCKED}[T]` fields."
                        )),
                        labels: Vec::new(),
                    });
                    held.push(Ty::Unknown);
                }
            }
        }
        if !params.is_empty() && params.len() != locks.len() {
            self.no_door(
                door,
                "The block takes one value per lock, in the order the locks are written.",
                span,
            );
        }
        // D6 is D1 widened, so the same question is asked of this block.
        // D6 is D1 widened, so the same two questions are asked of this block.
        if matches!(door, MultiLock::Writing) {
            self.s.an_update_block_returns_nothing(body, span);
            self.an_update_block_reads_what_it_writes(params, body, span);
        }
        let outer_door = std::mem::replace(
            &mut self.s.flow.at_a_write_door,
            matches!(door, MultiLock::Writing),
        );
        // Both kinds hold their locks open across the block, so both are one.
        let outer_inside = std::mem::replace(&mut self.s.inside_a_door, true);
        // **Nothing is asked of this block's promises**
        // ([ADR-277](../../docs/specification/adr/adr-277.md) D7), and that is
        // deliberate rather than an omission: `access_all` and `update_all` are
        // not ledger entries at all ([ADR-281](../../docs/specification/adr/adr-281.md)
        // D1), so there is no written type here to read a promise off. What
        // Part II 12.2 requires of the block — `sync`, and no lock — is
        // `NK2202`'s and `NK2203`'s, and a second refusal saying the same thing
        // in other words would be two rules for one mistake.
        self.lambda(
            params,
            mutable,
            body,
            &held,
            Promises {
                may_pause: true,
                may_fail: true,
            },
            span,
        );
        self.s.flow.at_a_write_door = outer_door;
        self.s.inside_a_door = outer_inside;
        match door {
            // What a lambda hands back is not written down (ADR-288 D13) — but
            // that it came **out of a lock** is
            // ([ADR-281](../../docs/specification/adr/adr-281.md) D22), and the
            // stamp is what a `set` reads. `Seen[?]` is the honest pair: the
            // type is unknown and where it came from is not.
            MultiLock::Reading => Ty::seen(Ty::Unknown),
            MultiLock::Writing => Ty::Tuple(Vec::new()),
        }
    }

    /// The one message shape for a door written wrong.
    fn no_door(&mut self, door: MultiLock, wanted: &str, span: &Span) {
        self.s.no_door(door.written(), door.shape(), wanted, span);
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
        Ty::from_ast(self.parsed, ty)
    }

    fn call(&mut self, func: &Expr, args: &[Expr], config: &[ast::ConfigArg], span: &Span) -> Ty {
        // **`T::attribute(X)` and `T::attributes(X)`, a walked type's own**
        // (ADR-331 D6): `X` is a type, not a value, and is not walked as one.
        if let Expr::Path(segments, _) = func
            && let [parameter, member] = segments.as_slice()
            && self
                .s
                .type_parameters
                .contains_key(self.parsed.text(*parameter))
            && matches!(self.parsed.text(*member), "attribute" | "attributes")
        {
            let Some(read) = self.an_attribute_read(args, span) else {
                return Ty::Unknown;
            };
            return match self.parsed.text(*member) {
                "attribute" => Ty::Nullable(Box::new(Ty::named(&read))),
                _ => Ty::Named {
                    name: "Vec".to_string(),
                    args: vec![Ty::named(&read)],
                    view: false,
                },
            };
        }
        // **A door over several locks types its own lambda**
        // ([ADR-281](../../docs/specification/adr/adr-281.md) D17), and it has to
        // be answered before the arguments are walked: what the lambda is handed
        // comes from what the *locks* hold, so the locks are typed first and the
        // lambda after. Every other call can walk its arguments in one pass.
        if let Expr::Variable(name, _) = func {
            if let Some(door) = MultiLock::named(self.parsed.text(*name)) {
                return self.locks(door, args, span);
            }
            // **`asset("…")` stands in a `comptime` initialiser and nowhere
            // else** ([ADR-310](../../docs/specification/adr/adr-310.md) D3).
            // The evaluator reads it where it belongs, so anything that reaches
            // *here* is one written somewhere it does not — and the file a
            // program reads while it **runs** has a name of its own.
            if self.parsed.text(*name) == ASSET && !self.s.flow.inside_a_comptime {
                self.s.an_asset_outside_a_comptime(span);
                return Ty::Unknown;
            }
        }
        // **`NK2203`**: a free call inside a door's block, to something that
        // takes a lock (ADR-281 D28). `println` is the one every program writes,
        // and ADR-288 D21 is where it was pinned down: it never pauses, so
        // `sync` says nothing about it, and it takes standard output's own lock
        // while yours is open.
        if self.s.inside_a_door
            && let Some(name) = self.free_callee(func)
        {
            let holds = self
                .world
                .own
                .functions
                .get(&name)
                .or_else(|| self.world.library.functions.get(&name))
                .map(|c| c.touches_a_lock)
                .unwrap_or(crate::contracts::Lock::No);
            self.s.a_lock_inside_a_lock(&name, &holds, span);
        }
        // **The callee is named and resolved before the arguments are walked**
        // ([ADR-288](../../docs/specification/adr/adr-288.md), the free half).
        // A lambda's parameters are typed from the callee's signature, so the
        // signature has to be in hand first — which is exactly the ordering the
        // *method* path was given when that record landed, and which this one
        // did not have: `hand(fn(n) { … })` left `n` with no type at all, and
        // [ADR-277](../../docs/specification/adr/adr-277.md) D7's promises had
        // nothing to be asked of.
        let name = match func {
            Expr::Variable(name, _) => self.parsed.text(*name).to_string(),
            // `unaliased`: `h::serve()` is `http::serve()` where the file wrote
            // `use http as h` (ADR-286 D12), and the ledger knows only the
            // package's own name.
            Expr::Path(segments, _) => self.parsed.unaliased(
                &segments
                    .iter()
                    .map(|s| self.parsed.text(*s))
                    .collect::<Vec<_>>()
                    .join("::"),
            ),
            other => {
                self.expr(other, span);
                return Ty::Unknown;
            }
        };

        // **A kept function value called by name**: a `let`, or a parameter
        // the function keeps. What its type says is what the call does.
        if matches!(func, Expr::Variable(_, _))
            && !self.s.run_code.contains(&name)
            && let Some(Ty::Fn {
                params,
                result,
                is_sync,
                can_throw: throws,
            }) = self.s.lookup(&name)
        {
            self.s.read_at.push((name.clone(), span.at() as i64));
            self.arguments_given(args, &params, false, None, span);
            self.kept_arguments_are_its_own(args, &params);
            self.s
                .facts
                .kept_calls
                .insert(((span.at() as i64), name), (!is_sync, throws));
            if let Some(current) = self.s.current.clone() {
                let entry = self
                    .s
                    .methods
                    .entry(current)
                    .or_insert_with(MethodCalls::empty);
                entry.code_pauses |= !is_sync;
                entry.code_fails |= throws;
            }
            return (*result).unwrap_or_else(|| Ty::named("()"));
        }

        // **A variant is built by its constructor, and its parts are what it
        // keeps** (0.0.236). `Stmt::Say("a")` resolved to nothing, so its
        // argument was walked with no type to meet: the literal went below as
        // a `&str` where the variant holds a `String`, and `rustc` said
        // *mismatched types* about a file nobody wrote. The parts are the
        // variant's declared types, a literal handed to text is built into
        // text of its own there (ADR-282 D4), and a name handed in is given.
        if let Some(parts) = self.s.variant_parts.get(&name).cloned() {
            let found = self.arguments_given(args, &parts, false, None, span);
            for ((arg, want), ty) in args.iter().zip(&parts).zip(&found) {
                if self.text_literal(want, arg, true).is_none() {
                    self.hands_over(arg, ty, "put into a variant", span);
                }
                // **A part that holds a `T?` takes a plain value as `Some`**,
                // as a parameter does (Part I 2.3).
                if let Some(how) = self.widened_slot(ty, want, Some(arg), span) {
                    self.s.facts.wrapped.insert(value_node(arg), how);
                }
            }
            let owner = name
                .rsplit_once("::")
                .map_or(name.as_str(), |(owner, _)| owner);
            return Ty::named(owner);
        }

        // **A call to an `extern` name is written inside `unsafe { … }`**
        // ([ADR-302](../../docs/specification/adr/adr-302.md) D3), and this is
        // where that is asked: the name is in hand and the block is a flag the
        // walk carries.
        self.s.a_foreign_call_outside_unsafe(&name, span);

        // **A type is constructed by its anonymous constructor**
        // ([ADR-140](../../docs/specification/adr/adr-140.md) D2), so a written
        // `Type::new` is the other spelling and is refused — in `std` as in a
        // `.nika` file, which is the whole of what D2 evens out.
        self.s.a_constructor_written_as_new(self.world, &name, span);

        // **What needs no `use` is the list on Part I's first page**
        // ([ADR-313](../../docs/specification/adr/adr-313.md) D1), and these two
        // are the halves of the rule: a name that lives in a module written
        // without it, and a module used without being introduced.
        self.s
            .a_std_name_without_its_module(self.world, &name, span);
        self.s.a_module_used_before_it_is_introduced(&name, span);
        // **The head of a call path is one of five things, as a value's is**
        // ([ADR-286](../../docs/specification/adr/adr-286.md) D31, #488):
        // `nowhere::wobble(1)` lowered, and the language below said *cannot
        // find module or crate* about a file nobody wrote. Whether a head
        // exists is always known, so this is no guess (C.4 is about types).
        // `Self` and a built-in type name a type that is always there.
        if let Expr::Path(segments, _) = func
            && let [head, _, ..] = segments.as_slice()
        {
            let head = self.parsed.text(*head);
            if head != "Self" && !OFFERED.contains(&head) {
                let head = head.to_string();
                let written = segments
                    .iter()
                    .map(|s| self.parsed.text(*s))
                    .collect::<Vec<_>>()
                    .join("::");
                self.a_head_nothing_declares(&head, &written, span);
            }
        }
        self.s
            .a_prelude_name_with_a_module_in_front(self.world, &name, span);
        self.s
            .a_constructor_nothing_describes(self.world, &name, args, span);

        // **A grammar is entered by an ordinary call**
        // ([ADR-296](../../docs/specification/adr/adr-296.md) D24), through a
        // **path** since [ADR-140](../../docs/specification/adr/adr-140.md) D3.
        // Answered before anything is resolved, because a grammar is not a
        // ledger entry a `resolve` would find.
        if let Some(entered) = self.s.grammar_path(&name) {
            return self.grammar_call(&entered, args, span);
        }

        // **A call to a function that walks a type's fields is an
        // *instantiation*** ([ADR-304](../../docs/specification/adr/adr-304.md)
        // D2): the shape is the caller's to supply, so the loop is unrolled
        // once per type argument actually used and this is where the type
        // argument is known.
        //
        // Recorded here and walked afterwards, because the body has to be read
        // with the fields in hand and this walk is in the middle of another
        // one. What is refused at the call itself is `NK1164` — *you passed
        // something that is not a struct*
        // ([ADR-304](../../docs/specification/adr/adr-304.md) D3) — which the
        // bound already answers and which is why this can assume a struct.
        self.an_instantiation(&name, args, span);

        // **A struct literal written like a call**
        // ([ADR-140](../../docs/specification/adr/adr-140.md) D1). `Stats(min: 1)`
        // parsed as Kap 4.2's named literal until D1 took that spelling out of the
        // language; it is a call with options now, and where the name is a **type**
        // that is the old form rather than a call anybody meant. Asked here because
        // the parser cannot ask it - the two forms are one spelling and a name
        // denotes one of them - and asked before the resolution below, which would
        // otherwise answer about the anonymous constructor: the silent lowering was
        // `Stats::new()` with the fields dropped, which `rustc` then refused about a
        // file nobody wrote ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)).
        if !config.is_empty() && args.is_empty() && self.declares_a_type(&name) {
            self.s
                .a_literal_written_like_a_call(self.world, &name, config, span);
            for option in config {
                self.expr(&option.value, span);
            }
            return Ty::named(&name);
        }

        // **A type of the program's own with no constructor, called** (0.0.240):
        // `NotANumber(text)` against `struct NotANumber { text: String }`
        // lowered to `NotANumber::new(…)`, and `rustc` said *no function named
        // `new`* about a file nobody wrote. A struct is built with a literal.
        if !args.is_empty() && self.s.structs.contains_key(&name) && self.resolve(&name).is_none() {
            self.s.a_struct_called_without_a_constructor(&name, span);
        }

        // A tuple variant of an enum declared here - `Op::Plus(1)` - is a value
        // of that enum, not a call to a function. Its arguments are still
        // walked, below, with the rest.
        let variant = name
            .split_once("::")
            .filter(|(ty, variant)| self.s.is_variant(ty, variant))
            .map(|(ty, _)| ty.to_string());
        // Neither a variant nor a hull is a ledger entry, so neither has a
        // signature to type an argument from.
        // **A binding in scope hides an item of its name** (#451), as a
        // `let` hides one and as Rust's scope does: `twice(4)` inside `fn
        // apply(twice: fn(i64) -> i64 sync)` is the parameter, not a free
        // `fn twice(a, b, c)` - which it was, and the call was checked
        // against the wrong signature.
        let shadowed = matches!(func, Expr::Variable(_, _)) && self.s.lookup(&name).is_some();
        let resolved = match variant.is_some() || is_hull(&name) || shadowed {
            true => None,
            false => self.resolve(&name),
        };
        let mut expected: Vec<Ty> = resolved
            .as_ref()
            .map(|(_, contract)| expected_arguments(contract))
            .unwrap_or_default();
        // **A parameter of code, run in place, takes what its type says**
        // (#125): `root_at(name)` for a `root_at: fn(ref String) -> i64` and
        // an owned `name` is a view handed over, which the walk below writes
        // the reference for - typed from nothing, the text went below by
        // value and `rustc` refused it.
        if resolved.is_none()
            && matches!(func, Expr::Variable(_, _))
            && self.s.run_code.contains(&name)
            && let Some(Ty::Fn { params, .. }) = self.s.lookup(&name)
        {
            expected = params;
        }
        // The same walk the method path uses, so the same questions are asked
        // in both - or `takes(self.name)` slips past `NK1131` while
        // `x.takes(self.name)` does not.
        let outer_paused = std::mem::take(&mut self.s.flow.paused_args);
        let found = self.arguments_given(
            args,
            &expected,
            resolved
                .as_ref()
                .is_some_and(|(key, _)| self.world.own.functions.contains_key(key)),
            resolved.as_ref().map(|(_, contract)| contract),
            span,
        );
        let paused_here = std::mem::replace(&mut self.s.flow.paused_args, outer_paused);
        // **A scope with a task that pauses polls it, and pauses itself**
        // (ADR-328 D10): its lambda paused because a task in it did.
        if resolved
            .as_ref()
            .is_some_and(|(key, _)| key == "task::scope")
            && let Some(body) = args
                .first()
                .filter(|arg| paused_here.contains(&value_node(arg)))
        {
            self.s.facts.paused_scopes.insert(value_node(body));
            self.s
                .a_call_that_may_pause("task::scope", true, false, span);
        }
        if resolved
            .as_ref()
            .is_some_and(|(key, _)| key == "supervisor::child")
            && let Some(body) = args.first()
        {
            self.a_child_that_keeps_state(body, span);
        }
        let passed: Vec<(String, Ty)> = config
            .iter()
            .map(|a| {
                self.a_field_of_a_borrowed_subject(&a.value, span, "passed");
                (
                    self.parsed.text(a.name).to_string(),
                    self.expr(&a.value, span),
                )
            })
            .collect();

        if let Some(ty) = variant {
            return Ty::named(&ty);
        }

        // **A handle is not constructed** (ADR-302 D7): it is an address a C
        // function hands back, so there is no value of one this language can
        // make. Refused here rather than below, where `rustc` would say the
        // tuple struct takes one field and name a type the source never wrote.
        if self.s.opaque_handles.contains(&name) {
            self.s.a_handle_is_not_made_here(&name, span);
            return Ty::named(&name);
        }

        // **A hull you can observe, you write**
        // ([ADR-281](../../docs/specification/adr/adr-281.md) D2). The three hull
        // types are made by a call, and it is this compiler's to type rather than
        // the ledger's: the ledger binds `$T` from a **receiver** (ADR-288) and a
        // constructor has none, so the type argument would have to come from what
        // is handed in.
        if is_hull(&name) {
            return self.s.hull(&name, args.len() as i64, &found, span);
        }

        let Some((key, contract)) = resolved else {
            if matches!(func, Expr::Variable(_, _)) {
                self.a_function_nothing_declares(&name, span);
            }
            if let Expr::Path(segments, _) = func
                && let [head, member] = segments.as_slice()
                && !shadowed
            {
                let head = self.parsed.unaliased(self.parsed.text(*head)).to_string();
                let member = self.parsed.text(*member).to_string();
                self.a_member_nothing_writes_down(&head, &member, false, span);
            }
            // A call nothing describes is a call this compiler cannot see the
            // end of, and a thread of its own is among the things it may do
            // (ADR-303 D7). What it is handed is therefore handed across.
            self.crosses_into_an_unseen_call(
                &name,
                Arguments {
                    args,
                    found: &found,
                    config,
                    passed: &passed,
                },
                span,
                &format!(
                    "The compiler knows nothing about `{name}`, so it has to assume it \
                     might start a thread of its own."
                ),
            );
            // And a call this compiler cannot see the end of says nothing about
            // whether it can **fail**, either
            // ([ADR-308](../../docs/specification/adr/adr-308.md)).
            self.guard_has_no_answer();
            // **A parameter of code, run in place, hands back what its type
            // says.** Typed as nothing, `let name = f(s)` left `name.contains`
            // unresolved, and an unresolved method is read as one that may
            // pause - so the function came out `async` with nothing in it that
            // pauses (found moving `foreign`'s walk into Nikaia, #125).
            if let Expr::Variable(_, _) = func
                && self.s.run_code.contains(&name)
                && let Some(Ty::Fn { params, result, .. }) = self.s.lookup(&name)
            {
                // **And what it takes as a view is lent to it**: an owned
                // argument for a `ref` parameter gains the `&` a declared
                // function's would (`the_compiler_writes_the_reference`).
                for (at, (given, wanted)) in args.iter().zip(&params).enumerate() {
                    let had = found.get(at).cloned().unwrap_or(Ty::Unknown);
                    if wanted.is_a_view()
                        && !had.is_a_view()
                        && !matches!(had, Ty::Unknown | Ty::Fn { .. })
                    {
                        self.s.facts.lent_args.insert(value_node(given));
                    }
                }
                return (*result).unwrap_or_else(|| Ty::named("()"));
            }
            return Ty::Unknown;
        };
        self.s.reachable(&name, &contract, span);
        // **A described call is asked too, where the description says the word**
        // ([ADR-290](../../docs/specification/adr/adr-290.md) D7). It fires on
        // the **claim** and never on its absence: a description that does not
        // say is a description that was not asked, and the program keeps the
        // answer it has today. Before this, a crate that answered every other
        // question honestly turned `NK2502` off by being described, which is
        // the half of [ADR-303](../../docs/specification/adr/adr-303.md) D7
        // that was left open.
        if contract.threads.may() {
            self.crosses_into_an_unseen_call(
                &name,
                Arguments {
                    args,
                    found: &found,
                    config,
                    passed: &passed,
                },
                span,
                &format!(
                    "`{name}` is described as starting a thread of its own (`threads = true`), \
                     so what it is given may be looked at from one"
                ),
            );
        }
        self.may_fail_here(&key, &contract, span);
        self.s.a_call_that_may_pause(
            &name,
            !contract.sync_claim.is_sync(),
            unpromised(&contract),
            span,
        );
        self.s.a_pausing_call_in_an_action(&name, &contract, span);
        // `Stats(first)` is the anonymous constructor of Kap 4.2, which the
        // lowering names `Stats::new` - and which hands back the type it is on,
        // whatever its declaration says about `Self`.
        //
        // **Unless the declaration already names it with its arguments**
        // ([ADR-140](../../docs/specification/adr/adr-140.md) D2). A `.nika`
        // file's constructor is on a type with no parameters, so naming the type
        // is the whole answer; `std`'s own entries are not — `Vec::new` declares
        // `-> Vec[?]`, and `Ty::named("Vec")` threw the `[?]` away, so
        // `let xs = Vec()` came out as a `Vec` and `NK1106` refused it against
        // every `Vec[T]` it was given to. The rule was right for the one case it
        // had and wrong for the one D2 brought in.
        let declared = contract
            .signature
            .as_ref()
            .and_then(|s| s.result.as_ref())
            .filter(|ty| !matches!(ty, Ty::Unknown))
            .filter(|ty| !matches!(ty, Ty::Named { name, .. } if name == "Self"));
        let constructed = key
            .strip_suffix("::new")
            .filter(|_| !name.ends_with("::new"))
            .map(|ty| match declared {
                Some(declared) => declared.clone(),
                None => Ty::named(ty),
            });
        self.s.flow.option_values = config.iter().map(|a| value_node(&a.value)).collect();
        let result = self.arguments(&key, &name, &contract, args, &found, &passed, span);
        self.s.flow.option_values.clear();
        // **What the arguments tell the signature**
        // ([ADR-295](../../docs/specification/adr/adr-295.md) D2). A free
        // function has no receiver, so ADR-288's binding had nothing to work
        // from and `hand(7)` handed back `?`; the same `bind` pointed at the
        // parameters answers `i64`. After `arguments` rather than before it,
        // because a variable fits everything and the check is therefore the
        // same either way round - while the *types* are not: a lambda's
        // parameters are typed from the signature (ADR-288), so the signature
        // has to reach them unsubstituted.
        let bound = from_arguments(&contract, &found);
        self.a_call_that_hands_its_number_back(&contract, &bound, args);
        // …and what the arguments tell a **bound** (ADR-295 D16), which is the
        // same binding read for the other question a type parameter raises.
        self.s
            .a_bound_the_argument_does_not_meet(self.world, &key, &bound, span);
        let result = constructed.unwrap_or_else(|| ty::substitute(&result, &bound));
        self.stamped_through(&contract, &found, result)
    }

    /// **A generic call whose result only an open number binds** (#497):
    /// `hand(7)` for `fn hand[T](x: T) -> T` hands back the `7`, whose type
    /// its uses decide (ADR-285 D1), so the call is that open number.
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
        // No signature is no claim. The hand-written half of `std.contracts`
        // has some, and an entry that says nothing is checked against nothing.
        let Some(signature) = contract.signature.clone() else {
            return Ty::Unknown;
        };
        let wanted = signature.arguments();

        if wanted.len() != found.len() {
            self.s.findings.push(Finding {
                severity: Severity::Error,
                span: *span,
                code: "NK1101".to_string(),
                message: format!(
                    "`{key}` takes {}, but you passed {}.",
                    plural(wanted.len(), "argument"),
                    found.len()
                ),
                notes: vec![format!("It's declared as `{key}{}`.", signature.text())],
                help: Some(match wanted.len() {
                    0 => format!("Call it without arguments: `{key}()`."),
                    _ => {
                        let takes = wanted
                            .iter()
                            .map(|(n, t)| format!("`{n}: {}`", t.text()))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("It takes {takes}{}.", a_root_is_one_of_two(wanted))
                    }
                }),
                labels: Vec::new(),
            });
            return signature.result_or_unit();
        }

        // **A length beside a view is checked at the call**
        // ([ADR-302](../../docs/specification/adr/adr-302.md) D6), before the
        // arguments are measured one at a time: the question is about a *pair*
        // of them, and a `usize` that is measured on its own has already
        // passed.
        self.a_length_that_fits_its_buffer(written, wanted, given, found, span);
        self.one_type_variable_two_integer_types(key, wanted, given, found, span);

        // Kap 5.1: an option is named, so it is checked by name - that it
        // exists, and that what is passed is what it takes.
        for (at, (name, found)) in passed.iter().enumerate() {
            match signature.config.iter().find(|c| c.name == *name) {
                Some(option) => {
                    if found.fits(&option.ty) {
                        continue;
                    }
                    // **An option of a `ref` type lends what it is passed**
                    // (#455), as a `ref` parameter does (ADR-094 D1): the
                    // owned value fits the view, and the `&` is the
                    // compiler's to write.
                    if let Some(value) = self.s.flow.option_values.get(at).copied()
                        && option.ty.is_a_view()
                        && !found.is_a_view()
                        && found.fits(&option.ty.owned())
                    {
                        self.s.facts.lent_options.insert(value);
                        continue;
                    }
                    let (want, ty) = (option.ty.clone(), option.ty.text());
                    let name = name.clone();
                    let key = key.to_string();
                    self.expect(found, &want, *span, "field", move |found, _| {
                        format!("The option `{name}` of `{key}` takes `{ty}`, but you're passing `{found}`.")
                    });
                }
                None => self.s.no_such_option(key, name, &signature, span),
            }
        }

        let text_view = Ty::view(ty::TEXT_VIEW);
        for (at, ((name, want), found)) in wanted.iter().zip(found).enumerate() {
            // **A view of a `String` is a view of text here**
            // ([ADR-233](../../docs/specification/adr/adr-233.md) D4): an
            // element of `words.iter()` is one, and the language below lends it
            // as the `&str` a parameter takes at the call, by itself.
            let found = match is_a_view_of_a_string(found) {
                true => &text_view,
                false => found,
            };
            // A literal that cannot fit the parameter it is given, asked here
            // rather than where the argument was walked: a free call walks its
            // arguments before it resolves the callee, so the type to measure
            // against is not known yet at that point (ADR-288's ordering is what
            // gives a *method* call the answer earlier).
            if let Some(given) = given.get(at) {
                self.constant_fits(given, Some(want), span);
                if self.an_open_number_where_none_fits(
                    found,
                    want,
                    given,
                    *span,
                    "argument",
                    &|found, want| {
                        format!(
                            "`{key}` expects `{name}` to be `{want}`, but you're passing `{found}`."
                        )
                    },
                ) {
                    continue;
                }
                // A free call walks its arguments first, so a call on a
                // number written as its receiver is told its parameter here.
                if let Expr::MethodCall { receiver, .. } = given
                    && let Expr::LitInt {
                        value, negative, ..
                    } = **receiver
                {
                    let key = (
                        value_node(receiver),
                        nikaia_std::tools::integers::integer(value, negative),
                    );
                    if let Some(at) = self.s.flow.literal_receivers.iter().position(|k| *k == key) {
                        let pending = std::mem::take(&mut self.s.flow.literal_receivers);
                        self.s.flow.literal_receivers = vec![pending[at]];
                        self.literal_receivers_take(want);
                        self.s.flow.literal_receivers = pending
                            .into_iter()
                            .enumerate()
                            .filter(|(i, _)| *i != at)
                            .map(|(_, k)| k)
                            .collect();
                    }
                }
            }
            self.a_cleanup_that_moved(key, name, want, given.get(at), span);
            // **A parameter is a use** (ADR-152 D4), and a literal handed to one
            // that takes an `Array[T, N]` **is** that array. What it changes is
            // the type the rest of this loop reads, rather than ending it: the
            // `&` the compiler writes (ADR-094 D1) and the ordinary fit are
            // still this argument's questions, and they have to be asked of what
            // the literal turned out to be.
            //
            // **And one handed to a `String` is one** (ADR-282 D4) — text of its
            // own where the callee keeps it, and the literal as it is where the
            // callee only reads it, because that parameter is a `&str` below
            // (D3) and a constant needs no copy to be read.
            // **What is printed is only read** (ADR-279 D5): the four print
            // calls take any value by `?`, which would otherwise count as
            // kept, and format it by reference (#473).
            let lent = match self.world.own.functions.contains_key(key) {
                true => crate::contracts::keeps::lends_in_own(
                    contract,
                    at + usize::from(signature.takes_a_receiver()),
                    &[self.world.library],
                ),
                false => crate::contracts::keeps::lends_in(
                    contract,
                    at + usize::from(signature.takes_a_receiver()),
                    &[self.world.library],
                ),
            } || (at == 0 && is_a_lookup(key))
                || matches!(key, "println" | "eprintln" | "print" | "eprint");
            let kept = !lent;
            // **What the callee keeps, it is given** (ADR-293 D29): no `&` is
            // written for this position, so a name here moves.
            // **Not across the C boundary**: what a C declaration takes it
            // takes by address or by copy, and the call writes that address
            // ([ADR-302](../../docs/specification/adr/adr-302.md) D5).
            if kept
                && !want.is_a_view()
                && !signature.mutable.contains(name)
                && !self.s.foreign_names.contains(written)
                && let Some(given) = given.get(at)
            {
                self.hands_over(
                    given,
                    found,
                    &format!("passed to `{written}`, which keeps it"),
                    span,
                );
            }
            // **A type variable only this argument decides takes the
            // argument's own type** (0.0.230, issue #155 before it): a text
            // literal is a `String`, so where a kept, by-value `$T` is bound by
            // nothing else - no receiver, no other parameter - it is constructed
            // as one. `hyper_shim::across_a_thread(value: $T)` with
            // `T: Describe` implemented for `String` is the case: the literal's
            // `&str` was a type the program never wrote. A `$T` a receiver
            // binds (`Vec[$T]::push`) is the receiver's to answer and is left.
            let text = Ty::named("String");
            let literal_wants = match kept && self.decided_by_this_argument(&signature, wanted, at)
            {
                true => &text,
                false => want,
            };
            let array = given.get(at).and_then(|given| {
                self.array_literal(found, want, given, span)
                    .or_else(|| self.text_literal(literal_wants, given, kept))
            });
            let found = array.as_ref().unwrap_or(found);
            // **A view handed to a `String` the callee only reads is lent as it
            // is** ([ADR-282](../../docs/specification/adr/adr-282.md) D11). The
            // parameter is a `&str` below (ADR-282 D5), so the view already is
            // what the callee takes: asking for `.to_owned()` here asked for a
            // copy nothing would keep. Only for a function this compiler
            // declares, because only there is the `&str` its own writing.
            //
            // **Not a `ref` the source wrote**, which is `NK1137`'s below: the
            // reference at a lending call is the compiler's, and a view that
            // arrives as a view is the case this is for.
            let wrote_ref = matches!(
                given.get(at),
                Some(Expr::Unary {
                    op: crate::ast::UnaryOp::Ref,
                    ..
                })
            );
            if !kept
                && !wrote_ref
                && *found == Ty::view("str")
                && *want == Ty::named("String")
                && self.world.own.functions.contains_key(key)
            {
                continue;
            }
            // **A view handed to a parameter both kinds flow into** is
            // borrowed as it is ([ADR-282](../../docs/specification/adr/adr-282.md)
            // D2): the emitter hands it over, as it does into a mixed field.
            if views_into_text(found, want) && self.either_param(key, at) {
                continue;
            }
            // **A `usize` at the C boundary takes this language's own integer**
            // ([ADR-302](../../docs/specification/adr/adr-302.md) D6,
            // [ADR-285](../../docs/specification/adr/adr-285.md) D1). A length
            // here is an `i64` and the machine-width type left the surface a
            // program can write, so a declaration that says `size_t` is handed
            // an `i64` and the conversion is emitted — the same arrangement
            // `str::repeat` has, arrived at from the **declaration** rather
            // than from a name.
            //
            // `continue`, because this *is* the fit: what is left for the
            // positions below to decide is a `&` this argument cannot want, a
            // number being copied rather than moved.
            if self.s.a_count_at_the_boundary(written, want, found) {
                continue;
            }
            // Part I 2.3's third position for the wrap: a plain value in a
            // parameter the callee declares nullable. Recorded before `fits`
            // is consulted, because this *is* the fit - `Ty::fits` allows it,
            // and what is left is telling the emitter to write the
            // constructor.
            // **The compiler writes the `&` at the call** (ADR-094 D1), where
            // the callee lends this position and the value is not already a
            // view. Asked before the wrap and the deref for `nullable_args`'
            // reason: a lent argument *is* a fit, and what is left is telling
            // the emitter what to write. The rule asks the fit itself, so an
            // argument that is simply the wrong value falls through to the
            // message it always had.
            if self.the_compiler_writes_the_reference(
                contract,
                Argument {
                    at,
                    given: given.get(at),
                    found,
                    want,
                    own: self.world.own.functions.contains_key(key),
                },
                written,
                span,
            ) {
                continue;
            }
            // **What is printed is only read** (ADR-279 D5): below, the four
            // print calls format their argument by reference, so a `??`
            // handed to one lends its left side as a lent argument does.
            if matches!(written, "println" | "eprintln" | "print" | "eprint")
                && let Some(given) = given.get(at)
                && self.a_pending_coalesce_is_lent(given)
            {
                continue;
            }
            // **A file's name is printed as its bytes** (ADR-319 D4): the
            // print calls take text or a name.
            if matches!(written, "println" | "eprintln" | "print" | "eprint")
                && let Some(given) = given.get(at)
                && matches!(found, Ty::Named { name, .. } if name == "fs::Path")
            {
                // An `f"…"` recorded which of its holes are names already.
                self.s
                    .facts
                    .path_holes
                    .entry(value_node(given))
                    .or_insert_with(|| vec![true]);
                continue;
            }
            if let Some(value) = given.get(at) {
                self.a_map_read_kept(found, want, value, span);
            }
            if let Some(how) = self.widened_slot(found, want, given.get(at), span) {
                if let Some(value) = given.get(at) {
                    self.s.facts.nullable_args.insert(value_node(value), how);
                }
                continue;
            }
            if self.fits_through_deref(found, want) {
                continue;
            }
            // **A view of a value that copies, where the value is wanted**
            // ([ADR-233](../../docs/specification/adr/adr-233.md) D4): the copy
            // out of it is a machine word, and the compiler's to write. It is
            // what a lambda `sort_by_key` hands a `ref i64` meets first.
            // A **name**: what an operator makes of a view is a value below
            // whatever this checker calls it, and a `*` on it is not Rust.
            if self.a_view_that_copies(found)
                && unviewed(found) == *want
                && matches!(given.get(at), Some(Expr::Variable(_, _)))
            {
                if let Some(value) = given.get(at) {
                    self.s.facts.copied_args.insert(value_node(value));
                }
                continue;
            }
            // A plain value where a hull is wanted. It is refused by a code of
            // its own because the sentence is worth more than a type mismatch's:
            // it names the hull and the value. **What the way out is changed**
            // ([ADR-281](../../docs/specification/adr/adr-281.md) D2) - the word
            // goes here, at the call, where it used to have to go somewhere else.
            if becomes_shared(found, want) {
                let value = given.get(at).and_then(|arg| names_of(self.world, arg));
                self.s.findings.push(Finding {
                    severity: Severity::Error,
                    span: *span,
                    code: "NK1115".to_string(),
                    message: match &value {
                        Some(value) => {
                            format!("`{key}` takes a shared value, but `{value}` isn't one.")
                        }
                        None => format!("`{key}` takes a shared value, but this isn't one."),
                    },
                    notes: vec![format!("It's declared as `{key}{}`.", signature.text())],
                    help: Some(match (&value, want) {
                        (Some(value), Ty::Named { name, .. }) => {
                            format!("Wrap it: `{name}({value})`.")
                        }
                        (None, Ty::Named { name, .. }) => {
                            format!("Wrap it: `{name}(…)`.")
                        }
                        _ => format!("Make it {} `{}`.", an_or_a(&want.text()), want.text()),
                    }),
                    labels: Vec::new(),
                });
                continue;
            }
            if self.a_name_in_text(want, given.get(at), span) {
                continue;
            }
            let (why, help) = match self.a_view_kept(
                found,
                want,
                given.get(at),
                &format!("`{key}` keeps its `{name}` after the call returns"),
            ) {
                Some((why, help)) => (why, help),
                None => (Vec::new(), convert(found, want)),
            };
            self.s.findings.push(Finding {
                severity: Severity::Error,
                span: *span,
                code: "NK1102".to_string(),
                message: format!(
                    "`{key}` expects `{name}` to be `{}`, but you're passing `{}`.",
                    want.text(),
                    found.text()
                ),
                notes: std::iter::once(format!("It's declared as `{key}{}`.", signature.text()))
                    .chain(why)
                    .collect(),
                help: Some(help),
                labels: Vec::new(),
            });
        }

        signature.result_or_unit()
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

    /// What a type seen through (`deref` in its ledger) hands out, with the
    /// receiver's own arguments bound: `&$T` of a `Shared[$T]`.
    fn seen_through(&self, ty: &Ty) -> Option<Ty> {
        self.s.seen_through(self.world, ty)
    }

    fn fits_through_deref(&self, found: &Ty, want: &Ty) -> bool {
        self.s.fits_through_deref(self.world, found, want)
    }

    /// **An open number put where no number fits** (#554): `let n = 12` and
    /// then `let s: String = n`. The number has no type until a use decides
    /// it ([ADR-285](../../docs/specification/adr/adr-285.md) D23), so it read
    /// as `?` here and passed, and `rustc` refused the generated file. A use
    /// that asks for a type no number can be is refused at that use, with the
    /// position's own code and sentence, the value named *a number*.
    fn an_open_number_where_none_fits(
        &mut self,
        found: &Ty,
        want: &Ty,
        value: &Expr,
        span: Span,
        what: &str,
        message: &dyn Fn(&str, &str) -> String,
    ) -> bool {
        self.s.an_open_number_where_none_fits(
            self.world,
            found,
            want,
            value,
            &span,
            what,
            &|hole, ty| message(hole, ty),
        )
    }

    /// [`Checker::expect`], for a position that has the value in hand, so a
    /// view of text kept where text of its own is wanted can say **why** here
    /// ([ADR-282](../../docs/specification/adr/adr-282.md) D25). `keeper` is the
    /// position's own words for what keeps it.
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
        // **A place that says its type is a use of the numbers put there**
        // ([ADR-285](../../docs/specification/adr/adr-285.md) D24): a field, a
        // `let`'s annotation, what a function hands back.
        if let Some(value) = value {
            self.number_asked(value, want, &span, true);
            if self.an_open_number_where_none_fits(found, want, value, span, what, &message) {
                return;
            }
        }
        if self.a_name_in_text(want, value, &span) {
            return;
        }
        // **A view into a position both kinds of text flow into** is borrowed
        // as it is (ADR-282 D14, ADR-282 D15): that is what the position is for.
        // The position reads as text of its own, so a view is what needs
        // letting through.
        let either = match what {
            "returns" => self.s.flow.expected_either,
            "field" => std::mem::take(&mut self.s.flow.field_either),
            _ => false,
        };
        if either && *found == Ty::view("str") && *want == Ty::named("String") {
            return;
        }
        // **Into a `String?` the same holds of what is there**
        // ([ADR-282](../../docs/specification/adr/adr-282.md) D12): a view,
        // present or absent, goes into a mixed one as it is, and is refused
        // from an owned one - which the `?` alone had let through to the
        // language below.
        if views_into_nullable_text(found, want) {
            if either {
                return;
            }
            let present = match found {
                Ty::Nullable(found) => found.as_ref(),
                found => found,
            };
            let text = Ty::named("String");
            let before = self.s.findings.len();
            self.expect(present, &text, span, what, message);
            if self.s.findings.len() > before
                && let Some((why, help)) = self.a_view_kept(present, &text, value, keeper)
                && let Some(finding) = self.s.findings.last_mut()
            {
                finding.notes.extend(why);
                finding.help = Some(help);
            }
            return;
        }
        let before = self.s.findings.len();
        self.expect(found, want, span, what, message);
        if self.s.findings.len() == before {
            return;
        }
        if let Some((why, help)) = self.a_view_kept(found, want, value, keeper)
            && let Some(finding) = self.s.findings.last_mut()
        {
            finding.notes.extend(why);
            finding.help = Some(help);
        }
    }

    /// **A view kept where the receiver says text of its own**
    /// ([ADR-282](../../docs/specification/adr/adr-282.md) D19):
    /// `names.push(line.trim())` on a `Vec[String]`. The signature says `$T`,
    /// which a view fits; the receiver binds `$T` to `String`, and that is the
    /// question - the one [`Checker::arguments`] asks of a parameter that says
    /// `String` itself. A position the tier pass made a view is `ref String`
    /// here and fits; one it made mixed is handed `value.into_either()`, which
    /// reads as text of its own.
    fn a_view_kept_where_the_receiver_says_text(
        &mut self,
        key: &str,
        contract: &FnContract,
        given: &[Expr],
        found: &[Ty],
        expected: &[Ty],
        span: &Span,
    ) {
        let Some(signature) = contract.signature.as_ref() else {
            return;
        };
        let arguments = signature.arguments();
        for (at, ((name, declared), want)) in arguments.iter().zip(expected).enumerate() {
            let generic = matches!(declared, Ty::Var { .. });
            let lent = match self.world.own.functions.contains_key(key) {
                true => crate::contracts::keeps::lends_in_own(
                    contract,
                    at + usize::from(signature.takes_a_receiver()),
                    &[self.world.library],
                ),
                false => crate::contracts::keeps::lends_in(
                    contract,
                    at + usize::from(signature.takes_a_receiver()),
                    &[self.world.library],
                ),
            } || (at == 0 && is_a_lookup(key));
            let kept = !lent;
            let Some(found) = found.get(at) else { continue };
            if generic && kept && self.s.a_view_of_what_does_not_copy(self.world, found, want) {
                self.s.a_view_kept_whole(key, name, found, want, span);
                continue;
            }
            if !generic || !kept || !views_into_text(found, want) {
                continue;
            }
            // A literal is built into text where it is kept (ADR-282 D4),
            // which the caller has already recorded.
            let literal = given.get(at).and_then(|g| self.text_literal(want, g, true));
            let found = literal.as_ref().unwrap_or(found);
            if !views_into_text(found, want) {
                continue;
            }
            let keeper = format!("`{key}` keeps its `{name}` after the call returns");
            let (key, name) = (key.to_string(), name.clone());
            self.expect_kept(
                found,
                want,
                given.get(at),
                &keeper,
                *span,
                "argument",
                move |found, want| {
                    format!("`{key}` expects `{name}` to be `{want}` here, but you're passing `{found}`.")
                },
            );
        }
    }

    /// Whether parameter `at` of a function this program declares is text both
    /// kinds of which flow into (ADR-282 D14): by its key, `f` or `Type::m`.
    fn either_param(&self, key: &str, at: usize) -> bool {
        check_items::either_param(&self.world.here, key, at as i64)
    }

    /// A function field code is put into where the lock column cannot read it
    /// (ADR-230 D1): undecided, never quiet.
    fn a_field_whose_code_is_not_seen(&mut self, owner: &str, field: &str) {
        self.s.a_field_whose_code_is_not_seen(owner, field);
    }

    /// Whether a struct's field is text both kinds of which flow into
    /// ([ADR-282](../../docs/specification/adr/adr-282.md) D14).
    fn either_field(&self, owner: &str, field: &str) -> bool {
        check_items::either_field(&self.world.here, owner, field)
    }

    fn a_view_kept(
        &self,
        found: &Ty,
        want: &Ty,
        value: Option<&Expr>,
        keeper: &str,
    ) -> Option<(Vec<String>, String)> {
        let named = match value {
            Some(Expr::Variable(name, _)) => Some(self.parsed.text(*name)),
            _ => None,
        };
        let kept = self.s.a_view_kept(found, want, named, keeper)?;
        Some((kept.notes, kept.help))
    }

    /// **The integer type a use asks of a call on a number written as its
    /// receiver is the number's** (#417), where the number is one of it.
    fn literal_receivers_take(&mut self, want: &Ty) {
        self.s.literal_receivers_take(want);
    }

    /// Report only when both sides are known and they disagree.
    fn expect(
        &mut self,
        found: &Ty,
        want: &Ty,
        span: Span,
        what: &str,
        message: impl FnOnce(&str, &str) -> String,
    ) {
        if found.is_unknown() {
            self.literal_receivers_take(want);
        }
        if self.fits_through_deref(found, want) {
            return;
        }
        // A file's name made from text: the position recorded how
        // (`wrap_for`, ADR-319 D2).
        if text_as_a_path(found, want).is_some() {
            return;
        }
        // An integer widened where no value can be lost (ADR-285 D32): the
        // position recorded the widening (`wrap_for`).
        if widened_into(found, want).is_some() {
            return;
        }
        let waiting = self
            .s
            .flow
            .unsigned_slots
            .get(&(span.at() as i64))
            .and_then(|slots| {
                slots
                    .iter()
                    .position(|s| s.refusal.is_none() && s.found == *found && s.want == *want)
            });
        let code = match what {
            "let" => "NK1103",
            "returns" => "NK1104",
            "assign" => "NK1105",
            "field" => "NK1106",
            // An argument a method's receiver types (ADR-282 D19), which is
            // the code `arguments` gives one its signature types.
            "argument" => "NK1102",
            // **`NK1166`, and it used to be an `unreachable!`.** A `comptime`
            // whose value disagrees with its declared type reached `expect`
            // with a word that had no code, and the compiler **panicked** -
            // which is [Part I 6.8](../../docs/specification/10-nikaia-light.md)'s
            // *a raw internal error reaching you is a Nikaia bug*, met by the
            // compiler itself. Found by writing
            // `comptime PRIMES: Array[i64, 4] = [2, 3, 5, 7]`, which is a
            // correct program and crashed the build.
            "const" => "NK1166",
            other => unreachable!("no code for `{other}`"),
        };
        // **An error, one of several, is said as that** (ADR-280 D30).
        let said = match several_errors(found) {
            Some(members) => {
                let text = found.text();
                message(&text, &want.text()).replace(
                    &format!("`{text}`"),
                    &format!("an error, one of {}", members_named(members)),
                )
            }
            None => message(&found.text(), &want.text()),
        };
        let finding = Finding {
            severity: Severity::Error,
            span,
            code: code.to_string(),
            message: said,
            notes: Vec::new(),
            help: Some(match several_errors(found) {
                Some(_) => "Take it apart first: `match error { … }`, and use what each arm \
                            binds."
                    .to_string(),
                None => convert(found, want),
            }),
            labels: Vec::new(),
        };
        // **Unless the walk may still show it is not negative** (D32): the
        // refusal waits for its answer.
        if let Some(at) = waiting
            && let Some(slots) = self.s.flow.unsigned_slots.get_mut(&(span.at() as i64))
        {
            slots[at].refusal = Some(finding);
            return;
        }
        self.s.findings.push(finding);
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
            let (value, into) = (&slot.written, slot.into);
            let mut finding = slot.refusal.unwrap_or_else(|| Finding {
                severity: Severity::Error,
                span: slot.span,
                code: "NK1102".to_string(),
                message: format!(
                    "This is `{}`, but `{}` is wanted here.",
                    slot.found.text(),
                    slot.want.text()
                ),
                notes: Vec::new(),
                help: None,
                labels: Vec::new(),
            });
            finding.notes.push(format!(
                "{} `{}` goes into {} `{into}` on its own only where the compiler can show it \
                 is not negative.",
                match an_or_a(&slot.found.text()) {
                    "an" => "An",
                    _ => "A",
                },
                slot.found.text(),
                an_or_a(&into)
            ));
            finding.help = Some(format!(
                "Write `{value} as {into}`, which stops the program on a negative value, or \
                 `assert {value} >= 0` before this line."
            ));
            self.s.findings.push(finding);
        }
    }

    /// `NK1108` for a condition that is not a `bool` (`tools/check_exprs.nika`).
    fn expect_bool(&mut self, found: &Ty, span: &Span, why: &str) {
        self.s.expect_bool(found, span, why);
    }

    /// A written call to something that can fail (`NK2605`).
    ///
    /// ADR-025 D1 states the rule for the calls **nobody wrote** - a block's
    /// closing brace, a loop's step - and says it is "exactly as a written
    /// call would" behave. The written call is the case the rule was
    /// generalised *from*, and it had no code: a function calling something
    /// that can fail and declaring nothing lowered in silence, and the ledger
    /// then published `[fn."ruft"] signature = "() -> String"` with no
    /// `throws` at all - a committed file that other programs read
    /// ([ADR-020](../../../../docs/specification/adr/adr-020.md)) saying a
    /// function cannot fail when its body can. `rustc` was what refused the
    /// program, about a file the author never wrote, which Part III C.1 calls
    /// a bug in this compiler.
    ///
    /// Reported only where the callee's contract **says** it can fail, which
    /// is what makes this need no guess: the ledger is the fact, and a callee
    /// no ledger describes says nothing here (the same silence `NK2502`'s
    /// method calls keep, C.4). So it never refuses a program that is right,
    /// and it grows as the ledger does.
    fn may_fail_here(&mut self, key: &str, contract: &FnContract, span: &Span) {
        // **`NK2108`: a call that can fail stands between a part taken out of
        // a `mut` parameter and its giving back.** Asked of the ledger's
        // word, as everything here is; a callee nothing describes says
        // nothing (C.4).
        if !contract.fails_with.is_empty() {
            self.s
                .a_way_out("fails", key.rsplit("::").next().unwrap_or(key), span);
        }
        // **Before the early return**, because the guard's question is asked of
        // exactly the calls this one declines to report: inside a `catch`,
        // `self.s.flow.caught` is what sends this function home, and a call that
        // carries a `throws` is precisely what gives that `catch` something to
        // do ([ADR-308](../../../docs/specification/adr/adr-308.md)).
        if !contract.fails_with.is_empty() {
            if let Some(guarded) = &mut self.s.flow.guarded {
                guarded.fallible = true;
                guarded.thrown.extend(contract.fails_with.iter().cloned());
            }
            self.newly_reaches_a_handler(key, span);
            // **And the lambda this may be inside**
            // ([ADR-277](../../docs/specification/adr/adr-277.md) D7). Here for
            // the same reason the line above is here: this is the one place
            // every written call — free or method — has its callee's contract
            // in hand, and a `catch` further out does not change what the
            // *lambda* was seen to do.
            //
            // **A `catch` further in does.** `fs::read(n, …) catch { "" }`
            // inside the body handles the failure before the lambda returns,
            // so the lambda cannot fail and there is nothing for `NK2606` to
            // say — its own help names this `catch` as the way out. The
            // question is `handed.caught`, not `self.s.flow.caught`, because only the
            // first stops at the lambda's edge.
            if let Some(handed) = self.s.handed_over.as_mut().filter(|h| !h.caught) {
                handed.fails = true;
                // **And `NK2606` is then the whole message.** Where the type
                // the lambda was handed to declares no failure, the function
                // *around* the lambda is not the one that has to answer for it
                // — the type is the contract, and telling the reader to write
                // `throws` on a function that does not fail would be a second
                // message for one mistake, in the wrong place. Where the type
                // **does** allow failing, the failure travels to the caller by
                // [ADR-288](../../docs/specification/adr/adr-288.md) D15 and
                // `NK2605` is right.
                if !handed.promised.may_fail {
                    return;
                }
            }
        }
        if contract.fails_with.is_empty() || self.s.flow.throwing || self.s.flow.caught {
            return;
        }
        // A grammar action is not code inside a function, and its failure does
        // not travel to one: past the `=>` it leaves the parser as the Nikaia
        // error it is and reaches the `catch` beside the `dsl`
        // ([ADR-023](../../../../docs/specification/adr/adr-023.md) D9). So
        // there is no `throws` to demand and no function to name.
        //
        // **Said here rather than by `current` being `None`**, which is what
        // said it until a rule's answers needed a key of their own to land
        // under ([ADR-186](../../../../docs/specification/adr/adr-186.md)):
        // the two facts are *this is not a
        // function* and *these calls belong to this entry*, and only the first
        // of them is this refusal's.
        if self.s.inside_an_action.is_some() {
            return;
        }
        // A `test` or a `bench` body is the other place with no function to
        // name, and there `current` is still `None`.
        let Some(function) = self.s.current.clone() else {
            return;
        };
        self.s.findings.push(Finding {
            severity: Severity::Error,
            span: *span,
            code: "NK2605".to_string(),
            message: format!("This function can fail, because `{key}` can fail."),
            notes: vec![
                format!(
                    "`{key}` can fail with {}.",
                    contract
                        .fails_with
                        .iter()
                        .map(|error| format!("`{error}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                "A call that can fail looks like any other call, so its failure passes \
                 straight through this function unless you handle it."
                    .to_string(),
            ],
            help: Some(format!(
                "Add `throws` to `{function}`, or handle it at the call: `… catch {{ … }}`."
            )),
            labels: Vec::new(),
        });
    }

    /// **`NK2402`: an error that newly reaches a `catch` is named once**
    /// ([ADR-308](../../docs/specification/adr/adr-308.md) D11).
    ///
    /// A `catch` handles everything that reaches it and `throws` names no types
    /// at a signature, so the set arriving at a handler is **open**: it grows
    /// whenever a callee gains a failure. The handler is still a correct
    /// program and still handles the new error — as it handles everything. What
    /// was missing is not a refusal. It is that nobody was told.
    ///
    /// So this is a **warning**, printed and stopping nothing, and it is given
    /// **once**: the commit of the ledger diff is the acknowledgement (D2), and
    /// after it the new set is the baseline. No marker in the source, nothing
    /// to type at the handler.
    ///
    /// **A handler that matches and one that does not are treated alike** (D3),
    /// because the question is the same for all three shapes — is this new
    /// error right where it landed? What the author does about it is theirs,
    /// and both answers are legitimate.
    ///
    /// The caret is on the **call** rather than on the `catch`, because that is
    /// what the sentence is about: *this call brings something new in here*.
    fn newly_reaches_a_handler(&mut self, key: &str, span: &Span) {
        let Some(gained) = self.newly.throws.get(key) else {
            return;
        };
        self.s.newly_reaches_a_handler(key, gained, span);
    }

    /// **`NK2403`: a value's cleanup moved with a contract**
    /// ([ADR-297](../../docs/specification/adr/adr-297.md), ADR-094 D5).
    ///
    /// A callee that keeps what it is given runs its cleanup when it is done
    /// with it; one that only reads it lends it, and the cleanup runs at the end
    /// of the caller's block. The call site shows neither, which ADR-094 D5
    /// accepts - and what it owes for that is this: when a change to the callee
    /// moves the point, on a type whose teardown does something, every call
    /// that hands it a name is told once.
    ///
    /// A **warning**, as `NK2402` is, and for its reason: the program is
    /// correct either way, and committing the ledger diff is the
    /// acknowledgement. Only a **name**: a temporary's cleanup was at the end of
    /// its statement, and where inside a call that is, nobody can observe.
    fn a_cleanup_that_moved(
        &mut self,
        key: &str,
        param: &str,
        want: &Ty,
        given: Option<&Expr>,
        span: &Span,
    ) {
        let Some(moved) = self
            .newly
            .keeps
            .get(key)
            .and_then(|all| all.iter().find(|kept| kept.param == param))
        else {
            return;
        };
        self.s.a_cleanup_that_moved(
            self.world, key, param, moved.now, &moved.ty, want, given, span,
        );
    }

    /// Where a method call stands in ADR-023 D8's propagation, for the emitter
    /// ([`Checked::fallible_methods`]).
    ///
    /// Every method call reaches this, and `fails` says whether the ledger
    /// describing its callee declares `throws`. A call whose receiver or whose
    /// method could not be resolved arrives with `false`, which is not "it
    /// cannot fail" but "there is no answer" - and it lands in the set that
    /// *removes* the pair, so an unresolved call leaves the statement's name
    /// alone rather than speaking for it.
    /// [`Checked::method_options`]: the callee's options, in the declaration's
    /// order, for the emitter to make positional.
    ///
    /// Recorded from the **contract** and not from the call, so a call that
    /// leaves an option out still gets its default — which is the half the
    /// emitter could not work out on its own either.
    fn method_options(&mut self, method: Ident, contract: &FnContract, span: &Span) {
        self.s.method_options(self.world, method, contract, span);
    }

    fn method_propagates(&mut self, method: Ident, fails: bool, span: &Span) {
        self.s.method_propagates(self.world, method, fails, span);
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
    fn a_task_takes_these(&mut self, body: &Expr, span: &Span) {
        self.s.a_task_takes_these(self.world, body, span);
    }

    /// **A key in the brackets of a map whose keys are owned**
    /// ([ADR-293](../../docs/specification/adr/adr-293.md) D27).
    ///
    /// The brackets went through `index::at` whatever was in them, and `at` is
    /// for a **position**: it turns a number into a `usize` and passes a view
    /// through. So a map keyed by an owned `String` could not be written with a
    /// `String` (no `at` for one) or with a literal (a `&str` where the map
    /// keeps a `String`), and a map keyed by an `i64` had its key turned into a
    /// `usize` - each `rustc`'s words about a file nobody wrote.
    ///
    /// A key is not a position. **Written**, the map keeps it: it goes in by
    /// value, a literal is built into text of its own where it stands (as
    /// ADR-282 does for every other place that keeps text), a name is handed
    /// over (`NK2105` if it is used again), and a **view** is refused with the
    /// sentence ADR-282 has for text kept where only a view was given.
    /// **Read**, the map keeps nothing: the key is lent, or goes in as it is
    /// where it is a view already.
    ///
    /// A map keyed by **views** is left exactly as it was: every example keys
    /// its maps that way, and a view goes through `at` untouched.
    fn a_map_key(&mut self, keys: &Ty, found: &Ty, index: &Expr, writing: bool, span: &Span) {
        // **A map whose key type nothing pinned is keyed by what goes in the
        // brackets** (0.0.235, issue #169). `collections::HashMap()`
        // with no annotation has `?` keys, and the brackets went through `at`,
        // which made `m[1] = 2` a map keyed by `usize` and `m[k]` a read the
        // language below refused. Where the key is a known value that is not
        // a view, it is the map's key; a view or a key of no known type goes
        // through `at` as before, which is the path every map of views takes.
        // **A map whose key type nothing pinned** (0.0.235, issue #169
        // issue #169). `collections::HashMap()` with no annotation has `?` keys,
        // and the brackets went through `at`, which is for a **position**: it
        // made `m[1] = 2` a map keyed by `usize` and `m[k]` a read the
        // language below refused. A key is not a position, and the receiver is
        // known to be a map, so neither side goes through `at` any more: a
        // write hands the key over as it is, and a read of a key that is not
        // already a view goes through `index::key`, which lends a number and
        // passes a view through - the one spelling that is right whatever the
        // map turns out to be keyed by, since this checker cannot say.
        if keys.is_unknown() {
            let form = match (writing, found.is_a_view()) {
                (true, _) => KeyForm::Handed,
                (false, true) => KeyForm::AsIs,
                (false, false) => KeyForm::Either,
            };
            self.s
                .facts
                .map_keys
                .insert((value_node(index), writing), form);
            return;
        }
        if keys.is_a_view() {
            return;
        }
        let form = match writing {
            true => {
                if self.text_literal(keys, index, true).is_none() {
                    self.expect_kept(
                        found,
                        keys,
                        Some(index),
                        "a map keeps its keys",
                        *span,
                        "assign",
                        |found, want| {
                            format!("This key is `{found}`, but the map's keys are `{want}`.")
                        },
                    );
                    self.hands_over(index, found, "used as a map key", span);
                }
                KeyForm::Handed
            }
            // **A name that is a view below whatever its type says** - a
            // `let` bound to a map read beside a jump, `let at = m[k] ??
            // continue` - is a reference already, and a `&` in front of it
            // was a `&&String` the map has no `Borrow` for (found moving
            // `describe`'s draft into Nikaia, #125).
            //
            // **And so is a parameter the function is lent** by inference
            // (ADR-279): `place: (i64, String)` only read is a `&(i64,
            // String)` below, and `&place` was a key the map has no `Borrow`
            // for (found moving `contracts::keep`'s plan into Nikaia, #125).
            false => match found.is_a_view()
                || matches!(index, Expr::LitStr { .. })
                || matches!(index, Expr::Variable(name, _)
                    if self.s.binding(self.parsed.text(*name)).is_some_and(|local| local.lent))
                // A `let` over a place, or a part of a tuple taken apart out
                // of one (#465), is a view below too.
                || self.s.a_lent_let(self.world, index)
                || (!matches!(found, Ty::Named { name, .. } if matches!(ty::base(name), "String" | "str"))
                    && self.a_lent_parameter(index))
            {
                true => KeyForm::AsIs,
                false => KeyForm::Lent,
            },
        };
        self.s
            .facts
            .map_keys
            .insert((value_node(index), writing), form);
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
        // `name.to_string()` is `name` (ADR-282 D8): what is handed over is it.
        if let Expr::MethodCall { receiver, .. } = value
            && self.s.facts.text_as_is.contains(&value_node(receiver))
        {
            return self.hands_over(receiver, ty, to, span);
        }
        let Some(&at) = self
            .s
            .flow
            .read_index
            .get(&(value_node(value), span.at() as i64))
        else {
            return;
        };
        if !matches!(value, Expr::Variable(_, _) | Expr::Field { .. }) {
            return;
        }
        let (path, seq) = (
            self.s.reads_on_paths[at as usize].path.clone(),
            self.s.reads_on_paths[at as usize].seq,
        );
        self.s.hands_over_path(self.world, path, ty, seq, to, span);
    }

    /// **A `??` in a position that only reads it lends its left side**
    /// (ADR-279 D5): recorded for the emitter, and no longer pending.
    fn a_pending_coalesce_is_lent(&mut self, given: &Expr) -> bool {
        if !matches!(given, Expr::Coalesce { .. }) {
            return false;
        }
        let key = value_node(given);
        let Some(at) = self
            .s
            .flow
            .pending_coalesces
            .iter()
            .position(|p| p.key == key)
        else {
            return false;
        };
        let pending = self.s.flow.pending_coalesces.remove(at);
        self.s
            .facts
            .lent_coalesces
            .insert(pending.key, opened_text(&pending.opened).to_string());
        true
    }

    /// The same for a `??` of text, where the position is a comparison
    /// (issue #98).
    fn a_pending_coalesce_of_text_is_lent(&mut self, given: &Expr) -> bool {
        let key = value_node(given);
        let of_text = self
            .s
            .flow
            .pending_coalesces
            .iter()
            .any(|p| p.key == key && p.opened == ".as_deref()");
        of_text && self.a_pending_coalesce_is_lent(given)
    }

    /// Every `??` of the statement just walked that no reading position lent
    /// is taken (ADR-279 D5).
    fn coalesces_not_lent_are_taken(&mut self) {
        self.s.coalesces_not_lent_are_taken(self.world);
    }

    /// A read of a name, recorded once per expression and statement.
    fn a_read(&mut self, expr: &Expr, path: String, span: &Span) {
        self.s.a_read(expr, &path, span);
    }

    /// `p.name` extends the read of `p` it stands on: what is read is the part
    /// ([ADR-293](../../docs/specification/adr/adr-293.md) D32).
    fn a_read_of_a_part(&mut self, base: &Expr, expr: &Expr, field: &str, span: &Span) {
        self.s.a_read_of_a_part(base, expr, field, span);
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

    /// The same, for the `.await` ([`Checked::pausing_methods`]).
    ///
    /// Separate from `method_propagates` because the two answers are separate:
    /// a call can fail without pausing and pause without failing, and the two
    /// sets are narrowed independently. Both are recorded at every method call,
    /// including the ones nothing could be established about - `false` there is
    /// "there is no answer", and it lands in the set that *removes* the pair.
    fn method_pauses(&mut self, method: Ident, pauses: bool, span: &Span) {
        self.s.method_pauses(self.world, method, pauses, span);
    }

    /// Whether this method call is an **eager** walk of a sequence whose step
    /// can fail ([ADR-025](../../docs/specification/adr/adr-025.md) D1).
    ///
    /// Asked twice — once for the `?` the emitter writes and once for the
    /// refusal — so it is one sentence rather than two that have to agree.
    fn walks_a_failing_sequence(&self, on: &Ty, contract: &FnContract) -> bool {
        self.s.walks_a_failing_sequence(on, contract)
    }

    /// `NK2701` for a **walk** rather than for a loop
    /// ([ADR-025](../../docs/specification/adr/adr-025.md) D1).
    ///
    /// The same rule and the same code as a `for` over the same sequence: a
    /// step that can fail fails the function around it, and nothing marks the
    /// call. The message says *a step of what it walks* rather than *a turn of
    /// this loop*, because that is what a reader is looking at.
    ///
    /// **It is the half whose absence was a wrong answer.** `io::lines().count()`
    /// used to compile and count the failures as lines, because the entry said
    /// `-> i64` and nothing asked what a step of the receiver does.
    fn a_walk_of_a_failing_sequence(&mut self, method: Ident, span: &Span) {
        self.s
            .a_walk_of_a_failing_sequence(self.world, method, span);
    }

    /// A loop over something whose step **pauses**
    /// ([ADR-172](../../docs/specification/adr/adr-172.md) D1).
    ///
    /// One thing follows and it is not a refusal: the emitter writes the loop
    /// as one that gives its thread up. Nothing is reported, because nothing is
    /// wrong — a `for` over a stream is an ordinary program and the whole of
    /// D1 is that it stops holding a thread it does not need.
    ///
    /// **The positive word and not the absence of `sync`**, which is the
    /// decision this reads rather than a shortcut into it: the absence is
    /// *nobody said*, a `map`'s step is exactly that, and writing `.await` on
    /// it would be `rustc` refusing a correct program about a file nobody wrote
    /// (Part III, C.1 and C.4).
    fn pausing_step(&mut self, over: &Ty, span: &Span) {
        self.s.pausing_step(over, span);
    }

    /// A loop over something whose step can fail (ADR-025 D1).
    ///
    /// Two things follow, and they are the two halves of the decision: the
    /// enclosing function must declare `throws`, and the emitter has to make
    /// the step propagate. This records the second and reports the first.
    fn fallible_step(&mut self, over: &Ty, bindings: usize, span: &Span) {
        self.s
            .fallible_step(self.world, over, bindings as i64, span);
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

    /// ADR-303 D7: a value handed to a call this compiler cannot see the end of
    /// may reach a thread that call owns (`NK2502`).
    ///
    /// A Rust dependency may bring its own runtime, so "what does it do with
    /// what I gave it" has no answer here - and a thread of its own is among the
    /// answers. D7's first rule is therefore the same rule as `NK2501`'s, asked
    /// at a call instead of at a task.
    ///
    /// **Unlike `NK2501` this is not hypothetical at `user_parallelism = no`.**
    /// The switch bounds what the *program* runs at once (ADR-037 D2); a foreign
    /// runtime's threads are not the program's, so the crossing is real at both
    /// settings and so is the refusal.
    fn crosses_into_an_unseen_call(
        &mut self,
        callee: &str,
        handed: Arguments<'_>,
        span: &Span,
        why: &str,
    ) {
        self.s.crosses_into_an_unseen_call(
            self.world,
            callee,
            handed.args,
            handed.found,
            handed.config,
            handed.passed,
            span,
            why,
        );
    }

    /// **`NK1229`: a supervisor's child that would not start fresh**
    /// ([ADR-328](../../docs/specification/adr/adr-328.md), Part II 12.8): its
    /// function is called again for every attempt, so a `mut` binding it
    /// captures carries one attempt's changes into the next.
    fn a_child_that_keeps_state(&mut self, body: &Expr, span: &Span) {
        self.s.a_child_that_keeps_state(self.world, body, span);
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

    /// Whether any ledger or this unit declares a **type** by this name.
    ///
    /// Presence and not fields, which is the difference from [`Self::fields_of`]:
    /// *an empty field list means nothing recorded and not nothing inside*
    /// (Part III 15.2), so a described foreign type has an entry and no fields,
    /// and it is declared.
    fn declares_a_type(&self, name: &str) -> bool {
        self.s.declares_a_type(self.world, name)
    }

    /// **`NK1162`: `m[k] += 1` on a map**
    /// ([ADR-293](../../docs/specification/adr/adr-293.md) D2).
    ///
    /// A compound assignment reads the slot and writes it, and since D1 the
    /// read is a **`T?`** — so the form has to say what an absent key counts
    /// as, which is the question [ADR-080](../../docs/specification/adr/adr-080.md)
    /// D2 left open. `m[k] = (m[k] ?? 0) + 1` is that sentence, and the message
    /// carries it.
    ///
    /// **A sequence is untouched**: `xs[i] += 1` reads a `T` and writes a `T`,
    /// because a list has a value at every index it has at all (D3).
    fn a_compound_write_to_a_map_slot(&mut self, target: &Expr, span: &Span) {
        self.nikaia(|state, world, host| {
            check_core_pilot::check_core_a_compound_write_to_a_map_slot(
                state, world, host, target, span,
            )
        })
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

    /// `NK1206` (`tools/check_exprs.nika`).
    fn a_loop_over_a_nullable(&mut self, over: &Ty, iter: &Expr, span: &Span) {
        self.s.a_loop_over_a_nullable(self.world, over, iter, span);
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

    /// The table a `comptime` map crosses as, and whether the reader was told
    /// what is wrong (`tools/check_consts.nika`).
    fn a_table_that_crosses(
        &mut self,
        bound: &str,
        want: &Ty,
        pairs: &[build_time::Value],
        span: &Span,
    ) -> (Option<(String, String)>, bool) {
        let (parsed, beside) = (self.parsed, self.beside);
        let table = self.s.a_table_that_crosses(
            self.world,
            bound,
            want,
            pairs,
            span,
            &|ty: &str, variant: &str| payload_of(parsed, beside, ty, variant),
            &|f: f64| format!("{f:?}"),
        );
        match table.found {
            true => (Some((table.below, table.written)), false),
            false => (None, table.said),
        }
    }

    /// **`[1, 2, 3]`**, and what its type is
    /// ([ADR-135](../../docs/specification/adr/adr-135.md) D1).
    ///
    /// The elements are walked in order and the first one that has a type is
    /// what the rest answer to. A type this checker could not work out is
    /// **silence** and not a disagreement: Stage 0 knows the type of rather
    /// less than half of what a program writes, and a refusal on doubt is
    /// [Part III C.4](../../docs/specification/30-nikaia-tooling.md)'s correct
    /// program refused.
    ///
    /// **Once per literal.** Three elements that disagree with the first are
    /// one mistake, so the walk stops reporting after the first pair - but it
    /// keeps *walking*, because an element is an expression and a mistake
    /// inside one is still a mistake.
    fn list_literal(&mut self, items: &[Expr], span: &Span) -> Ty {
        let mut agreed = Ty::Unknown;
        let mut kind: Option<(String, String)> = None;
        let mut said = false;
        // **A lambda in a list of a declared `fn` type takes that type**
        // (#497), as one bound by an annotated `let` does:
        // `let fs: Vec[fn(i64) -> i64] = [fn(x) { x * 2 }]`.
        let element = self
            .s
            .flow
            .lambda_expected
            .take()
            .and_then(|declared| match declared {
                Ty::Named { name, args, .. } if name == "Vec" => args.into_iter().next(),
                _ => None,
            });
        let founds: Vec<Ty> = items
            .iter()
            .map(|item| {
                if matches!(item, Expr::Closure { .. }) {
                    self.s.flow.lambda_expected =
                        element.clone().filter(|ty| matches!(ty, Ty::Fn { .. }));
                }
                let found = self.expr(item, span);
                self.s.flow.lambda_expected = None;
                found
            })
            .collect();
        for (item, ty) in items.iter().zip(&founds) {
            self.hands_over(item, ty, "put into a list", span);
        }
        // **A list with a `null` in it is a list of `T?`**, met as the arms of
        // a choice meet (`arms_meet_at_null`): `[null, 7]` holds `Some(7)`
        // below, where the `7` reached `rustc` bare beside a `None` (#312).
        if items.iter().any(|item| matches!(item, Expr::LitNull(_))) {
            let arms: Vec<(Option<&Expr>, Ty)> = items
                .iter()
                .zip(&founds)
                .map(|(item, ty)| (Some(item), ty.clone()))
                .collect();
            if let Some(element) = self.arms_meet_at_null(&arms) {
                return Ty::Named {
                    name: "Vec".to_string(),
                    args: vec![element],
                    view: false,
                };
            }
        }
        // **A text literal takes its neighbours' text** (ADR-282 D4): where
        // the list already holds text of its own, `["a", f"c"]` is a list of
        // `String` and the literal is constructed as one. A list holds one
        // type, so the literal has no other answer - and a **view** among the
        // elements is not a literal, so it still disagrees below.
        let holds_text = founds.iter().any(|found| *found == Ty::named("String"));
        let founds: Vec<Ty> = items
            .iter()
            .zip(founds)
            .map(|(item, found)| match (item, holds_text) {
                (Expr::LitStr { .. }, true) => {
                    self.s.facts.owned_texts.insert(value_node(item));
                    Ty::named("String")
                }
                _ => found,
            })
            .collect();
        let integers: Vec<Option<String>> = founds
            .iter()
            .map(|found| match found {
                Ty::Named {
                    name,
                    args,
                    view: false,
                } if args.is_empty() => Some(name.clone()),
                _ => None,
            })
            .collect();
        for (item, found) in items.iter().zip(founds) {
            // **A number beside text, where neither has a type yet.** A bare
            // `1` fits every numeric type and so it arrives here as `?` - which
            // is right, and which would let `[1, "two"]` past to be answered by
            // the backend about a file nobody wrote
            // ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)).
            // The **kind** is the part of a literal that is known without a
            // type, and two kinds that differ is a disagreement whatever the
            // types say.
            if let Some(shape) = element_kind(item, &found) {
                match &kind {
                    None => kind = Some((shape, found.text())),
                    Some((first, written)) if *first != shape && !said => {
                        said = true;
                        let (first, written) = (first.clone(), written.clone());
                        self.s
                            .a_list_whose_elements_disagree(&first, &written, &shape, &found, span);
                    }
                    Some(_) => {}
                }
            }
            if found.is_unknown() {
                continue;
            }
            if agreed.is_unknown() {
                agreed = found;
                continue;
            }
            // **Two integer types hold their common type** (ADR-285 D32),
            // whatever comes first: `[a, b]` over a `u32` and an `i64` is a
            // `Vec[i64]`.
            if let (
                Ty::Named {
                    name: a,
                    args: x,
                    view: false,
                },
                Ty::Named {
                    name: b,
                    args: y,
                    view: false,
                },
            ) = (&agreed, &found)
                && x.is_empty()
                && y.is_empty()
                && !is_literal(item)
                && let Some(common) = common_integer(a, b)
            {
                agreed = Ty::named(common);
                continue;
            }
            if !found.fits(&agreed) && !said {
                said = true;
                let first = agreed.clone();
                self.s.a_list_whose_elements_disagree(
                    &element_kind(item, &first).unwrap_or_else(|| "a value".to_string()),
                    &first.text(),
                    "a value",
                    &found,
                    span,
                );
            }
        }
        // **And the narrower elements are widened into it** (D32).
        if let Ty::Named {
            name: common,
            args,
            view: false,
        } = &agreed
            && args.is_empty()
            && let Some(common) = INTEGERS.iter().find(|n| **n == common.as_str())
        {
            for (item, own) in items.iter().zip(&integers) {
                if let Some(own) = own
                    && own != common
                    && !is_literal(item)
                    && common_integer(own, common) == Some(*common)
                {
                    self.s
                        .facts
                        .wrapped
                        .insert(value_node(item), Wrap::Widen(common.to_string()));
                }
            }
        }
        // **A lambda beside a function value is kept as that value is**
        // (Part I 5.3): `[adder(1), fn(x) { x * 2 }]` is a list of one type,
        // and the type is the one the other elements say.
        if matches!(agreed, Ty::Fn { .. }) {
            for item in items {
                self.a_kept_lambda(&agreed, item, span);
            }
        }
        Ty::Named {
            name: "Vec".to_string(),
            args: vec![agreed],
            view: false,
        }
    }

    /// Whether this program writes `impl Error for` the type, in this file or
    /// another of the package.
    fn implements_error(&self, ty: &str) -> bool {
        check_items::implements_error(&self.world.here, &self.world.beside, ty)
    }

    /// **A length beside a view is checked at the call**
    /// ([ADR-302](../../docs/specification/adr/adr-302.md) D6).
    ///
    /// `read(fd, buf, count)` declares `count: usize`, and the two parameters
    /// are **one fact** in C: the pointer says where and the count says how far.
    /// A call that passes a longer count is the buffer overrun the boundary
    /// exists to stop, and it is refused here rather than by the operating
    /// system.
    ///
    /// **What makes a pair is the declaration's own types**: a `&[T]` and, right
    /// after it, a `usize`. The type is what says *length* rather than the name,
    /// because `usize` is not a type this language's own values have
    /// ([ADR-285](../../docs/specification/adr/adr-285.md) D1) — a length here
    /// is an `i64` and the machine-width type left the surface a program can
    /// write. A declaration that writes `usize` beside a buffer is therefore
    /// saying C's `size_t`, and this is what that says.
    ///
    /// **Two shapes are accepted and everything else is refused** (D2's *narrow
    /// on purpose*): the buffer's own `len()`, and a constant the buffer's
    /// length is known to cover — which is an `Array[T, N]`, whose length is
    /// part of its type ([ADR-152](../../docs/specification/adr/adr-152.md) D1).
    /// A **zero** is accepted against any buffer, because no length is smaller.
    ///
    /// Nothing is claimed where the callee is not a foreign declaration, or
    /// where this checker could not work the buffer's type out (Part III, C.4).
    fn a_length_that_fits_its_buffer(
        &mut self,
        written: &str,
        wanted: &[(String, Ty)],
        given: &[Expr],
        found: &[Ty],
        span: &Span,
    ) {
        self.s
            .a_length_that_fits_its_buffer(self.world, written, wanted, given, found, span);
    }

    /// **A literal takes the use's type, whichever literal it is**: an array
    /// where an `Array[T, N]` is wanted (ADR-152 D4), and text of its own where
    /// a `String` is ([ADR-282](../../docs/specification/adr/adr-282.md) D4).
    /// One entry for every position that is a use, so a position cannot ask
    /// one of the two questions and forget the other.
    fn literal_by_use(&mut self, found: &Ty, want: &Ty, value: &Expr, span: &Span) -> Option<Ty> {
        self.a_kept_lambda(want, value, span);
        self.array_literal(found, want, value, span)
            .or_else(|| self.text_literal(want, value, true))
            .or_else(|| self.tuple_literal(found, want, value))
            .or_else(|| self.list_of_tuple_literals(found, want, value))
            .or_else(|| self.collect_by_use(found, want, value))
            .or_else(|| number_where_no_number_is(want, value))
    }

    /// A list of tuple literals where a list of tuples with text is wanted
    /// (`tools/check_texts.nika`).
    fn list_of_tuple_literals(&mut self, found: &Ty, want: &Ty, value: &Expr) -> Option<Ty> {
        self.s
            .list_of_tuple_literals(self.world, found, want, value)
    }

    /// A `collect` that builds what its position asks
    /// (`tools/check_numbers2.nika`).
    fn collect_by_use(&mut self, found: &Ty, want: &Ty, value: &Expr) -> Option<Ty> {
        self.s.collect_by_use(self.world, found, want, value)
    }

    /// **A hull a kept lambda captures, it captures a handle of** (ADR-230
    /// D4, ADR-312 D5): handing a hull on duplicates the count, so the name
    /// outside is still there on the next line. Asked wherever a lambda is
    /// kept - a field, a `let`, and a parameter the callee keeps (#536).
    fn a_kept_lambdas_hulls(&mut self, value: &Expr) {
        self.s.a_kept_lambdas_hulls(self.world, value);
    }

    /// **What a kept lambda captures, it takes** (Part I 8.3, ADR-328 D7, #95
    /// step 6): it is called after the statement that made it, so a value a
    /// move takes away goes with it and is `NK2101` where it is used again. One
    /// that pauses owns it once and counts the calls that borrow it - each
    /// call's future must own what it reads - which the emitter writes from
    /// `kept_owned`: no copy.
    fn a_kept_lambda_takes_these(&mut self, value: &Expr, pauses: bool, span: &Span) {
        self.s
            .a_kept_lambda_takes_these(self.world, value, pauses, span);
    }

    /// **A lambda where a function value is kept** is one shared closure
    /// below, and one that may pause hands back a boxed future.
    fn a_kept_lambda(&mut self, want: &Ty, value: &Expr, span: &Span) {
        let want = match want {
            Ty::Nullable(inner) => inner.as_ref(),
            other => other,
        };
        // **An element of a list is kept by the list** (Part I 5.3): `let fs:
        // Vec[fn(i64) -> i64] = [fn(x) { x + 1 }]` holds each lambda as the
        // shared closure a field holds, and a bare closure among them was a
        // type of its own below.
        if let (Ty::Named { name, args, .. }, Expr::ListLit { items, .. }) = (want, value)
            && matches!(name.as_str(), "Vec" | "List" | ty::ARRAY)
            && let Some(element) = args.first()
        {
            for item in items {
                self.a_kept_lambda(element, item, span);
            }
            return;
        }
        if let (
            Ty::Fn {
                is_sync,
                can_throw: throws,
                ..
            },
            Expr::Closure { .. },
        ) = (want, value)
        {
            self.s
                .facts
                .kept_lambdas
                .insert(value_node(value), (!is_sync, *throws));
            self.a_lambdas_text_is_its_own(want, value);
            self.a_kept_lambdas_hulls(value);
            self.a_kept_lambda_takes_these(value, !is_sync, span);
        }
        // **A function of this program, named**, is the closure that calls it:
        // the call is typed here so that it is lowered as a written one is.
        if let (
            Ty::Fn {
                params,
                is_sync,
                can_throw: throws,
                ..
            },
            Expr::Variable(name, _),
        ) = (want, value)
        {
            let named = self.parsed.text(*name).to_string();
            if self.s.lookup(&named).is_none() && self.world.own.functions.contains_key(&named) {
                self.s.facts.kept_functions.insert(
                    ((span.at() as i64), named),
                    (!is_sync, *throws, (params.len() as i64)),
                );
                let (names, call) = kept_function_call(self.parsed, *name, params.len());
                let frame = names
                    .iter()
                    .zip(params)
                    .map(|(n, ty)| self.local_free(self.parsed.text(*n).to_string(), ty.clone()))
                    .collect();
                self.s.scope.push(frame);
                self.expr(&call, span);
                self.s.scope.pop();
            }
        }
    }

    /// **A lambda's text literal is built where its type keeps text**: the
    /// value a lambda ends in is handed to whoever calls it, as a function's
    /// is (ADR-282 D4).
    fn a_lambdas_text_is_its_own(&mut self, want: &Ty, value: &Expr) {
        self.s.a_lambdas_text_is_its_own(self.world, want, value);
    }

    /// A tuple of text literals where a tuple with text is wanted
    /// (`tools/check_texts.nika`).
    fn tuple_literal(&mut self, found: &Ty, want: &Ty, value: &Expr) -> Option<Ty> {
        self.s.tuple_literal(self.world, found, want, value)
    }

    /// **A text literal where a `String` is wanted is a `String`**
    /// ([ADR-282](../../docs/specification/adr/adr-282.md) D4), and hands back
    /// the type it became.
    ///
    /// `"x"` is a view of static text, and a view put where text of its own
    /// is declared was refused until the program wrote `.to_string()` - the
    /// most common refusal a newcomer met, and one that protected nothing: the
    /// text is a constant in the program, so making a `String` of it is
    /// *constructing* a value, as `[1, 2]` constructs the `Vec` it stands in,
    /// and not copying text the program had. ADR-005 §3's rule is about the
    /// second, and it stands: a **view** of text the program has still needs
    /// its `.clone()`.
    ///
    /// `owned` says whether the position keeps what it is given. It is `false`
    /// for an argument the callee only reads, whose parameter is a `&str` below
    /// (D3): the literal fits it as it is, the answer is still *a `String`*
    /// for the fit, and nothing is recorded, so nothing is allocated.
    ///
    /// Also through `T?`, whose wrap is the caller's (`wraps_into_nullable`),
    /// and through a list literal where a `Vec[String]` is wanted, element by
    /// element - but only where **every** element is a text literal, so a list
    /// that mixes in a view keeps the refusal it had.
    /// The function type of `on`'s field called `method`, where `on` is a
    /// struct with such a field and no method of that name.
    fn a_field_that_is_code(&self, on: &Ty, method: Ident) -> Option<Ty> {
        self.s.a_field_that_is_code(self.world, on, method)
    }

    /// **A call of a field that holds a function**, `button.on_click(4)`:
    /// what it is given, the lock column and what the emitter needs, and the
    /// call's result - `None` where `on` has no such field. `outer_inside` is
    /// whether a lock was already open around this call. Asked by the plain
    /// call and by `x?.on_click(4)` alike (#533), which differ only in whether
    /// the call happens.
    fn a_called_field(
        &mut self,
        on: &Ty,
        receiver: &Expr,
        method: Ident,
        args: &[Expr],
        outer_inside: bool,
        span: &Span,
    ) -> Option<Ty> {
        let Some(Ty::Fn {
            params,
            result,
            is_sync,
            can_throw: throws,
        }) = self.a_field_that_is_code(on, method)
        else {
            return None;
        };
        self.arguments_given(args, &params, false, None, span);
        self.kept_arguments_are_its_own(args, &params);
        // **A call through a function field reaches what was
        // stored there** (ADR-230 D1, D2): recorded for the lock
        // column, and asked here where a lock is already open.
        if let Ty::Named { name: owner, .. } = on {
            let key = format!("{owner}.{}", self.parsed.text(method));
            if let Some(current) = self.s.current.clone()
                && self.s.task_bindings.is_empty()
            {
                self.s
                    .methods
                    .entry(current)
                    .or_insert_with(MethodCalls::empty)
                    .fields_called
                    .insert(key.clone());
            }
            // The door this call stands in, not this call: the flag
            // was set above for the call's own block.
            let holds = self.world.own.code_locks.get(&key).copied();
            if let Some(holds) = holds {
                let own = std::mem::replace(&mut self.s.inside_a_door, outer_inside);
                self.s.a_lock_inside_a_lock(&key, &holds, span);
                self.s.inside_a_door = own;
            }
        }
        self.s
            .facts
            .field_calls
            .insert(value_node(receiver), !is_sync);
        if let Some(current) = self.s.current.clone() {
            let entry = self
                .s
                .methods
                .entry(current)
                .or_insert_with(MethodCalls::empty);
            entry.code_pauses |= !is_sync;
            entry.code_fails |= throws;
        }
        if throws {
            self.s
                .flow
                .fallible_methods
                .insert(((span.at() as i64), self.parsed.text(method).to_string()));
        }
        Some((*result).unwrap_or_else(|| Ty::named("()")))
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

    /// A list literal where an `Array[T, N]` is wanted
    /// (`tools/check_numbers2.nika`).
    fn array_literal(&mut self, found: &Ty, want: &Ty, value: &Expr, span: &Span) -> Option<Ty> {
        self.s.array_literal(found, want, value, span)
    }
}

impl<'a> Checker<'a> {
    fn unwritable_field(&self, ty: &Ty) -> Option<(String, String)> {
        match self.s.unwritable_field(self.world, ty).as_slice() {
            [field, held] => Some((field.clone(), held.clone())),
            _ => None,
        }
    }

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

    /// A computed value written against the type its declaration states
    /// (`tools/check_consts.nika`).
    fn written_below(&self, value: &build_time::Value, want: Option<&Ty>) -> Option<String> {
        let (parsed, beside) = (self.parsed, self.beside);
        check_consts::written_below(
            &self.s,
            self.world,
            value,
            want,
            &|ty: &str, variant: &str| payload_of(parsed, beside, ty, variant),
            &|f: f64| format!("{f:?}"),
        )
    }

    /// **`NK1171`: a member a known type or module does not have**
    /// ([ADR-286](../../docs/specification/adr/adr-286.md) D36): what is written
    /// down is what can be called. `"x".as_str()` and `String::from("x")` are
    /// Rust's, and passed here untyped to be answered by the language below.
    /// Known is a type or module the program, `std`'s ledger or a crate's
    /// description writes down; a type parameter, a head only a `use` names
    /// and a foreign name are not claimed.
    fn a_member_nothing_writes_down(
        &mut self,
        head: &str,
        member: &str,
        method: bool,
        span: &Span,
    ) {
        self.s
            .a_member_nothing_writes_down(self.world, head, member, method, span);
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

    fn an_instantiation(&mut self, name: &str, args: &[Expr], span: &Span) {
        self.nikaia(|state, world, host| {
            check_core_pilot::check_core_an_instantiation(state, world, host, name, args, span)
        })
    }

    /// **A cast whose operand is a `for` binding**
    /// ([ADR-182](../../docs/specification/adr/adr-182.md) D1).
    ///
    /// `for n in NS { sum = sum + (n as i64) }` reached the language below as
    /// `n as i64` over a `&i32`, and what came back was *casting `&i32` as
    /// `i64` is invalid* with a way out that reads *dereference the
    /// expression* — a noun and an instruction about a file nobody wrote
    /// ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)).
    ///
    /// **Only a bare name**, because that is the only operand shape that can
    /// arrive as a view: a call, a literal and an arithmetic expression each
    /// come to a value, and a field read or an index is already taken apart by
    /// the lowering that wrote it.
    fn a_cast_over_a_lent_binding(&mut self, operand: &Expr, span: &Span) {
        self.s.a_cast_over_a_lent_binding(self.world, operand, span);
    }

    /// **A `let` whose declared type is a number and whose value is a `for`
    /// binding** ([ADR-182](../../docs/specification/adr/adr-182.md) D5).
    ///
    /// `for m in xs { let q: i64 = m }` reached the language below as
    /// `let q: i64 = m;` over a `&i64`, and what came back was *mismatched
    /// types*, with *consider dereferencing the borrow* as the way out — about
    /// a borrow the source does not contain
    /// ([Part III C.1](../../docs/specification/30-nikaia-tooling.md)).
    ///
    /// **Only where the annotation is a number**, because that is the whole of
    /// what `num::value` answers and the whole of what may be read through a
    /// view without a copy being **inserted**
    /// ([ADR-283](../../docs/specification/adr/adr-283.md) D3). A `let r: Row =
    /// <binding>` is a different question with a different answer, and it is
    /// issue #153's to carry until
    /// somebody decides it.
    fn a_number_read_through_a_lent_binding(&mut self, want: &Ty, value: &Expr, span: &Span) {
        let Ty::Named {
            name: want,
            args,
            view,
        } = want
        else {
            return;
        };
        if *view {
            return;
        }
        const NUMBERS: [&str; 9] = [
            "i32", "i64", "u8", "u32", "u64", "f64", "bool", "char", "scalar",
        ];
        if args.is_empty() && NUMBERS.contains(&want.as_str()) {
            self.a_cast_over_a_lent_binding(value, span);
            return;
        }
        // **And everything else is a refusal**
        // ([ADR-185](../../docs/specification/adr/adr-185.md) D1): a number is
        // `Copy` and reading one through a view inserts nothing, and a `struct`
        // is not — so the same answer here would be a copy the source did not
        // write, which [ADR-283](../../docs/specification/adr/adr-283.md) D3
        // forbids in as many words.
        let Expr::Variable(name, _) = value else {
            return;
        };
        let name = self.parsed.text(*name).to_string();
        // A number is copied out of the view, and one put where no number fits
        // is refused as that (#554).
        if !self.s.binding(&name).is_some_and(|local| local.lent) || self.number_shaped(value) {
            return;
        }
        let want = want.clone();
        self.s.a_copy_the_source_did_not_write(&name, &want, span);
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

    fn bind_local(&mut self, local: Local) {
        self.s.bind_local(local);
    }

    fn resolve(&self, name: &str) -> Option<(String, FnContract)> {
        resolve_call(self.world, name).map(|found| (found.key, found.contract))
    }

    /// Walk a call's arguments, telling a lambda what it will be handed.
    ///
    /// The types come from the callee's signature, so this can only run once
    /// the callee is known - which is why the resolution moved ahead of the
    /// walk (ADR-288). An argument whose parameter says nothing is walked
    /// exactly as it was before.
    ///
    /// `declared_here` says whether the signature came from **this program's**
    /// ledger rather than from `std`'s. It decides one thing and
    /// [ADR-277](../../docs/specification/adr/adr-277.md) D12 is why: a `std`
    /// entry's lambda type is a hand-written description of a **Rust**
    /// signature, which takes a plain closure whatever its `sync` column says,
    /// so the future shape must not be written for one.
    fn arguments_given(
        &mut self,
        args: &[Expr],
        expected: &[Ty],
        declared_here: bool,
        // The callee's contract, where one was resolved. Read for one question:
        // whether it **keeps** the parameter a lambda is landing in
        // ([ADR-277](../../docs/specification/adr/adr-277.md) D11).
        callee: Option<&FnContract>,
        span: &Span,
    ) -> Vec<Ty> {
        args.iter()
            .enumerate()
            .map(|(at, arg)| match (arg, expected.get(at)) {
                (
                    Expr::Closure {
                        params,
                        mutable,
                        body,
                        ..
                    },
                    Some(Ty::Fn {
                        params: given,
                        result,
                        is_sync,
                        can_throw: throws,
                    }),
                ) => {
                    // **A lambda handed to a parameter whose type may pause is
                    // written as a closure returning a boxed future**
                    // ([ADR-277](../../docs/specification/adr/adr-277.md) D11).
                    // Recorded here because it is a question about the
                    // parameter's **type**, which the emitter has no way to ask
                    // ([ADR-288](../../docs/specification/adr/adr-288.md)) — the
                    // same arrangement `lent_args` and `nullable_args` have, and
                    // keyed the same way.
                    if !is_sync && declared_here {
                        self.s
                            .facts
                            .future_lambdas
                            .insert(((span.at() as i64), (at as i64)));
                        // **And an `async` closure where the callee *runs* it**
                        // ([ADR-277](../../docs/specification/adr/adr-277.md)
                        // D1). The box is what a **kept** parameter needs, and
                        // a run parameter never did: `impl AsyncFn(A) -> R` is
                        // 1.37 ns/call against the box's 11.99, on a 0.31
                        // floor.
                        //
                        // **Run is the absence of `keeps`**, and a callee this
                        // walk could not resolve is read as run - which agrees
                        // with what the *declaration* writer does with the same
                        // absent answer. The two reading one column the same
                        // way is the whole of what keeps them from writing two
                        // shapes for one parameter.
                        let runs = callee.is_none_or(|c| {
                            c.signature
                                .as_ref()
                                .and_then(|s| s.arguments().get(at).cloned())
                                .is_none_or(|(name, _)| !c.keeps.contains(&name))
                        });
                        if runs {
                            self.s
                                .facts
                                .run_lambdas
                                .insert(((span.at() as i64), (at as i64)));
                        }
                    }
                    // **A parameter the callee keeps is a kept function
                    // value**, `sync` or not: the callee stores it. A `std`
                    // entry too, where its `keeps` names the parameter
                    // (`net::serve`'s handler, ADR-326 D1): its Rust takes the
                    // same `Kept` this program's own declaration would, so
                    // ADR-277 D12's plain closure is not what it describes.
                    if !self.runs_the_parameter(callee, at) {
                        self.s
                            .facts
                            .kept_lambdas
                            .insert(value_node(arg), (!is_sync, *throws));
                        self.a_kept_lambdas_hulls(arg);
                        self.a_kept_lambda_takes_these(arg, !is_sync, span);
                    }
                    if let Some(want) = expected.get(at) {
                        self.a_lambdas_text_is_its_own(want, arg);
                    }
                    let (came_to, paused) = self.lambda(
                        params,
                        mutable,
                        body,
                        given,
                        Promises {
                            may_pause: !is_sync,
                            may_fail: *throws,
                        },
                        span,
                    );
                    // **A lambda that paused, handed to a `std` entry**
                    // ([ADR-233](../../docs/specification/adr/adr-233.md) D1):
                    // kept for the call to lower to the entry's pausing
                    // counterpart, which only the call knows.
                    if paused && !declared_here {
                        self.s
                            .flow
                            .paused_args
                            .push((value_node(arg) as usize) as i64);
                    }
                    // **The lambda as the type it was handed to, with what its
                    // body comes to where that type does not say** (ADR-293
                    // D5): `fn($T) -> $U` binds `$U` from it, which is how a
                    // `map` knows its elements. Where the type does say, what it
                    // says stands - this is an answer for a variable, not a
                    // second opinion on a written result.
                    let result = match result.as_ref() {
                        Some(written) if !written.is_unknown() => Some(written.clone()),
                        _ => (!came_to.is_unknown()).then_some(came_to),
                    };
                    Ty::Fn {
                        params: given.clone(),
                        result: Box::new(result),
                        is_sync: *is_sync,
                        can_throw: *throws,
                    }
                }
                _ => {
                    // **A parameter that only borrows takes nothing out of
                    // `self`** (0.0.252): `quoted(self.attribute)` into
                    // `attribute: ref String` is lent, which is what the
                    // subject's own loan allows. Found moving the template
                    // splitter into Nikaia.
                    if !expected.get(at).is_some_and(Ty::is_a_view) {
                        self.a_field_of_a_borrowed_subject(arg, span, "passed");
                    }
                    let found = self.expr(arg, span);
                    // **A declared function, run by a parameter that may pause**
                    // (#516): the emitter wraps the one that never does.
                    let named = matches!(arg, Expr::Path(_, _))
                        || matches!(arg, Expr::Variable(name, _)
                            if self.s.binding(self.parsed.text(*name)).is_none());
                    if declared_here
                        && named
                        && self.runs_the_parameter(callee, at)
                        && matches!(expected.get(at), Some(Ty::Fn { is_sync: false, .. }))
                        && let Ty::Fn { params, .. } = &found
                    {
                        self.s
                            .facts
                            .sync_functions_run_async
                            .insert(((span.at() as i64), (at as i64)), params.len() as i64);
                    }
                    // **A kept value handed to a parameter that only runs
                    // it** is lent as the closure it holds.
                    if declared_here
                        && matches!(expected.get(at), Some(Ty::Fn { .. }))
                        && matches!(found, Ty::Fn { .. })
                        && self.runs_the_parameter(callee, at)
                        && !self.a_run_parameter(arg)
                        // A declared function named here is no kept value: it
                        // is passed as it is (#502).
                        && !matches!(arg, Expr::Path(_, _))
                        && !matches!(arg, Expr::Variable(name, _)
                            if self.s.binding(self.parsed.text(*name)).is_none())
                    {
                        self.s.facts.kept_args.insert(value_node(arg));
                    }
                    found
                }
            })
            .collect()
    }

    /// Whether parameter `at` is a by-value type variable that nothing else in
    /// the signature binds: no receiver, and no other parameter names it.
    fn decided_by_this_argument(
        &self,
        signature: &crate::contracts::Signature,
        wanted: &[(String, Ty)],
        at: usize,
    ) -> bool {
        self.s
            .decided_by_this_argument(signature, wanted, at as i64)
    }

    /// **A kept function takes its arguments by value**, so a text literal
    /// handed to one where it keeps text is built there (ADR-282 D4).
    fn kept_arguments_are_its_own(&mut self, args: &[Expr], params: &[Ty]) {
        self.s.kept_arguments_are_its_own(self.world, args, params);
    }

    /// Whether `expr` names a parameter of the function being walked that is
    /// code it only runs: a closure argument below, not a kept value.
    fn a_run_parameter(&self, expr: &Expr) -> bool {
        matches!(expr, Expr::Variable(name, _) if self.s.run_code.contains(self.parsed.text(*name)))
    }

    /// Whether the callee only **runs** its `at`th parameter: the absence of
    /// `keeps`, and run where no contract was resolved.
    fn runs_the_parameter(&self, callee: Option<&FnContract>, at: usize) -> bool {
        callee.is_none_or(|c| {
            c.signature
                .as_ref()
                .and_then(|s| s.arguments().get(at).cloned())
                .is_none_or(|(name, _)| !c.keeps.contains(&name))
        })
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
        let frame = params
            .iter()
            .enumerate()
            .map(|(at, p)| {
                let local = self.lambda_parameter(
                    *p,
                    mutable,
                    given.get(at).cloned().unwrap_or(Ty::Unknown),
                    span,
                );
                if local.ty.is_unknown() {
                    self.s.flow.unknown_bound.insert(local.id as usize as i64);
                }
                local
            })
            .collect();

        let depth = self.s.scope.len() as i64;
        self.s.repeats.push(Repeats::a_lambda(depth));
        self.s.scope.push(frame);
        // The other door into a lambda's body, and it needs the same boundary
        // as the one in `expr`: which of the two a lambda arrives through is
        // whether the callee's signature typed its parameters, and that has
        // nothing to do with what a `break` in it may reach.
        let saved = self.enter_boundary("lambda");
        self.s.handed_over = Some(Handed {
            pauses: false,
            fails: false,
            caught: false,
            promised,
        });
        let tail = self.block(body);
        let seen = self.s.handed_over.take();
        self.leave_boundary(saved);
        self.s.scope.pop();
        self.s.repeats.pop();
        let paused = seen.as_ref().is_some_and(|seen| seen.pauses);
        if let Some(seen) = seen {
            self.s
                .a_handler_that_does_more_than_the_type_allows(&promised, &seen, span);
        }
        // **What its body comes to** ([ADR-293](../../docs/specification/adr/adr-293.md)
        // D5), which is not a claim about the lambda's type but a fact about a
        // block this checker has just walked - the same one `spawn` reads for
        // a task. ADR-288 D13 was about *writing* a lambda's result down, which
        // nothing does; a `map` still has to know what its elements are.
        //
        // **And whether it paused** ([ADR-233](../../docs/specification/adr/adr-233.md)
        // D1): a `std` entry that takes the lambda is lowered to its pausing
        // counterpart where it did.
        (tail, paused)
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

    /// `NK2205`: a `set` whose argument reads the same container with `get`
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D10,
    /// [ADR-281](../../../docs/specification/adr/adr-281.md)).
    ///
    /// `kasse.set(kasse.get() + 100)` takes the lock **twice** — once to read
    /// and once to store — and between the two the value can change, so what is
    /// stored is computed from a state that may no longer hold. Making a new
    /// value out of the old one is what the third door is for, and it takes the
    /// lock once.
    ///
    /// **Syntactic, on purpose** (D10's own line): it catches the `get`
    /// written *inside* the `set`, which is the shape people write, and not the
    /// same pair spread over two lines. The second is a question about what
    /// happened between two statements, and nothing here answers that — so
    /// widening this rule would mean guessing, and the narrow one is right
    /// about what it does see.
    /// **The stamp passes through a call the callee cannot put it back with**
    /// ([ADR-281](../../docs/specification/adr/adr-281.md) D23).
    ///
    /// A call whose argument is stamped is allowed where the callee's `touches`
    /// column names no lock — it cannot write the value into one — and **its
    /// result is stamped**. That is what makes `kasse.set(bumped(stand))` the
    /// same answer as `kasse.set(stand + 1)` without an analysis following the
    /// value through `bumped`.
    ///
    /// **A callee that touches a lock, or that nothing describes, does not
    /// pass one on.** An absent `touches` reads as *touches everything*
    /// ([ADR-292](../../docs/specification/adr/adr-292.md)), so a call nobody
    /// wrote down hands back a plain value rather than a stamped one: the
    /// wrong answer that way costs a refusal that is not raised, and the wrong
    /// answer the other way refuses a program (C.4).
    fn stamped_through(&self, contract: &FnContract, given: &[Ty], result: Ty) -> Ty {
        self.s.stamped_through(contract, given, result)
    }

    /// **`NK2207`: an `update` block that assigns to its `mut v` without
    /// reading it** ([ADR-281](../../docs/specification/adr/adr-281.md) D25).
    ///
    /// `update fn(mut v) { v = n }` is a `set` through the back door and is
    /// refused as one: nothing about it decides *inside* the lock, which is
    /// what the door is for. A block that **reads** `v` — `v += n`,
    /// `v.push(e)`, `if v > 100 { v = 0 }` — is the door working.
    ///
    /// **Read means mentioned anywhere but as the whole target**, which is the
    /// safe direction for a refusal: `v += n` mentions it, `v.f = n` mentions
    /// it, and only `v = …` does not. Asking a narrower question would refuse a
    /// block that reads `v` in a way this walk did not recognise, and that is
    /// [Part III C.4](../../docs/specification/30-nikaia-tooling.md)'s correct
    /// program refused.
    fn an_update_block_reads_what_it_writes(
        &mut self,
        params: &[winnow_grammar::Symbol],
        body: &Block,
        span: &Span,
    ) {
        self.s
            .an_update_block_reads_what_it_writes(self.world, params, body, span);
    }

    /// `NK2204`: an assignment straight into a `SharedMut`
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D10,
    /// [ADR-281](../../../docs/specification/adr/adr-281.md)).
    ///
    /// `kasse = 42` looks like an ordinary assignment and is not: the lock has
    /// to be taken for the write, and what takes it is a **door**. Without this
    /// the program meets `rustc` about a type it never wrote — the hull, not
    /// the value inside it — which is
    /// [Part III C.1](../../../docs/specification/30-nikaia-tooling.md).
    ///
    /// **The message names the door with the value in it**, because the repair
    /// is mechanical and a help line that says *"use a door"* leaves the reader
    /// to work out which of the four.
    fn a_write_that_skips_the_door(&mut self, target: &Expr, into: &Ty, value: &Expr, span: &Span) {
        let Ty::Named { name, .. } = into else {
            return;
        };
        if name != SHARED_MUT {
            return;
        }
        // The name, where the target is one. A field or an index into a hull is
        // a shape nothing decides yet, and saying nothing is the direction that
        // cannot refuse a program that is right.
        let Expr::Variable(bound, _) = target else {
            return;
        };
        let bound = self.parsed.text(*bound).to_string();
        let written = self.written(value);
        self.s.findings.push(Finding {
            severity: Severity::Error,
            span: *span,
            code: "NK2204".to_string(),
            message: format!(
                "You can't assign to `{bound}` directly: it holds shared mutable state."
            ),
            notes: vec![
                "Changing shared state goes through a method that takes the lock.".to_string(),
            ],
            help: Some(format!("Write `{bound}.set({written})`.")),
            labels: Vec::new(),
        });
    }

    /// The value as the author wrote it, where this compiler can say so.
    ///
    /// **An expression has no span** — [ADR-081](../../../docs/specification/adr/adr-081.md)
    /// D2 gave one to `Binary` and to nothing else — so there is no source text
    /// to quote back. What can be rebuilt is rebuilt, and the rest is an
    /// ellipsis rather than a guess: a help line that quoted the wrong thing
    /// would be worse than one that quotes nothing, and the shape the
    /// specification's own example writes (`kasse = 42`) is a literal.
    fn written(&self, value: &Expr) -> String {
        written_by(self.parsed, value)
    }

    /// **A `let` that binds several names at once**
    /// ([ADR-291](../../../docs/specification/adr/adr-291.md)).
    ///
    /// Part I 8.1.2 writes `let (user, rights, prefs) = overlap { … }` and Part
    /// II 12.5 writes `let (tx, rx) = channel::bounded(100)`. Both take a value
    /// apart by **position**, and neither writes a type - so this binds each
    /// name to the part at its index where the value's type is a tuple of the
    /// right width, and to `?` where it is not.
    ///
    /// **The widths are not compared**, and that is deliberate rather than
    /// unfinished. A value whose type this compiler cannot see is `?`, and
    /// refusing a destructure of one because it *looks* like the wrong width
    /// would refuse a correct program
    /// ([Part III C.4](../../../docs/specification/30-nikaia-tooling.md)). What
    /// a wrong width costs today is the language below saying so about a `let`
    /// the author did write, which is a smaller and honest failure - and the
    /// day the tuple's own width is known for certain is the day to refuse it
    /// here.
    ///
    /// **A written type is refused**, because `let (a, b): T = …` would have to
    /// say which name `T` is about and nothing decides that. Refused rather than
    /// ignored: ignoring it takes something the author wrote and drops it.
    fn tuple_let(
        &mut self,
        names: &[Ident],
        ty: Option<&crate::ast::Type>,
        found: &Ty,
        value: &Expr,
        span: &Span,
    ) -> Ty {
        self.s
            .tuple_let(self.world, names, ty.is_some(), found, value, span)
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

    /// **`comptime(steps: 50G, ram: 16Gi)`**
    /// ([ADR-321](../../docs/specification/adr/adr-321.md) D11): the steps and
    /// bytes it names, or `None` where one is refused - a name other than the
    /// two (`NK1109`), or a value that is not a whole number the build knows.
    fn raised_bounds(
        &mut self,
        bounds: &[crate::ast::ConfigArg],
        span: &Span,
    ) -> Option<(Option<u64>, Option<u64>)> {
        let raised = self.s.raised_bounds(self.world, bounds, span)?;
        Some((raised.steps, raised.ram))
    }

    /// **`dsl G { … } eod` over a grammar of this program** (#518): the
    /// grammar's `entry` run on the block's text while the program is built,
    /// as `comptime … = G::entry("…")` would be - the value is recorded for
    /// the emitter under the statement, as a `comptime`'s is.
    ///
    /// **Only a grammar with one entry**: no rule is a grammar's main one
    /// (ADR-296 D25), and which entry a block over a grammar with several
    /// runs is not decided (#518), so that one is left as it was.
    fn a_block_over_a_grammar(
        &mut self,
        block: &Expr,
        target: Ident,
        content: &str,
        span: &Span,
    ) -> Ty {
        let grammar = self.parsed.text(target).to_string();
        let Some(entry) = self
            .s
            .grammars
            .get(&grammar)
            .and_then(|entries| entries.iter().next().cloned())
        else {
            return Ty::Unknown;
        };
        let name = self.parsed.interner.intern_string("__nikaia_dsl");
        let call = Expr::Call {
            func: Box::new(Expr::Path(
                vec![target, self.parsed.interner.intern_string(&entry)],
                crate::ast::fresh_id(),
            )),
            args: vec![Expr::LitStr {
                text: build_time::written(content.trim()),
                id: crate::ast::fresh_id(),
            }],
            config: Vec::new(),
            id: crate::ast::fresh_id(),
        };
        // Recorded under the statement, as a `comptime`'s is, and moved to the
        // block's own node: two blocks in one statement are two values.
        let outer = self.s.facts.comptime_values.remove(&(span.at() as i64));
        self.comptime_binding(name, None, &call, &[], span);
        if let Some(value) = self.s.facts.comptime_values.remove(&(span.at() as i64)) {
            self.s.facts.dsl_values.insert(value_node(block), value);
        }
        if let Some(outer) = outer {
            self.s.facts.comptime_values.insert(span.at() as i64, outer);
        }
        // The block's type is the entry's result, as the call's would be.
        self.world
            .own
            .functions
            .get(&format!("{grammar}::{entry}"))
            .and_then(|contract| contract.signature.as_ref())
            .and_then(|signature| signature.result.clone())
            .unwrap_or(Ty::Unknown)
    }

    /// **One `comptime`, wherever it stands**
    /// ([ADR-287](../../../docs/specification/adr/adr-287.md)).
    ///
    /// [ADR-287](../../docs/specification/adr/adr-287.md) D3 decided both
    /// places and said the difference is *"where the name is visible, never
    /// what may stand to the right of the `='"*. So there is one function and
    /// the two callers differ only in which frame the name lands in - a second
    /// copy of the fold, the refusal and the emitter's spelling would be three
    /// chances for the two places to disagree about what a constant is.
    fn comptime_binding(
        &mut self,
        name: Ident,
        ty: Option<&crate::ast::Type>,
        value: &Expr,
        bounds: &[crate::ast::ConfigArg],
        span: &Span,
    ) -> Ty {
        // **The bounds this one raises** (ADR-321 D11), read before anything
        // runs: a bound that is refused runs nothing.
        let raised = self.raised_bounds(bounds, span);
        // **`asset("…")` is a name only here** (ADR-310 D3), so the ordinary
        // walk's refusal is off while the initialiser is read: the evaluator
        // answers it, and two walks over one expression must not both have an
        // opinion about the same call.
        let outside = std::mem::replace(&mut self.s.flow.inside_a_comptime, true);
        let walked_from = self.s.findings.len();
        let found = self.expr(value, span);
        self.s.flow.inside_a_comptime = outside;
        // **A name nothing declares is the one error** (ADR-287 D19, #380):
        // `comptime D = C + 1` above `comptime C = 1` in a body is `NK1117`,
        // because a body is read top to bottom. `NK1127` beside it - *can't be
        // computed while the program is built* - named the wrong cause.
        // A **name**, and not a module without its `use`: `io::read_to_string()`
        // is `NK1117` for the missing line and still `NK1127`, because a call
        // into `std` cannot be computed while the program is built either way.
        let said_a_name = self.s.findings[walked_from..]
            .iter()
            .any(|f| f.code == "NK1117" && f.message.ends_with("` isn't declared anywhere."));
        let bound = self.parsed.text(name).to_string();
        self.s.nameable(&bound, span, "a `comptime`");
        let want = ty.as_ref().map(|ty| self.declared(ty, span));

        // What the emitter writes, spelled in the language below. An
        // integer takes the type its declaration pinned, and otherwise
        // the first one that holds it - Part I 2.4's rule, applied here
        // because Rust's `const` will not take the absence.
        //
        // **Before the type check and not after it**, which is
        // [ADR-311](../../docs/specification/adr/adr-311.md) D1's doing: whether
        // a growable value crosses as a fixed one is a question about the
        // **value**, because the length that makes an `Array[T, N]` a type is
        // the one the build computed.
        let folded = self.constant_of(value);
        // **The second stage of
        // [ADR-287](../../docs/specification/adr/adr-287.md) D6**: a call, and
        // with it everything a called body can reach. The fold above is the
        // first stage and stays in front of it, because it is what says which
        // integer type a *declaration* pinned — a question the interpreter does
        // not ask and does not need to.
        let (evaluated, said) = match &folded {
            // A fold past the 65 bits has been refused already, and has no
            // number to hand on.
            Some(folded) if folded.beyond => (None, false),
            Some(folded) => (Some(build_time::Value::Int(folded.value)), false),
            None => match raised {
                None => (None, true),
                Some(raised) => {
                    let outside = std::mem::replace(&mut self.s.flow.raised, raised);
                    let outcome = match self.compiled_build_time_value(value, &found, &bound, span)
                    {
                        Some(outcome) => outcome,
                        None => self.build_time_value(value, &bound, span),
                    };
                    self.s.flow.raised = outside;
                    outcome
                }
            },
        };
        // Counted rather than returned, so that every refusal below - the
        // crossing's, the ordinary mismatch's, and whatever `constant_fits`
        // makes of a literal - suppresses `NK1127` the same way. One mistake,
        // one error, and the rule does not have to be remembered at each site.
        let before = self.s.findings.len();
        if let Some(want) = &want {
            self.constant_fits(value, Some(want), span);
            // **The annotation is a use, and a use answers the literal**
            // (ADR-152 D4) - the same line a `let` runs one construct over, and
            // it was missing here: `comptime PRIMES: Array[i64, 4] = [2, 3, 5, 7]`
            // is a `Vec[?]` against an `Array[i64, 4]` without it.
            let narrowed = self
                .array_literal(&found, want, value, span)
                .unwrap_or_else(|| found.clone());
            // **Growable going in, fixed coming out**
            // ([ADR-311](../../docs/specification/adr/adr-311.md) D1). A body
            // that builds its table with `push` has a `Vec`, and a `const`
            // below cannot hold one - but the value has been computed by the
            // line above, so its length is known and `Array[T, N]` is exactly
            // what it crosses as.
            match self
                .s
                .crosses_as_fixed(&narrowed, want, evaluated.as_ref(), span)
            {
                Crossing::Fits | Crossing::Said | Crossing::Unanswered => {}
                Crossing::Other => {
                    // …and D2's other half: a value that owns memory is refused
                    // for **what it is**, with the view-shaped equivalent as the
                    // way out. Only where the build computed one, because that
                    // is what lets the way out name the length.
                    match (&evaluated, is_growable(want)) {
                        // **A `struct` is only as writable as its fields**, and
                        // the one that is not is worth naming: a reader looking
                        // at `Bag { items: [1, 2, 3] }` cannot see which half
                        // of it a `const` cannot hold.
                        (Some(build_time::Value::Struct { .. }), _)
                            if self.unwritable_field(want).is_some() =>
                        {
                            let (field, held) = self.unwritable_field(want).expect("just asked");
                            self.s.a_field_that_owns_memory(
                                &bound,
                                &field,
                                &held,
                                "a `comptime`",
                                span,
                            );
                        }
                        (
                            Some(
                                computed
                                @ (build_time::Value::List(_) | build_time::Value::Text(_)),
                            ),
                            true,
                        ) => {
                            let (want, computed) = (want.clone(), computed.clone());
                            self.s.a_constant_that_owns_memory(
                                &bound,
                                &want,
                                &computed,
                                "a `comptime`",
                                span,
                            );
                        }
                        // **An `Array` with no length** is what a `comptime`
                        // list was written as, and the mismatch is the length:
                        // the help handed back the type that was written, which
                        // told nobody anything.
                        (Some(build_time::Value::List(items)), false)
                            if matches!(want, Ty::Named { name, args, .. }
                                if name == "Array" && args.len() == 1) =>
                        {
                            let element = match want {
                                Ty::Named { args, .. } => args[0].text(),
                                _ => unreachable!("matched above"),
                            };
                            self.s.findings.push(Finding {
                                severity: Severity::Error,
                                span: *span,
                                code: "NK1166".to_string(),
                                message: format!(
                                    "`{bound}` is an `Array[{element}]` with no length, which a \
                                     `comptime` needs."
                                ),
                                notes: vec![
                                    "An `Array` built into the program carries its length in \
                                     its type."
                                        .to_string(),
                                ],
                                help: Some(format!(
                                    "Declare it as `Array[{element}, {}]`: the build computed {}.",
                                    items.len(),
                                    plural(items.len(), "element")
                                )),
                                labels: Vec::new(),
                            });
                        }
                        _ => {
                            self.expect(&narrowed, want, *span, "const", |found, want| {
                                format!(
                                    "This value is `{found}`, but the `comptime` declares `{want}`."
                                )
                            });
                        }
                    }
                }
            }
        } else {
            self.constant_fits(value, None, span);
        }
        let said_a_type = self.s.findings.len() > before;

        let below = match (&want, &folded, value) {
            // **A type this program declares is its own name below**, and the
            // *value* is what says it is one: `rust_constant_type` knows the
            // types Part I 2.2 offers and cannot know a `Point`, where a
            // `Value::Struct` names the very type the declaration did.
            (Some(want), _, _) => rust_constant_type(self.world.library, want)
                .or_else(|| self.s.declared_below(self.world, want))
                .or_else(|| match &evaluated {
                    Some(build_time::Value::Struct { name, .. })
                        if matches!(want, Ty::Named { name: wanted, args, view: false }
                        if wanted == name && args.is_empty()) =>
                    {
                        Some(name.clone())
                    }
                    _ => None,
                }),
            (None, Some(folded), _) => Some(match &folded.pinned {
                Some(pinned) => pinned.clone(),
                None => folded.first_type().to_string(),
            }),
            (None, None, _) => match &evaluated {
                // A float with nothing declaring which one it is takes the
                // wider, as an integer does (Part I 2.4).
                Some(build_time::Value::Float(_)) => Some("f64".to_string()),
                // A `bool` from the interpreter is a `bool` below, whether it
                // was written `true` or came out of a call.
                Some(build_time::Value::Bool(_)) => Some("bool".to_string()),
                // **The expression's type, never the value's**
                // ([ADR-287](../../docs/specification/adr/adr-287.md) D21): a
                // call's result type, so `big(1)` and `big(3)` are one type
                // and no type waits on an evaluation.
                Some(build_time::Value::Int(value)) => Some(
                    integer_named(&found)
                        .unwrap_or_else(|| Constant::of(*value).first_type().to_string()),
                ),
                // **An array with nothing declaring its type.** The element
                // type is the **checker's** where it has one - a body declared
                // `-> Vec[i64]` says `i64`, and reading it off the values
                // instead would make `[0, 1, 4]` an `[i32; 3]` and every later
                // `i64` arithmetic on it `rustc`'s complaint about a file
                // nobody wrote. Part I 2.4's widest-holder rule is the fallback
                // for the case nothing declared anything.
                Some(build_time::Value::List(items)) => element_below(self.world.library, &found)
                    .or_else(|| rust_array_type(items))
                    .map(|ty| format!("[{ty}; {}]", items.len())),
                // **Text crosses as a view, and `&str` is the one a `const`
                // holds** ([ADR-311](../../docs/specification/adr/adr-311.md)
                // D1). A `String` allocates and `const X: String` is not
                // something the language below has; `const X: &str` is, and its
                // lifetime is `'static` by the elision a `const` already makes.
                Some(build_time::Value::Text(_)) => Some("&str".to_string()),
                // A pair with nothing declaring what it is for says nothing:
                // `Fixed[K, V]` is the declaration that makes a list of them a
                // table, and without it there is no type to write.
                Some(build_time::Value::Tuple(_)) => None,
                // **A struct is its own name below**, and the fields carry
                // themselves — `const P: Point = Point { x: 1, y: 2 };` is
                // Rust, and a struct whose fields own nothing is already the
                // view form ([ADR-311](../../docs/specification/adr/adr-311.md)
                // D1's *a number is already its own view*, one shape out).
                Some(build_time::Value::Struct { name, .. }) => Some(name.clone()),
                // …and a variant is its `enum`'s name, one shape over.
                Some(build_time::Value::Variant { ty, .. }) => Some(ty.clone()),
                Some(build_time::Value::Constant { constructor, .. }) => constructed(constructor),
                None => None,
            },
        };

        // **A map the build can see** ([ADR-311](../../docs/specification/adr/adr-311.md)
        // D1): a list of pairs, and the **declared type** is what says it is a
        // table rather than a list — the rule
        // [ADR-152](../../docs/specification/adr/adr-152.md) D4 already makes
        // for `Array[T, N]`, one shape out. Both halves at once, because the
        // type below and the value below are one answer here: `Fixed<i64>` and
        // a `Fixed::new(…)` are written from the same table.
        let (crossed, said_a_table) = match (&want, &evaluated) {
            (Some(want), Some(build_time::Value::List(pairs))) if is_fixed(want) => {
                self.a_table_that_crosses(&bound, want, pairs, span)
            }
            _ => (None, false),
        };

        // **The value, spelled below, read against the declaration**
        // ([ADR-179](../../docs/specification/adr/adr-179.md) D2). Every shape
        // but one spells itself — an integer is its digits wherever it stands
        // — and a **list** is the one that does not: the same value is
        // `[1, 2, 3]` against an `Array[i64, 3]` and `&[1, 2, 3]` against a
        // `&[i64]`, so the type is walked beside it.
        //
        // A pair has no `const` form of its own, and a list of them has one
        // only where a declaration says it is a table — which `crossed` below
        // is.
        let written = match &evaluated {
            Some(value) => self.written_below(value, want.as_ref()),
            None => None,
        };
        let (below, written) = match crossed {
            Some((below, written)) => (Some(below), Some(written)),
            None => (below, written),
        };
        // **A part of the value the language below cannot write.**
        //
        // Asked of the **value** rather than of the declared type, because an
        // `enum` is not answerable from its type: `Shape::Empty` is a `const`
        // and `Shape::Many([1, 2])` is not, and the two are the same `Shape`.
        // Without this the second one lowered — `const M: Shape =
        // Shape::Many([1, 2]);` against a variant that declares a `Vec` — and
        // `rustc` answered about the generated file, which is [Part III
        // C.1](../../docs/specification/30-nikaia-tooling.md)'s class.
        let said_a_part = match &evaluated {
            Some(value) => match self.unwritable_in(value) {
                Some((where_it_is, held)) => {
                    self.s.a_field_that_owns_memory(
                        &bound,
                        &where_it_is,
                        &held,
                        "a `comptime`",
                        span,
                    );
                    true
                }
                None => false,
            },
            None => false,
        };
        match (&below, &written) {
            // **Nothing is recorded where a part of it cannot be written**,
            // or the emitter would write the `const` the refusal just said is
            // not one.
            (Some(below), Some(written)) if !said_a_part => {
                self.s
                    .facts
                    .comptime_values
                    .insert(span.at() as i64, (below.clone(), written.clone()));
            }
            // …and nothing at all where the refusal has already been made by
            // name: `NK1152` and `NK1165` each say what `NK1127` would, with
            // the part that matters in it.
            _ if said || said_a_type || said_a_table || said_a_part || said_a_name => {}
            _ => self.s.findings.push(Finding {
                code: "NK1127".to_string(),
                severity: Severity::Error,
                span: *span,
                message: format!("`{bound}` can't be computed while the program is built."),
                notes: vec![
                    "A `comptime` has to be computed while the program is built.".to_string(),
                    "At build time the compiler can compute numbers, `bool`s, text, lists, \
                         structs and enum variants: literals, `f\"…\"`, arithmetic and \
                         comparisons, `if`, `for` over a range, `while`, and calls to your own \
                         functions and methods made of those."
                        .to_string(),
                ],
                help: Some(format!(
                    "Write `let {bound} = …` to compute it while the program runs instead."
                )),
                labels: Vec::new(),
            }),
        }

        // **An integer with no written type is an open number**
        // ([ADR-287](../../docs/specification/adr/adr-287.md) D21): each use
        // takes it in the type it asks for, checked against the value, as a
        // literal is taken.
        let open = want.is_none() && matches!(evaluated, Some(build_time::Value::Int(_)));
        let held = match open {
            true => Ty::Unknown,
            false => want.unwrap_or(found),
        };
        let id = self.a_new_binding();
        if open {
            self.s.open_comptimes.insert(id, *span);
        }
        // **What the name is worth is what was *evaluated*, not what folded.**
        // They were the same thing while the fold was the whole evaluator; with
        // a call in it they are not, and binding the fold left
        // `comptime ANSWER = double(21)` visible as a name with no value — so
        // the next constant that read it was `NK1127` although the one before
        // it had just been computed.
        self.bind_local(Local {
            written_mut: false,
            id,
            literal: None,
            name: bound,
            ty: held,
            lent: false,
            changing: false,
            // The fold's number, which is what `constant_of` reads one
            // `comptime` later and what `ADR-285 D29's overflow check needs.
            constant: match &evaluated {
                Some(build_time::Value::Int(value)) => Some(*value),
                _ => None,
            },
            // …and the whole value, which is what a *call* one `comptime`
            // later needs: `ORIGIN.scaled(10)` wants a `Point`.
            built: evaluated,
            immutable: None,
            empty_list: None,
            open_number: None,
        });
        Ty::Tuple(Vec::new())
    }

    /// **What a `return` hands back, asked once**
    /// ([ADR-276](../../docs/specification/adr/adr-276.md) D21): the statement
    /// and the expression are the same `return`, so they ask the same
    /// questions in the same order rather than in two places that drift.
    fn returns(&mut self, value: Option<&Expr>, span: &Span, in_an_expression: bool) {
        // **The `&` at a `return`** ([ADR-094](../../docs/specification/adr/adr-094.md)
        // D1's third position), asked before the refusal it takes the place of:
        // a place inside a borrowed subject, handed back where the function
        // declares a view, is a view *of* the subject and not a move out of it.
        let lending =
            value.is_some_and(|value| self.s.hands_back_a_view_of_the_subject(self.world, value));
        if lending {
            self.s.facts.lent_returns.insert(span.at() as i64);
        }
        // **And a `ref` written there is the compiler's line said twice**
        // (#96, ADR-094 D4): `return ref row.name` where `return row.name`
        // already lends.
        if let Some(
            written @ Expr::Unary {
                op: crate::ast::UnaryOp::Ref,
                expr: inner,
                ..
            },
        ) = value
            && self.s.hands_back_a_view_of_the_subject(self.world, inner)
        {
            self.s.the_caller_writes_no_reference(
                written,
                span,
                "a place inside what this function borrows is handed back as a view of it",
            );
        }
        if let Some(value) = value
            && !lending
        {
            self.a_field_of_a_borrowed_subject(value, span, "handed back");
            self.a_field_of_a_lent_place_handed_back(value, span);
            self.a_mut_parameter_given_away(value, span, "handed back");
        }
        if let Some(Expr::Closure { .. }) = value {
            self.s.flow.lambda_expected = self.s.expected.clone();
        }
        let found = match value {
            Some(value) => self.expr(value, span),
            None => Ty::Tuple(Vec::new()),
        };
        self.s.flow.lambda_expected = None;
        let Some(expected) = self.s.expected.clone() else {
            return;
        };
        // The line hands back the **view**, so that is what answers to the
        // declared type. `view_of` and not a `&` pasted on the front, for the
        // reason the argument position has it: a view of a `String` is a `ref
        // String` and the one rewrite Part I 6.5 makes.
        let found = match lending {
            true => view_of(&found),
            false => found,
        };
        let mut found = found;
        if let Some(value) = value {
            self.constant_fits(value, Some(&expected), span);
            match in_an_expression {
                false => self.wraps_into_nullable(&found, &expected, value, span),
                true => {
                    if let Some(how) = self.widened_slot(&found, &expected, Some(value), span) {
                        self.s.facts.wrapped.insert(value_node(value), how);
                    }
                }
            }
            // **The declared result is a use** (ADR-152 D4).
            found = self
                .literal_by_use(&found, &expected, value, span)
                .unwrap_or(found);
        }
        self.expect_kept(
            &found,
            &expected,
            value,
            "the function returns it, and the caller keeps it after the call ends",
            *span,
            "returns",
            |found, want| {
                format!(
                    "You're returning `{found}`, but the function is declared to return `{want}`."
                )
            },
        );
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

    fn method(&self, key: &str) -> Option<(String, FnContract)> {
        resolve_method(self.world, key).map(|found| (found.key, found.contract))
    }

    /// The fields of a type, when something knows them.
    /// What a value's own type arguments bind its declaration's parameters to.
    ///
    /// `Pair[i64]` against `struct Pair[T]` gives `T = i64`, positionally,
    /// because a type argument's position is the only thing that says which
    /// parameter it fills. Empty for every type that declares none, which is
    /// every type written today.
    fn arguments_of(&self, on: &Ty) -> BTreeMap<String, Ty> {
        self.s.arguments_of(on)
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
        let lent: BTreeSet<String> = parts
            .iter()
            .filter(|(name, ty)| {
                let taken = self.s.handed[from as usize..].iter().any(|taken| {
                    taken.path == **name || taken.path.starts_with(&format!("{name}."))
                }) || self.s.changed[changed_from..].iter().any(|n| n == *name);
                self.s.takes_away(self.world, ty) && !taken && !gives_back(body, name, self.parsed)
            })
            .map(|(name, _)| name.clone())
            .collect();
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

/// What one turn of a `for` binds, when the collection's element type is
/// written down.
///
/// Only a list with one element type and one binding: `for (k, v) in map`
/// takes apart a pair whose shape Stage 0 has no signature for, and a view of
/// a collection yields views whose spelling the language below chooses.
/// Whether a method takes its receiver **by value**, which for a produced
/// sequence is what consumes it ([ADR-105](../../docs/specification/adr/adr-105.md) D2).
///
/// Read off the signature rather than off a list of names: every `Seq` entry
/// writes `(Seq[$T], …)` and a container's writes `(&Vec[$T], …)`, so the file
/// that describes the method is what says whether the walk keeps it.
/// **What a sequence entry's result is as a whole**
/// ([ADR-293](../../docs/specification/adr/adr-293.md) D17): a word the entry
/// writes on its result, where every sequence handed in has it too.
///
/// A `map` walked from the back is its input walked from the back, so `map`
/// writes `ends sized` and a `filter` over `io::lines()` still cannot be. Where
/// the entry says `ends_by_length`, walking the result from the back also needs
/// every input's length - `take`, `zip`, `skip`, `step_by`.
///
/// **A result never replays** unless the entry says so: a range replays, and a
/// `rev` of one is an ordinary sequence, walked once.
fn shape_through(contract: &FnContract, receiver: &Ty, found: &[Ty], result: Ty) -> Ty {
    nikaia_std::tools::check_calls::shape_through(contract, receiver, found, result)
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

fn walks_by_value(contract: &FnContract) -> bool {
    nikaia_std::tools::check_calls::walks_by_value(contract)
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
    }
}

/// What each of a `for`'s bindings is: [`element_of`] for one, and for
/// several the parts the element is taken apart into - a map's key and value,
/// or a tuple's parts, position by position. Where the element cannot be taken
/// apart into that many, every binding is unknown, as before.
fn elements_of(over: &Ty, bindings: usize) -> Vec<Ty> {
    nikaia_std::tools::check_types::elements_walked(over, bindings as i64)
}

/// A number, a `bool` or a `char`: read through a view, the value is the same
/// value ([`Checked::copied_loop_bindings`]).
fn a_copy_by_value(ty: &Ty) -> bool {
    nikaia_std::tools::check_types::a_copy_by_value(ty)
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

/// What a `&x` was a view **of**, as far as [`view_of`] runs backwards.
///
/// It does not run backwards cleanly — a `&str` is a view of a `String` and
/// also a `&str` standing on its own — so this answers the owned types that
/// would have produced the view, and the caller asks the fit of each.
///
/// The one place that needs it is [`Checker::the_compiler_writes_the_reference`],
/// deciding whether a `&` the source wrote is the one the compiler would have
/// written anyway. There, a list that is too short costs a refusal that is not
/// raised and a list that is too long costs a correct program refused, so it is
/// the short one: only the views this compiler itself makes.
fn viewed_from(view: &Ty) -> Vec<Ty> {
    nikaia_std::tools::check_types::viewed_from_type(view)
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

/// The three types whose hull a program writes by calling their name
/// ([ADR-281](../../../docs/specification/adr/adr-281.md) D2).
///
/// **A hull you cannot observe, the compiler writes; a hull you can observe, you
/// write.** A `T?` costs nothing and hides nothing - the same value, possibly
/// absent - so its `Some(…)` is written for you (ADR-278). These three change
/// **when a value is cleaned up**, which Part I 6.2 says a program can see, so the
/// word stands where it happens.
fn is_hull(name: &str) -> bool {
    HULLS.contains(&name)
}

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

    fn shape(self) -> &'static str {
        match self {
            MultiLock::Reading => "access_all(a, b) fn(x, y) { … }",
            MultiLock::Writing => "update_all(a, b) fn(x, y) { … }",
        }
    }
}

/// What a lock holds, where the type says it is one.
///
/// `SharedMut[T]` is a count around a lock and `Locked[T]` is the lock on its
/// own; both hold a `T` and a door over several takes either.
fn locked_content_of(ty: &Ty) -> Option<Ty> {
    nikaia_std::tools::check_types::locked_content_of(ty)
}

/// Whether a plain value stands where a **hull** is wanted - the shape whose
/// message names the constructor.
///
/// `want` is `Shared[T]`, `SharedMut[T]` or `Locked[T]` and `found` is the `T` it
/// would hold. It used to *permit* that at two positions, because the annotation
/// was the constructor ([ADR-312](../../../docs/specification/adr/adr-312.md)).
/// It permits nothing now
/// ([ADR-281](../../../docs/specification/adr/adr-281.md) D2): a hull is made by a
/// call, wherever a call may stand, and what this answers is only *which sentence
/// to write* when one is missing.
///
/// That is the whole of why the positions stopped being a list. A literal could
/// not be handled at all while the annotation was the constructor - it has no
/// type of its own, so nothing knew the hull was wanted - and `SharedMut(0)`
/// needs nobody to know.
fn becomes_shared(found: &Ty, want: &Ty) -> bool {
    nikaia_std::tools::check_types::becomes_shared(found, want)
}

/// Part III C.2: every diagnostic names a concrete way out.
fn convert(found: &Ty, want: &Ty) -> String {
    nikaia_std::tools::check_types::way_out_of_a_mismatch(found, want)
}

/// **Whether a `'…'` as written is one ASCII character**, which is what makes
/// it a byte as well as a scalar (Part I 2.6; `tools/check_texts.nika`).
pub fn a_byte(written: &str) -> bool {
    check_texts::literal_is_a_byte(written)
}

/// The byte a `'…'` that [`a_byte`] took is, as the language below writes one.
pub fn byte_literal(written: &str) -> String {
    let byte = crate::build_time::decoded(written)
        .and_then(|text| text.bytes().next())
        .unwrap_or(0);
    match byte {
        b'\\' | b'\'' => format!("b'\\{}'", byte as char),
        0x20..=0x7e => format!("b'{}'", byte as char),
        _ => format!("b'\\x{byte:02x}'"),
    }
}

/// **What a table about an expression is keyed on**: the id the parser gave the
/// node ([ADR-340](../../docs/specification/adr/adr-340.md) D4), which a clone
/// keeps and an address does not.
pub fn value_node(expr: &Expr) -> i64 {
    crate::ast::id_of(expr) as i64
}

/// **`an i32` and `a u64`**: the article as the name is said (`NK1116`'s rule).
/// A `u` is said *you*, so it takes `a`; the other vowels take `an`.
pub fn an_or_a(ty: &str) -> &'static str {
    match ty.starts_with(['i', 'a', 'e', 'o', 'I', 'A', 'E', 'O']) {
        true => "an",
        false => "a",
    }
}

/// **A handler's `error` where several error types arrive** (ADR-280 D30):
/// an `Error` with its members.
fn several_errors(ty: &Ty) -> Option<&[Ty]> {
    match ty {
        Ty::Named {
            name,
            args,
            view: false,
        } if name == "Error" && args.len() > 1 => Some(args.as_slice()),
        _ => None,
    }
}

/// The members, as a message names them: `` `ConfigError`, `io::IoError` ``.
fn members_named(members: &[Ty]) -> String {
    nikaia_std::tools::check_places::members_named(members)
}

/// The value a block ends in: its last statement, where that is an expression.
fn tail_of(block: &Block) -> Option<&Expr> {
    match block.stmts.last().map(|s| &s.node) {
        Some(Stmt::Expr(value)) => Some(value),
        _ => None,
    }
}

/// **Whether an arm hands a name back by value**: as its own value, or through
/// a `return`. Either needs the part itself and not a view of it.
fn gives_back(body: &Expr, name: &str, parsed: &Parsed) -> bool {
    fn is_name(expr: &Expr, name: &str, parsed: &Parsed) -> bool {
        matches!(expr, Expr::Variable(n, _) if parsed.text(*n) == name)
    }
    fn tail(expr: &Expr, name: &str, parsed: &Parsed) -> bool {
        match expr {
            Expr::Block(block, _) => block_tail(block, name, parsed),
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                block_tail(then_branch, name, parsed)
                    || else_branch
                        .as_ref()
                        .is_some_and(|b| block_tail(b, name, parsed))
            }
            Expr::Match { arms, .. } => arms.iter().any(|arm| tail(&arm.body, name, parsed)),
            other => is_name(other, name, parsed),
        }
    }
    fn block_tail(block: &Block, name: &str, parsed: &Parsed) -> bool {
        matches!(block.stmts.last().map(|s| &s.node), Some(Stmt::Expr(e)) if tail(e, name, parsed))
    }
    fn block_returns(block: &Block, name: &str, parsed: &Parsed) -> bool {
        block.stmts.iter().any(|stmt| match &stmt.node {
            Stmt::Return(Some(value)) => {
                is_name(value, name, parsed) || returns(value, name, parsed)
            }
            Stmt::For { iter, body, .. } => {
                returns(iter, name, parsed) || block_returns(body, name, parsed)
            }
            Stmt::While { cond, body } => {
                returns(cond, name, parsed) || block_returns(body, name, parsed)
            }
            Stmt::Let { value, .. } | Stmt::Comptime { value, .. } => returns(value, name, parsed),
            Stmt::Assign { value, .. } => returns(value, name, parsed),
            Stmt::Expr(value) => returns(value, name, parsed),
            _ => false,
        })
    }
    fn returns(expr: &Expr, name: &str, parsed: &Parsed) -> bool {
        let mut found = false;
        crate::emit::visit_expr(expr, &mut |inner| match inner {
            Expr::Return(value, _)
                if (**value)
                    .as_ref()
                    .is_some_and(|value| is_name(value, name, parsed)) =>
            {
                found = true
            }
            Expr::Block(block, _) | Expr::Unsafe(block, _) => {
                found |= block_returns(block, name, parsed);
            }
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                found |= block_returns(then_branch, name, parsed)
                    || else_branch
                        .as_ref()
                        .is_some_and(|b| block_returns(b, name, parsed));
            }
            _ => {}
        });
        found
    }
    tail(body, name, parsed) || returns(body, name, parsed)
}

impl Checker<'_> {
    /// A name that is a parameter the function only lends on
    /// (`tools/check_fields.nika`).
    fn a_lent_parameter(&self, given: &Expr) -> bool {
        self.s.a_lent_parameter(self.world, given)
    }
}

/// A view of a value that copies - a number, a truth value, a character - and
/// nothing else (ADR-231 D1).
fn copies_as_a_view(ty: &Ty) -> bool {
    nikaia_std::tools::check_types::copies_as_a_view(ty)
}

/// `ref String` as the language below has it, a `&String`: what a `$T` bound
/// to `String` becomes under `ref $T`, which is an element of `words.iter()`.
fn is_a_view_of_a_string(ty: &Ty) -> bool {
    nikaia_std::tools::check_types::is_a_view_of_a_string(ty)
}

/// A view of a value that copies read as the value, and anything else as it is
/// ([ADR-233](../../docs/specification/adr/adr-233.md) D4).
impl Checker<'_> {
    /// An empty map written to for the first time (`tools/check_fields.nika`).
    fn a_map_first_written(&mut self, target: &Expr, found: &Ty) {
        self.s.a_map_first_written(self.world, target, found);
    }
}

/// The map types whose key and value a first `insert` decides (#543).
/// By the last segment: the type is `collections::HashMap` where the `use`
/// named the module.
fn a_map(name: &str) -> bool {
    ["HashMap", "BTreeMap", "Map"].contains(&name.rsplit("::").next().unwrap_or(name))
}

fn value_of_a_copy(ty: Ty) -> Ty {
    nikaia_std::tools::check_types::value_of_a_copy(&ty)
}

/// The value a view is of.
fn unviewed(ty: &Ty) -> Ty {
    nikaia_std::tools::check_types::unviewed_type(ty)
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
    let mut rows: Vec<(usize, &Claim)> = checked
        .facts
        .claims
        .iter()
        .map(|((start, _), claim)| {
            let line = source[..(*start as usize).min(source.len())]
                .matches('\n')
                .count()
                + 1;
            (line, claim)
        })
        .collect();
    rows.sort_by_key(|(line, _)| *line);
    use crate::prove::Held;
    let count = |want: fn(&Held) -> bool| {
        rows.iter()
            .filter(|(_, c)| c.held.as_ref().is_some_and(want))
            .count()
    };
    let mut out = format!(
        "asserts in {path}: {} - proved {}, preconditions {}, checked at run time {}, \
         checked by a test {}, refused {}\n",
        rows.len(),
        count(|h| matches!(h, Held::Proved)),
        count(|h| matches!(h, Held::Precondition(..))),
        count(|h| matches!(h, Held::AtRunTime(_))),
        count(|h| matches!(h, Held::ByTheTest)),
        count(|h| matches!(h, Held::Refused)),
    );
    for (line, claim) in rows {
        let how = match &claim.held {
            Some(Held::Proved) => "proved".to_string(),
            Some(Held::Precondition(f, 0)) => format!(
                "precondition of `{f}`: proved at every call the compiler sees; any other \
                 reaches the entry that checks it"
            ),
            Some(Held::Precondition(f, 1)) => format!(
                "precondition of `{f}`: one call checks it when it runs; any other call \
                 proves it or reaches the entry that checks it"
            ),
            Some(Held::Precondition(f, n)) => format!(
                "precondition of `{f}`: {n} calls check it when they run; any other call \
                 proves it or reaches the entry that checks it"
            ),
            Some(Held::AtRunTime(why)) => format!("checked at run time: {why}"),
            Some(Held::ByTheTest) => "checked when the test runs".to_string(),
            Some(Held::Refused) | None => {
                "refused: a claim about data from outside the program".to_string()
            }
        };
        out.push_str(&format!("  {path}:{line}  {}  {how}\n", claim.written));
    }
    out
}

/// Whether an expression is a literal of a kind other than `kind` (`"bool"`
/// or `"text"`): the one answer a literal whose type is settled below gives
/// here.
fn is_a_literal_other_than(expr: &Expr, kind: &str) -> bool {
    match expr {
        Expr::LitBool(_, _) => kind != "bool",
        Expr::LitStr { .. } | Expr::LitInterpolated { .. } => kind != "text",
        Expr::LitInt { .. } | Expr::LitFloat(_, _) | Expr::LitChar(_, _) | Expr::LitNull(_) => true,
        _ => false,
    }
}

/// The prelude's claim ([ADR-269](../../docs/specification/adr/adr-269.md) D2).
pub const ASSERT: &str = "assert";

/// Why a call cannot stand in an `assert`'s condition, or nothing where it can
/// ([ADR-269](../../docs/specification/adr/adr-269.md) D3): it may pause, may
/// fail, touches something or changes a value. A `touches` nobody wrote is not
/// an answer, and counts as touching.
fn not_pure_because(contract: &FnContract) -> Option<String> {
    nikaia_std::tools::check_walks::check_not_pure_because(contract)
}

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
    let mut out = String::with_capacity(text.len() + 8);
    let mut rest = text;
    while let Some(at) = rest.find('`') {
        let (before, from) = rest.split_at(at);
        let fixed = ["a ", "an ", "A ", "An "].iter().find_map(|article| {
            let head = before.strip_suffix(article)?;
            let starts_a_word = head
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric() && c != '`' && c != '_');
            starts_a_word.then_some((head, *article))
        });
        match fixed {
            // **A word that starts with a sign keeps what was written**: `an
            // `|` pattern` is said *an or-pattern*, and no rule reads that.
            Some((head, article))
                if from[1..].chars().next().is_some_and(char::is_alphanumeric) =>
            {
                let word: String = from[1..].chars().take_while(|c| *c != '`').collect();
                let right = nikaia_std::tools::check_words::article(&word);
                let right = match article.starts_with('A') {
                    true => {
                        let mut capital = right[..1].to_uppercase();
                        capital.push_str(&right[1..]);
                        capital
                    }
                    false => right,
                };
                out.push_str(head);
                out.push_str(&right);
                out.push(' ');
            }
            _ => out.push_str(before),
        }
        // The backticked word itself, closing tick included, is copied as it is.
        let close = from[1..].find('`').map_or(from.len(), |end| end + 2);
        out.push_str(&from[..close]);
        rest = &from[close..];
    }
    out.push_str(rest);
    out
}

/// What a template's `<for>` binds around a hole: each name beside the
/// collection it walks (#524).
type LoopNames = Vec<(String, String)>;

/// **The two forms of a root**, where that is the argument a call left out
/// ([ADR-108](../../docs/specification/adr/adr-108.md) D1: *a call that leaves
/// it out is `NK1101`, and the message names the two forms of D2*).
///
/// A sentence about one `std` type, written here because the ledger has no
/// column for an `enum`'s variants: what `it takes `root: ref Root`` leaves a
/// reader with is a type name and a question. The two lines beside `PROMISED`
/// and `NOT_A_MODULE` in this file are there for the same reason — a thing the
/// compiler has to know and the ledger does not carry.
///
/// **Empty for every other missing argument**, which is what keeps this from
/// being a habit: the parameter has to be named `root` *and* typed `fs::Root`.
fn a_root_is_one_of_two(wanted: &[(String, Ty)]) -> String {
    nikaia_std::tools::check_places::a_root_is_one_of_two(wanted)
}

fn plural(n: usize, what: &str) -> String {
    nikaia_std::tools::check_words::counted(n as i64, what)
}

/// What the receiver's actual type binds this signature's variables to.
///
/// The receiver is the signature's first parameter where there is one, so this
/// is one `bind` against one pattern - the narrowness is ADR-288's decision
/// rather than a gap. A signature with no variables produces an empty map and
/// every substitution below is the identity.
fn bindings(contract: &FnContract, receiver: &Ty) -> BTreeMap<String, Ty> {
    nikaia_std::tools::check_calls::receiver_bindings(contract, receiver)
}

/// What the **arguments** bind this signature's variables to.
///
/// ADR-288 bound from the receiver only, and said so on purpose: binding from
/// arguments is where a signature language grows into a unification algorithm.
/// [ADR-295](../../../docs/specification/adr/adr-295.md) D2 takes that step and
/// keeps it one step. This is the same `ty::bind` against one pattern per
/// argument - a structural walk, no queue, no fixpoint, no occurs check -
/// because a type parameter is **written** rather than inferred, so there is
/// never a variable on the right-hand side to solve for.
///
/// A mismatch binds nothing, exactly as the receiver's does: the question is
/// what a caller can *tell* the signature, and `substitute` turns what it could
/// not tell into `?`.
fn from_arguments(contract: &FnContract, found: &[Ty]) -> BTreeMap<String, Ty> {
    nikaia_std::tools::check_calls::argument_bindings(contract, found)
}

/// The types a callee's parameters expect, as a call site sees them.
///
/// A method's receiver is the first parameter, so the arguments a *call* writes
/// are the ones after it - `Signature::arguments` already draws that line.
fn expected_arguments(contract: &FnContract) -> Vec<Ty> {
    nikaia_std::tools::check_calls::expected_arguments(contract)
}

/// Whether handing a value of this type out takes nothing away.
///
/// Part I 2.2's numbers, `bool` and `char`, and any **view**: all of them are
/// copied rather than moved in the language below, so a field of one may leave a
/// borrowed subject freely. Everything else - a `String`, a `Vec`, a struct this
/// program declares - is moved, which is what `NK1131` is about.
///
/// A short list on purpose, and the polarity is the usual one: a type not on it
/// **Whether two values of this type may be compared**
/// ([ADR-204](../../docs/specification/adr/adr-204.md) D1).
///
/// `==` on a declared type had no lowering at all: `rustc` answered *binary
/// operation `==` cannot be applied to type `P`* about a file nobody wrote, with
/// *consider annotating `P` with `#[derive(PartialEq)]`* as the help — [Part III
/// C.1 and C.2](../../docs/specification/30-nikaia-tooling.md) at once. So the
/// question has to be answerable here, and the answer decides both the derive
/// the emitter writes and the refusal this file raises.
///
/// **Three sources and one rule.** The language's own types answer from the list
/// below; a **declared** type answers from its parts, structurally, which is what
/// cannot be wrong; and a type whose parts are Rust answers from the ledger's
/// `compares` column, whose absence is *no* ([ADR-010](../../docs/specification/adr/adr-010.md)
/// D1).
///
/// **`Ty::Unknown` compares.** Nothing is known, so nothing is refused — [Part
/// III C.4](../../docs/specification/30-nikaia-tooling.md)'s direction, and the
/// same answer every other question here gives an absent claim.
struct Comparable<'a> {
    /// What each type another file of the package declares is made of.
    beside: &'a BTreeMap<String, Vec<Ty>>,
    structs: &'a BTreeMap<String, Vec<FieldContract>>,
    enums: &'a BTreeMap<String, BTreeSet<String>>,
    /// What a declared `enum`'s variants hold ([`Checker::enum_payloads`]).
    payloads: &'a BTreeMap<String, Vec<Ty>>,
    /// A generic struct's parameters in declaration order
    /// ([`Checker::struct_parameters`]), so `Spanned[Stmt]` is asked about the
    /// `Stmt` its `node` holds rather than about a `T` that says nothing.
    parameters: &'a BTreeMap<String, Vec<String>>,
    own: &'a Ledger,
    library: &'a Ledger,
}

/// Which of the two questions [`Comparable`] is being asked.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Strength {
    /// `==` works: Rust's `PartialEq`.
    Compares,
    /// …and equality is an equivalence: Rust's `Eq`, which a float is not and
    /// which a type holding one cannot be.
    Totally,
}

impl Comparable<'_> {
    /// The primitives and the containers, which are the language's rather than a
    /// module's and so are a list here rather than a column anywhere.
    ///
    /// A container compares exactly when what it holds does, which is Rust's own
    /// rule for each of them and is why they are one arm.
    fn of(&self, ty: &Ty) -> bool {
        self.asking(ty, Strength::Compares, &mut BTreeSet::new())
    }

    /// The same question about **`Eq`**, which is not the same walk one step
    /// deeper: a `struct` holding a type that compares and is not an equivalence
    /// is the same, and the float may be several declarations away.
    fn totally(&self, ty: &Ty) -> bool {
        self.asking(ty, Strength::Totally, &mut BTreeSet::new())
    }

    /// `seen` is the types already being asked about, so a `struct` that holds
    /// itself through a `Vec` answers once rather than forever.
    fn asking(&self, ty: &Ty, how: Strength, seen: &mut BTreeSet<String>) -> bool {
        match ty {
            // An absent claim is not a refusal (C.4). A type variable is one:
            // a library's `$V` is substituted away at a call, and one that is
            // not is `Unknown` by the same rule.
            Ty::Unknown | Ty::Var { .. } | Ty::Count(_) => true,
            Ty::Tuple(parts) => parts.iter().all(|part| self.asking(part, how, seen)),
            Ty::Nullable(inner) => self.asking(inner, how, seen),
            // **A produced sequence is not a value this language holds**: it is
            // walked, and what it hands over is the item.
            Ty::Seq { .. } => false,
            // **What the C boundary lends is an address**, and an address is not
            // a value this language compares
            // ([ADR-302](../../docs/specification/adr/adr-302.md) D5).
            Ty::Pointed { .. } => false,
            Ty::Fn { .. } => false,
            Ty::Named { name, args, .. } => self.named(name, args, how, seen),
        }
    }

    fn named(&self, name: &str, args: &[Ty], how: Strength, seen: &mut BTreeSet<String>) -> bool {
        // **A view compares as what it views does**: `&str` is `String`'s
        // comparison and `&T` is `T`'s, which is Rust's rule too.
        let holds =
            |seen: &mut BTreeSet<String>| args.iter().all(|arg| self.asking(arg, how, seen));
        match name {
            // **A float compares and is not an equivalence**, because
            // `NaN != NaN`. It is the one primitive the two questions differ on,
            // and every type holding one differs with it however far down.
            "f64" => how == Strength::Compares,
            "i32" | "i64" | "u8" | "u32" | "u64" | "bool" | "char" | "scalar" | "String"
            | "str" | "Bytes" => true,
            // A container compares when what it holds does.
            "Vec"
            | "List"
            | ty::ARRAY
            | "Option"
            | "Set"
            | "collections::HashMap"
            | "collections::HashSet"
            | "collections::BTreeMap"
            | "collections::BTreeSet"
            | "HashMap"
            | "HashSet"
            | "BTreeMap"
            | "BTreeSet"
            | "Shared" => holds(seen),
            _ => {
                // Keyed by the whole type: `Spanned[Stmt]` and `Spanned[Item]`
                // are two questions (ADR-294).
                let key = match args.is_empty() {
                    true => name.to_string(),
                    false => format!("{name}{args:?}"),
                };
                if !seen.insert(key) {
                    // Already being asked about, one level up: a type is not
                    // *un*comparable for holding itself.
                    return true;
                }
                if let Some(fields) = self.structs.get(name) {
                    // **A generic struct is asked about what it was given**: its
                    // fields name its parameters, which answer *yes* as a type
                    // variable does, so `Block { stmts: Vec[Spanned[Stmt]] }`
                    // derived `PartialEq` while `Stmt` did not, and `rustc`
                    // refused the derive (ADR-294).
                    let bound: BTreeMap<String, Ty> = self
                        .parameters
                        .get(name)
                        .map(|params| params.iter().cloned().zip(args.iter().cloned()).collect())
                        .unwrap_or_default();
                    return fields.iter().all(|f| match bound.is_empty() {
                        true => self.asking(&f.ty, how, seen),
                        false => self.asking(&ty::substitute(&f.ty, &bound), how, seen),
                    });
                }
                if self.enums.contains_key(name) {
                    return self.payloads(name).all(|ty| self.asking(ty, how, seen));
                }
                if let Some(parts) = self.beside.get(name) {
                    return parts.iter().all(|ty| self.asking(ty, how, seen));
                }
                // **A name nothing here knows** is another refusal's -
                // `NK1135`, `NK2504` or `rustc`'s - and answers as `Unknown`
                // does rather than as a second refusal naming the wrong cause
                // (ADR-204 D1, #379). Only a name no ledger knows: one that
                // has an entry answers from its column below.
                if nikaia_std::tools::types::nobody_knows(name, self.own, self.library) {
                    return true;
                }
                // **A type whose parts are Rust**: the column, whose absence is
                // no. And it answers the `==` question only — a library type is
                // never taken for an *equivalence*, because whether it is one is
                // a second claim nobody has had a use for. So a `struct` holding
                // one compares and is not a map key yet, which is the
                // under-approximation that cannot be wrong.
                how == Strength::Compares
                    && [self.own, self.library]
                        .into_iter()
                        .filter_map(|ledger| ledger.types.get(name))
                        .any(|contract| contract.compares)
            }
        }
    }

    /// Every type a declared `enum`'s variants hold, **or a library one's**.
    ///
    /// A declared `enum` answers from [`Checker::enum_payloads`]; one that
    /// arrived in a ledger answers from its `variants` column, which carries the
    /// payload types for the same reason.
    fn payloads<'s>(&'s self, name: &str) -> Box<dyn Iterator<Item = &'s Ty> + 's> {
        if let Some(held) = self.payloads.get(name) {
            return Box::new(held.iter());
        }
        let name = name.to_string();
        Box::new(
            [self.own, self.library]
                .into_iter()
                .filter_map(move |ledger| ledger.types.get(&name))
                .flat_map(|contract| {
                    contract
                        .variants
                        .iter()
                        .flat_map(|variant| variant.holds.iter().map(|f| &f.ty))
                }),
        )
    }
}

/// **A part that lets the type holding it derive `Copy`**
/// ([`Checker::collect_copies`]): a number, a truth value, a character, a
/// declared type already found to be one, a Rust type whose description says
/// `copies` (ADR-294 D9.3), and a tuple, a nullable or an array of those.
/// Never a view, a type variable, an unknown or any other Rust crate's type:
/// each is a claim nothing here can back.
fn a_copied_part(ty: &Ty, copies: &BTreeSet<String>) -> bool {
    match ty {
        Ty::Named { name, args, .. } if name == ty::ARRAY => args
            .iter()
            .all(|arg| matches!(arg, Ty::Count(_)) || a_copied_part(arg, copies)),
        Ty::Named {
            name,
            args,
            view: false,
        } => {
            args.is_empty()
                && (matches!(
                    name.as_str(),
                    "i32" | "i64" | "u8" | "u32" | "u64" | "f64" | "bool" | "char" | "scalar"
                ) || copies.contains(name))
        }
        Ty::Nullable(inner) => a_copied_part(inner, copies),
        Ty::Tuple(parts) => parts.iter().all(|part| a_copied_part(part, copies)),
        _ => false,
    }
}

/// is one this says nothing about only when it is also unknown.
fn copies(ty: &Ty) -> bool {
    nikaia_std::tools::check_types::copies_plainly(ty)
}

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

/// **A `std` lookup whose key the language below takes by reference**
/// (0.0.235): the map or set is asked about the key and keeps nothing, so the
/// key is lent ([`Checker::a_lookup_key_lent`]) and not handed over.
fn is_a_lookup(key: &str) -> bool {
    nikaia_std::tools::check_words::is_a_lookup(key)
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

/// **Whether an entry is another package's `sync` that was never promised**
/// ([ADR-288](../../docs/specification/adr/adr-288.md) D28): the one reading of
/// *may pause* that is not a pause.
fn unpromised(contract: &FnContract) -> bool {
    contract.sync_claim == crate::contracts::Sync::Unpromised
}

/// The whole-number types an operation may be written unchecked in
/// (ADR-306 D10).
fn is_a_whole_number(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize"
    )
}
