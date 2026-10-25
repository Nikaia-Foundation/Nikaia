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
use crate::ast::{self, Expr, Item, Span};
use crate::build_time;
use crate::contracts::ty::TyOps;
use crate::contracts::{FieldContract, Ledger, ty::Ty};
use crate::fold::Constant;
use crate::parser::Parsed;
use nikaia_std::tools::check_crossings;
use nikaia_std::tools::check_flow::{CheckConstant, CheckFlow};
use nikaia_std::tools::check_items;
use nikaia_std::tools::check_reports::CheckWall;
pub use nikaia_std::tools::check_state::{
    CheckFinding as Finding, CheckLabel as Label, CheckSeverity as Severity,
};
use nikaia_std::tools::check_state::{CheckState, CheckWorld};
use nikaia_std::tools::check_texts::{self};
use nikaia_std::tools::check_walked;
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
    let mut checked = walked(
        parsed, beside, own, library, modules, newly, reads, false, true,
    );
    refuse_unproved_claims(&mut checked);
    if beside.is_empty() && modules.is_empty() {
        dump_accepted(parsed, &checked);
    }
    checked
}

/// **Where the sweep of accepted programs collects them** (#705): the
/// directory `NIKAIA_DUMP_ACCEPTED` names, read once. Unset in every build
/// and every ordinary test run.
pub(crate) fn accepted_dump_dir() -> Option<&'static std::path::Path> {
    static DIR: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        std::env::var_os("NIKAIA_DUMP_ACCEPTED")
            .filter(|dir| !dir.is_empty())
            .map(std::path::PathBuf::from)
    })
    .as_deref()
}

/// **A file the check accepted, written where the sweep reads it** (#705):
/// `<binary>-<hash>.nika`, the binary being the test that checked it and the
/// hash the source's (FNV-1a), so the same program checked twice is one file.
/// A file of a program of several is not written: it needs its project.
fn dump_accepted(parsed: &Parsed, checked: &Checked) {
    let (Some(dir), Some(source)) = (accepted_dump_dir(), parsed.dumped_source.as_deref()) else {
        return;
    };
    if parsed.package.is_some()
        || checked
            .findings
            .iter()
            .any(|f| f.severity == Severity::Error)
    {
        return;
    }
    let binary = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default();
    // A test binary is `<name>-<cargo's hash>`; the name is what a reader knows.
    let binary = match binary.rsplit_once('-') {
        Some((name, hash)) if hash.chars().all(|c| c.is_ascii_hexdigit()) => name.to_string(),
        _ => binary,
    };
    let hash = source.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    let path = dir.join(format!("{binary}-{hash:016x}.nika"));
    if !path.exists() {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(path, source);
    }
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
    let views: Vec<_> = beside.iter().map(|other| parsed_view_of(other)).collect();
    let here_at = beside
        .iter()
        .position(|other| std::ptr::eq(*other, parsed))
        .map_or(-1, |at| at as i64);
    let own_methods = nikaia_std::tools::ty::method_index(own);
    let library_methods = nikaia_std::tools::ty::method_index(library);
    let world = CheckWorld {
        words: &parsed.interner,
        own,
        library,
        own_methods: &own_methods,
        library_methods: &library_methods,
        here: parsed_view_of(parsed),
        parts_beside: nikaia_std::tools::check_copies::check_parts_beside(&views, here_at),
        beside: views,
        here_at,
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
    checker.nikaia(|state, world, host| {
        nikaia_std::tools::check_program::check_program_program(state, world, host)
    });
    // **Only the call sites were wanted**, and everything below this line
    // answers a different question at a cost the asking unit has already paid
    // for itself (ADR-304 D6).
    if harvesting {
        return finished(checker);
    }
    // **The calls that stand in the program's other files**, before the turns
    // are walked, because a copy is written by the unit that declares the
    // function and decided at the call - which may be a file away (Part I 9.1).
    checker.nikaia(|state, world, host| {
        nikaia_std::tools::check_program::check_program_instantiations_beside(state, world, host)
    });
    // **And once more per unrolled turn** (ADR-304 D7, ADR-304 D6), which the
    // walk above is what found: a call may stand above the function it names.
    checker.nikaia(|state, world, host| {
        nikaia_std::tools::check_program::check_program_unroll(state, world, host)
    });
    checker.unsigned_slots_proved(parsed);
    checker.s.walls_a_refusal_explains();
    checker.s.facts.walks_fields = checker.s.walks_fields.clone();
    // ADR-296 D5: the DSL parameters a call forgot, and the ones it invented.
    // A separate walk because it answers a question about a *statement's
    // holes* rather than about a type, and it needs no ledger to answer it.
    checker.s.findings.extend(crate::dsl::check(parsed));
    // **The ledger's pass reports nothing** (#727): what its caller reads is
    // which calls resolve where and what each body stores (`methods`,
    // `stored_code`), never a finding or a proof, so the walks below that
    // answer only with findings and proofs run in the check that reports them.
    if computes {
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
        let (changed, _) = crate::view_changes::analyse(
            parsed,
            own,
            library,
            &checker.s.facts.lent_lets,
            &crate::view_changes::coalesced(&checker.s.facts),
        );
        checker.s.findings.extend(changed);
        // A view handed back that points into a buffer the body owns (`NK2303`).
        // The tether's own refusal
        // ([ADR-283](../../docs/specification/adr/adr-283.md) D9), and a walk of
        // its own for the same reason `views` is: it asks where a *value* points
        // rather than what a type is. It reads both ledgers, because which calls
        // make a buffer is what they say.
        // The plans once, kept for the emitter where they are its own: made
        // against `std` alone, as it makes them (#727).
        let plans = crate::contracts::keep::buffer_plans(parsed, own, library);
        let std = crate::contracts::std_ledger();
        if std::ptr::eq(library, std) || library == std {
            checker.s.facts.keep_plans = Some(plans.clone());
        }
        checker
            .s
            .findings
            .extend(crate::contracts::tether::refusals_of(plans));
    }
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
    // The findings and method sets in their final form (`check_walked.nika`);
    // the order is the span's.
    checker.nikaia(|state, _, _| check_walked::check_walked_settle(state));
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

/// **The integer types a program writes** (Part I 2.2), as the type names
/// they are.
const INTEGERS: [&str; 5] = ["u8", "u32", "u64", "i32", "i64"];

/// **The lossless common type of two integer types** (`tools/check_widen.nika`).
pub fn common_integer(a: &str, b: &str) -> Option<&'static str> {
    check_widen::common_integer(a, b).map(|common| {
        INTEGERS
            .iter()
            .copied()
            .find(|n| *n == common)
            .unwrap_or("i64")
    })
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
    check_walked::check_walked_unrolling(&walks_fields, &unrolled)
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

/// The value as the author wrote it, where this compiler can say so: a literal,
/// a name, a negation, and an ellipsis for the rest. `CheckHost::written`.
pub(super) fn written_by(parsed: &Parsed, value: &Expr) -> String {
    nikaia_std::tools::check_walked::check_walked_written(value, &|name| {
        parsed.text(name).to_string()
    })
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
    world: &'a CheckWorld<'a, 'a, 'a, 'a, 'a, 'a, 'a>,
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
        self.collect_copies();
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

    /// **Every open number of a body, decided** once the body has been walked
    /// ([ADR-285](../../docs/specification/adr/adr-285.md) D24, D25, D27).
    fn numbers_typed_by_their_uses(&mut self) {
        self.s.numbers_typed_by_their_uses();
    }

    // --- statements ---------------------------------------------------------

    // --- expressions --------------------------------------------------------

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
        let asking = Comparable {
            beside: &self.world.parts_beside,
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

    /// **The signed values in unsigned slots, asked of the walk** (ADR-285
    /// D32, [`crate::bounds::proven`]): one proved `>= 0` where it stands is
    /// taken as it is; one that is not is the slot's refusal, with the two
    /// ways to say it.
    fn unsigned_slots_proved(&mut self, parsed: &Parsed) {
        let (own, library) = (self.world.own, self.world.library);
        self.nikaia(|state, _, _| {
            check_walked::check_walked_unsigned_slots(state, &|facts, nodes| {
                crate::bounds::proven(
                    parsed,
                    &crate::bounds::Sites::of(facts),
                    &[own, library],
                    nodes,
                    // What the checker accepts does not depend on an optimization
                    // (ADR-361 D10).
                    false,
                )
                .nonnegative
                .into_iter()
                .collect()
            })
        });
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
}

/// **Whether this file declares a type by this name**
/// ([ADR-286](../../docs/specification/adr/adr-286.md) D31).
///
/// For the package's *other* files, which share one namespace with this one
/// (Part I 9.1). It reads the item tree rather than a ledger because a ledger
/// records what a type **promises**, and what is asked here is only that the
/// word was written down: a private `enum` a neighbouring file declares is a
/// head, whatever its contract says.
/// What the checker reads of a parsed file, borrowed (`tools/parsed_view.nika`).
fn parsed_view_of(parsed: &Parsed) -> ParsedView<'_, '_, '_, '_, '_, '_> {
    ParsedView {
        words: &parsed.interner,
        aliases: &parsed.aliases,
        items: &parsed.program.items,
        package: parsed.package.as_deref(),
        spans: &parsed.expr_spans,
        viewed_arms: &parsed.viewed_arms,
    }
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
/// **A `'…'` as the language below writes it** (ADR-320 D8): as written
/// where it is one scalar, and the one scalar NFC composes it to, as
/// `'\u{…}'`, where it is several - the checker has said `NK1213` there, and
/// refused (`NK1212`) every literal of several that composes to more than one.
pub fn char_literal(written: &str) -> String {
    let Some(value) = crate::build_time::decoded(written) else {
        return format!("'{written}'");
    };
    if value.chars().count() <= 1 {
        return format!("'{written}'");
    }
    let composed = nikaia_std::text::normalized(&value, nikaia_std::text::Form::Nfc);
    match composed.chars().next() {
        Some(one) => format!("'\\u{{{:x}}}'", one as u32),
        None => format!("'{written}'"),
    }
}

pub fn byte_literal(written: &str) -> String {
    nikaia_std::tools::check_texts::check_byte_literal(written)
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
/// **Red and green only on a terminal**, and never where `NO_COLOR` is set
/// (Part III, `--asserts`): the caller decides and passes `colour`.
///
/// **Checked as the build checks it**: with the program's other files and
/// what the build may read and compile, so that a `comptime` a call computes
/// and a call run for a claim (ADR-359 D7, D11) answer here as they did there.
#[allow(clippy::too_many_arguments)]
pub fn claims_report(
    parsed: &Parsed,
    source: &str,
    path: &str,
    beside: &[&Parsed],
    own: &Ledger,
    library: &Ledger,
    reads: &Reads,
    refuses: bool,
    colour: bool,
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
    nikaia_std::tools::check_facts::check_claims_report(
        &checked.facts.claims,
        source,
        path,
        &checked.facts.published,
        colour,
        refuses,
    )
}

/// The prelude's claim ([ADR-269](../../docs/specification/adr/adr-269.md) D2).
pub const ASSERT: &str = "assert";

/// **A claim the compiler did not prove is refused** (`NK1300`,
/// [ADR-380](../../docs/specification/adr/adr-380.md) D1): an `assert` it
/// checks when the program runs, and a precondition some call does not prove.
/// Not proved is not false, and the message says so. The build's
/// `unproved-claims = "warn"` lowers each to a warning and keeps its check
/// (D2); that is the project's, as `refuted-claims` is. A precondition of
/// `main`, checked where the words arrive, a test's verdict and an `assume`
/// are not asked; nor is a claim shown false (`NK1207`), which is worse than
/// not proved and told already, or a precondition only calls that break it
/// every time leave open.
pub fn refuse_unproved_claims(checked: &mut Checked) {
    use nikaia_std::tools::prover_results::ClaimHeld;
    let refuted: Vec<&Finding> = checked
        .findings
        .iter()
        .filter(|f| f.code == "NK1207")
        .collect();
    let mut unproved = Vec::new();
    for ((start, _), claim) in &checked.facts.claims {
        if claim.assumed || refuted.iter().any(|f| i64::from(f.span.start) == *start) {
            continue;
        }
        let message = match &claim.held {
            Some(ClaimHeld::AtRunTime(why)) => {
                format!("`{}` is not proved: {why}.", claim.written)
            }
            Some(ClaimHeld::Precondition(f, n)) if f != "main" => {
                // The calls `NK1207` told break it every time.
                let broken = refuted
                    .iter()
                    .filter(|r| r.message.contains(&format!("`{f}`'s precondition")))
                    .count() as i64;
                let calls = match n - broken {
                    n if n <= 0 => continue,
                    1 => "one call does".to_string(),
                    n => format!("{n} calls do"),
                };
                format!(
                    "`{}`, the precondition of `{f}`, is not proved: {calls} not prove it.",
                    claim.written
                )
            }
            _ => continue,
        };
        let start = *start as u32;
        unproved.push(Finding {
            severity: Severity::Error,
            span: Span {
                start,
                end: start + "assert".len() as u32,
            },
            code: "NK1300".to_string(),
            message,
            notes: vec![
                "Not proved is not false: the compiler could not show the claim from what it \
                 knows where it stands, and it may well hold."
                    .to_string(),
                "`nikaia check --asserts` lists every claim and why it is not proved.".to_string(),
            ],
            help: Some(
                "Give the compiler what it needs to prove it, or build with \
                 `unproved-claims = \"warn\"` to have it checked when the program runs."
                    .to_string(),
            ),
            labels: Vec::new(),
        });
    }
    checked.findings.extend(unproved);
}

/// **`std` assumes nothing** ([ADR-376](../../docs/specification/adr/adr-376.md)
/// D3): every `assume` of a file of `std`'s Nikaia is refused (`NK1295`). The
/// checker keeps each `assume` as a claim; only the caller that checks `std`
/// knows the file is `std`'s.
pub fn refuse_assumes_in_std(checked: &mut Checked) {
    for ((start, _), claim) in &checked.facts.claims {
        if !claim.assumed {
            continue;
        }
        let start = *start as u32;
        checked.findings.push(Finding {
            severity: Severity::Error,
            span: Span {
                start,
                end: start + "assume".len() as u32,
            },
            code: "NK1295".to_string(),
            message: "`std` assumes nothing: an `assume` is refused in `std`'s Nikaia.".to_string(),
            notes: vec![
                "`std`'s `unsafe` lives in the small crates of ADR-218, each listed.".to_string(),
            ],
            help: Some(
                "Prove the claim with `assert`, or move the code into a listed crate.".to_string(),
            ),
            labels: Vec::new(),
        });
    }
}

/// In Nikaia (`tools/written.nika`, ADR-294, #125).
pub fn written(parsed: &Parsed, expr: &Expr) -> String {
    nikaia_std::tools::written::written_expression(&parsed.interner, expr)
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
