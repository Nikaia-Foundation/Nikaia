// crates/nikaia/src/prove.rs
//
// Every `assert` outside a `test` block, proved while the program is built
// ([ADR-269](../../docs/specification/adr/adr-269.md)).
//
// The prover is `tools/prove_run.nika` (the rounds of walks and runs, the
// integer `comptime`s, what a ledger publishes, whether one contract implies
// another), with its walk and passes in `prover_walk.nika` and
// `prover_passes.nika` (ADR-294, #436). What stays here is what only the
// compiler can answer: the thread's book (`proofs.rs`) and counters, the
// parser's reading of a condition, and the compiler's types around a run.

use std::collections::{BTreeMap, BTreeSet};

use nikaia_std::tools::prove_run;
use nikaia_std::tools::prove_terms;
use nikaia_std::tools::prover_arena::TermArena;
use nikaia_std::tools::prover_solver::SolverAnswer;
use nikaia_std::tools::prover_state::ProverState;
use nikaia_std::tools::solver_terms::SolverTerm;

use crate::ast::{Expr, Item, Stmt};
use crate::check::Finding;
use crate::contracts::Ledger;
use crate::parser::Parsed;

// **What the prover hands the rest of the compiler** is
// `tools/prover_results.nika` (ADR-294, #436).
pub use nikaia_std::tools::prover_results::{
    CallReach as Reach, ClaimHeld as Held, PreconditionCheck as Check, Published, joined_reach,
};

/// What running a call for a claim came to, and the integer `comptime`s
/// (ADR-359 D7, D11, D12): `tools/prove_run.nika`.
pub use nikaia_std::tools::prove_run::{ProveClaimRun as ClaimRun, ProveComptimes as Comptimes};

/// How each call the prover saw reaches its callee, by callee and arguments
/// as written. A call it did not see is not here, and reaches the checked
/// entry.
pub type Reaches = BTreeMap<(String, String), Reach>;

/// What the prover found: its refusals, how each claim it reached is held,
/// keyed as [`crate::check::Checked::claims`] is, the functions that have a
/// checked entry, and how each call reaches them.
#[derive(Debug, Default)]
pub struct Proved {
    pub findings: Vec<Finding>,
    pub held: BTreeMap<(usize, usize), Held>,
    /// By the ledger's key (`f`, `Type::m`): what the checked entry checks.
    pub entries: BTreeMap<String, Vec<Check>>,
    pub reaches: Reaches,
    /// By the ledger's key: what the ledger publishes (ADR-269 D18).
    pub published: BTreeMap<String, Published>,
    /// **`std`'s preconditions at the calls** (ADR-269 D5, #1003): claims
    /// no `assert` states, keyed as `held` is, each written
    /// `callee: condition`.
    pub std_claims: BTreeMap<(usize, usize), String>,
}

/// Read off the checker's `comptime_values` (the type below and the literal,
/// by the start of the item or statement): each one whose type is a whole
/// number and whose literal an `i64` holds.
pub fn comptimes_of(parsed: &Parsed, values: &BTreeMap<i64, (String, String)>) -> Comptimes {
    prove_run::prove_comptimes_of(&parsed.program, &parsed.interner, values)
}

/// **How a call is run while the program is built** (ADR-359 D11): the call
/// as written with each argument the literal of its value, the callee by the
/// ledger's key, and the values. The checker's runs it as a `comptime` runs
/// (ADR-321 D1).
pub type ClaimRunner<'a> = &'a dyn Fn(&Expr, &str, &[i64]) -> ClaimRun;

thread_local! {
    /// **How many calls were run for claims on this thread** (ADR-359 D13): a
    /// program whose claims are decided without a run runs none.
    static CLAIM_RUNS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many calls were run for claims on this thread so far.
pub fn claim_runs() -> usize {
    CLAIM_RUNS.with(|runs| runs.get())
}

thread_local! {
    /// **How many claims were asked a second time, with result summaries, on
    /// this thread** (ADR-359 D2): a program whose claims are decided without
    /// them asks none.
    static SECOND_QUERIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

static TOTAL_RUNS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static TOTAL_SECOND: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static TOTAL_DIVISORS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static TOTAL_LOOP_CLAIMS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static TOTAL_LOOP_QUERIES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static TOTAL_LOOP_SEARCHED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static TOTAL_LOOP_READ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static TOTAL_LOOP_UNFOUND: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static TOTAL_LOOP_OUTSIDE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// **How the invariants used came about in this process** (ADR-361 D4):
/// searched for and found while a build's `nikaia.proofs` was open, or read
/// from it and only checked; the searches that found none; and those
/// found where no file was open, which no file can spare.
pub fn invariant_totals() -> (usize, usize, usize, usize) {
    use std::sync::atomic::Ordering::Relaxed;
    (
        TOTAL_LOOP_SEARCHED.load(Relaxed),
        TOTAL_LOOP_READ.load(Relaxed),
        TOTAL_LOOP_UNFOUND.load(Relaxed),
        TOTAL_LOOP_OUTSIDE.load(Relaxed),
    )
}

/// **What reading through loops came to in this process** (ADR-361 D8):
/// claims asked through a loop, and the queries that found and checked the
/// invariants and asked those claims.
pub fn loop_totals() -> (usize, usize) {
    use std::sync::atomic::Ordering::Relaxed;
    (
        TOTAL_LOOP_CLAIMS.load(Relaxed),
        TOTAL_LOOP_QUERIES.load(Relaxed),
    )
}

/// **What the prover's extra work came to in this process** (ADR-359 D13,
/// D2, D6): calls run for claims, claims asked a second time, divisors asked
/// whether what is known pins them. `NIKAIA_PROVER_COUNTS` prints it when the
/// compiler ends, so a build's cost is counted, not timed.
pub fn totals() -> (usize, usize, usize) {
    use std::sync::atomic::Ordering::Relaxed;
    (
        TOTAL_RUNS.load(Relaxed),
        TOTAL_SECOND.load(Relaxed),
        TOTAL_DIVISORS.load(Relaxed),
    )
}

/// How many claims were asked a second time on this thread so far.
pub fn second_queries() -> usize {
    SECOND_QUERIES.with(|n| n.get())
}

/// The runner `prove_rounds` is lent, counting each run (#1117).
type CountedRun<'a> = std::sync::Arc<dyn Fn(&Expr, &str, &[i64]) -> ClaimRun + 'a>;

/// Prove every `assert` of the program (`prove_run::prove_rounds`). `claims`
/// are the checker's: a call is an `assert` exactly where the checker
/// recognised the prelude's. `run`, where the build can compile and run code,
/// runs a call an open claim reads (ADR-359 D11-D13): only then is the walk
/// repeated, with its value.
pub fn prove(
    parsed: &Parsed,
    own: &Ledger,
    library: &Ledger,
    claims: &BTreeSet<(usize, usize)>,
    comptimes: &Comptimes,
    run: Option<ClaimRunner<'_>>,
) -> Proved {
    use std::sync::atomic::Ordering::Relaxed;
    let claims: BTreeSet<(i64, i64)> = claims
        .iter()
        .map(|(at, node)| (*at as i64, *node as i64))
        .collect();
    // A build that cannot run code gives no runner, and `prove_rounds` asks
    // no round (#1117).
    let runner = run.map(|run| {
        // `Kept` holds an `Arc`; this one lives for the call and crosses no
        // thread.
        #[allow(clippy::arc_with_non_send_sync)]
        let counted: CountedRun<'_> =
            std::sync::Arc::new(move |site: &Expr, callee: &str, args: &[i64]| {
                CLAIM_RUNS.with(|runs| runs.set(runs.get() + 1));
                TOTAL_RUNS.fetch_add(1, Relaxed);
                run(site, callee, args)
            });
        nikaia_std::func::Kept(counted)
    });
    // What the walks searched for, by whether a build's `nikaia.proofs` was
    // open (ADR-361 D4).
    let searched = |state: &ProverState| {
        let searched = match crate::proofs::book_open() {
            true => &TOTAL_LOOP_SEARCHED,
            false => &TOTAL_LOOP_OUTSIDE,
        };
        searched.fetch_add(state.loops_searched as usize, Relaxed);
        TOTAL_LOOP_READ.fetch_add(state.loops_read as usize, Relaxed);
        TOTAL_LOOP_UNFOUND.fetch_add(state.loops_unfound as usize, Relaxed);
    };
    let counted = |state: &ProverState| {
        SECOND_QUERIES.with(|n| n.set(n.get() + state.second_queries as usize));
        TOTAL_SECOND.fetch_add(state.second_queries as usize, Relaxed);
        TOTAL_DIVISORS.fetch_add(state.divisor_queries as usize, Relaxed);
        TOTAL_LOOP_CLAIMS.fetch_add(state.loop_claims as usize, Relaxed);
        TOTAL_LOOP_QUERIES.fetch_add(state.loop_queries as usize, Relaxed);
    };
    let walked = prove_run::prove_rounds(
        &parsed.program,
        &parsed.interner,
        own,
        library,
        &claims,
        comptimes,
        &runner,
        &crate::proofs::asked_so_far,
        &crate::proofs::invariants_here,
        &crate::proofs::ask_again_from,
        &crate::proofs::record_invariants,
        &searched,
        &counted,
        &answer_of,
        &model_of,
        &|name| crate::emit::escaped(name).into_owned(),
        &|e| crate::emit::literal_expressions(parsed, e),
        &|name| parsed.unaliased(name),
        &|e| crate::check::written(parsed, e),
        &condition_nodes,
    );
    finished(walked)
}

fn finished(walked: prove_run::ProveWalked) -> Proved {
    let prove_run::ProveWalked { walk, answers } = walked;
    Proved {
        findings: walk
            .findings
            .into_iter()
            .map(crate::traits::from_nikaia)
            .collect(),
        held: walk
            .held
            .into_iter()
            .map(|((at, node), held)| ((at as usize, node as usize), held))
            .collect(),
        entries: answers.entries,
        reaches: walk.reaches,
        published: answers.published,
        std_claims: walk
            .std_claims
            .into_iter()
            .map(|((at, node), written)| ((at as usize, node as usize), written))
            .collect(),
    }
}

/// **What a `pub` function's contract changed in the breaking direction**
/// since the committed ledger (ADR-269 D19), as warnings: a precondition the
/// old one does not imply - a caller that showed the old may now stop where
/// it is checked - and a postcondition the new one does not imply - a caller's
/// proof that leaned on it may no longer hold. The solver decides each
/// direction; where it cannot show the implication, the change is said.
/// `mine` says which entries are this package's own.
pub fn changed_contracts(
    now: &Ledger,
    committed: &Ledger,
    keepers: &BTreeMap<String, BTreeMap<String, Vec<String>>>,
    mine: impl Fn(&str) -> bool,
) -> String {
    nikaia_std::tools::contract_changes::changed_contracts(
        now,
        committed,
        keepers,
        &|key| mine(key),
        &|from, to, names| prove_run::prove_implies(from, to, names, &answer_of, &condition_nodes),
    )
}

/// **A condition the ledger states, read back** (ADR-269 D18): the text is
/// the language's own syntax, so the compiler's parser reads it, inside an
/// `assert` of a function nobody calls, and the prover's own reading turns it
/// into a term. Only `names` are variables of it.
fn condition(arena: &mut TermArena, text: &str, names: &BTreeSet<String>) -> Option<i64> {
    let source = format!("fn __condition() {{\n    assert({text})\n}}\n");
    let parsed = crate::parser::parse_to_ast(&source).ok()?;
    let Item::Fn { body, .. } = &parsed.program.items.first()?.node else {
        return None;
    };
    let Stmt::Expr(Expr::Call { args, .. }) = &body.stmts.first()?.node else {
        return None;
    };
    prove_terms::claim_term(
        args.first()?,
        &parsed.interner,
        &|name| names.contains(name),
        &|_| None,
        &mut arena.nodes,
    )
}

/// Whether `text` reads back as a condition over `names`: what a ledger may
/// publish of it (ADR-314 D3).
pub(crate) fn reads_back(text: &str, names: &BTreeSet<String>) -> bool {
    condition(&mut TermArena::empty(), text, names).is_some()
}

/// A condition read back on its own, for the walk to adopt
/// (`TermArena::adopt`): its nodes, the root last; none where it does not
/// read back.
pub(crate) fn condition_nodes(text: &str, names: &BTreeSet<String>) -> Vec<SolverTerm> {
    let mut arena = TermArena::empty();
    match condition(&mut arena, text, names) {
        Some(root) => {
            debug_assert_eq!(root as usize + 1, arena.nodes.len());
            arena.nodes
        }
        None => Vec::new(),
    }
}

/// What the solver says to `facts` and `goal`, as `prover_solver` reads it.
fn answer_of(held: &TermArena, facts: &[i64], goal: i64) -> SolverAnswer {
    prove_run::prove_answer(crate::proofs::ask(held, facts, goal))
}

/// A checked model of `facts` and `goal`'s negation: `prove_run::prove_model`.
fn model_of(held: &TermArena, facts: &[i64], goal: i64) -> Option<BTreeMap<String, i64>> {
    prove_run::prove_model(held, facts, goal, crate::proofs::ask(held, facts, goal))
}

/// **The ledger with what the prover published of each function** (ADR-269
/// D17, #776): `prove_run::prove_with_published`.
pub fn with_published(own: &Ledger, published: &BTreeMap<String, Published>) -> Ledger {
    prove_run::prove_with_published(own, published)
}
