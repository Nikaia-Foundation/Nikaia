// crates/nikaia/src/prove.rs
//
// Every `assert` outside a `test` block, proved while the program is built
// ([ADR-269](../../docs/specification/adr/adr-269.md)).
//
// **What it reads** (D9): comparisons of whole numbers built from names,
// literals, `+`, `-` and a `*` by a constant, joined with `&&`, `||` and `!`.
// A name is a variable of the proof only where it is an immutable binding of a
// whole-number type; anything else makes the claim one this prover cannot
// read, and that is never a guess: the claim is checked when the program
// runs (D4).
//
// **What it knows** (D9): the branch an `if` is in, what a branch that always
// leaves has ruled out — `return … if c` is exactly that
// ([ADR-276](../../docs/specification/adr/adr-276.md)) — a `for` over a
// range, a `let`'s value, the function's preconditions (D5) and every claim
// proved before.
//
// **How** (D10) is not decided here: each claim is a query to `nikaia-logic`
// ([ADR-270](../../docs/specification/adr/adr-270.md)) - its facts and its
// goal as terms - and the reference solver there answers it. This file is the
// frontend: it knows the program, and what an answer means for it.
//
// **Soundness around bindings.** A fact names variables by their name, so a
// name bound again drops every fact that mentions it, and a block whose
// bindings this walk cannot see — a lambda's body, a `match` arm — starts with
// no facts and no variables. Losing facts only costs proofs.

use std::collections::{BTreeMap, BTreeSet};

use nikaia_std::tools::logic_search::lg_verify_model;
use nikaia_std::tools::prover_arena::TermArena;
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
}

/// **What the build computed for each integer `comptime`** (ADR-359 D7): an
/// item's by its name, one inside a body by its statement's start.
#[derive(Debug, Default, Clone)]
pub struct Comptimes {
    pub by_name: BTreeMap<String, i64>,
    pub at: BTreeMap<i64, i64>,
}

impl Comptimes {
    /// Read off the checker's `comptime_values` (the type below and the
    /// literal, by the start of the item or statement): each one whose type is
    /// a whole number and whose literal an `i64` holds.
    pub fn of(parsed: &Parsed, values: &BTreeMap<i64, (String, String)>) -> Comptimes {
        let mut found = Comptimes::default();
        let whole = |below: &str| {
            matches!(
                below,
                "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize"
            )
        };
        for (at, (below, written)) in values {
            if !whole(below) {
                continue;
            }
            let Ok(value) = written.trim().replace('_', "").parse::<i64>() else {
                continue;
            };
            found.at.insert(*at, value);
        }
        for item in &parsed.program.items {
            if let Item::Comptime { name, .. } = &item.node
                && let Some(value) = found.at.remove(&(item.span.start as i64))
            {
                found.by_name.insert(parsed.text(*name).to_string(), value);
            }
        }
        found
    }
}

/// How a call is named in [`Reaches`].
pub fn call_key(parsed: &Parsed, callee: &str, args: &[Expr]) -> (String, String) {
    let args: Vec<String> = args
        .iter()
        .map(|a| crate::check::written(parsed, a))
        .collect();
    (callee.to_string(), args.join(", "))
}

/// **What running a call for a claim came to** (ADR-359 D11, D12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimRun {
    /// The value it handed back.
    Value(i64),
    /// Why it was not run: the claim stays a check, and `--asserts` says this.
    NotRun(String),
    /// It ran and stopped, in the words the program would print.
    Stopped(String),
}

/// **How a call is run while the program is built** (ADR-359 D11): the call
/// as written with each argument the literal of its value, the callee by the
/// ledger's key, and the values. The checker's runs it as a `comptime` runs
/// (ADR-321 D1).
pub type ClaimRunner<'a> = &'a dyn Fn(&Expr, &str, &[i64]) -> ClaimRun;

/// How many times the walk is repeated with what the calls it wanted came to:
/// a run's value can make another call's arguments known (`f(g(2))`).
const RUN_ROUNDS: usize = 4;

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

/// Prove every `assert` of the program. `claims` are the checker's: a call is
/// an `assert` exactly where the checker recognised the prelude's. `run`,
/// where the build can compile and run code, runs a call an open claim reads
/// (ADR-359 D11-D13): only then is the walk repeated, with its value.
pub fn prove(
    parsed: &Parsed,
    own: &Ledger,
    library: &Ledger,
    claims: &BTreeSet<(usize, usize)>,
    comptimes: &Comptimes,
    run: Option<ClaimRunner<'_>>,
) -> Proved {
    let mut pinned = comptimes.by_name.clone();
    let mut not_run: BTreeMap<String, String> = BTreeMap::new();
    let mut stopped: BTreeMap<String, String> = BTreeMap::new();
    // **Once per call and its arguments** (D13): two calls of `f(2)` are one
    // run.
    let mut answered: BTreeMap<(String, Vec<i64>), ClaimRun> = BTreeMap::new();
    let mut round = 0;
    loop {
        let state = |recorded: &BTreeMap<i64, String>| {
            let mut state = ProverState::fresh();
            state.pinned = pinned.clone();
            state.comptime_at = comptimes.at.clone();
            state.not_run = not_run.clone();
            state.stopped = stopped.clone();
            state.loops_recorded = recorded.clone();
            state
        };
        // **The invariants in `nikaia.proofs`** (ADR-361 D4) are checked and
        // used without a search. A walk that searched one is repeated with
        // what it found, so that what the file keeps is what the next build,
        // reading it, asks.
        let before = crate::proofs::asked_so_far();
        let mut recorded = crate::proofs::invariants_here();
        let (mut walk, mut answers) = walked(parsed, own, library, claims, state(&recorded));
        count_invariants(&walk.state);
        for _ in 0..2 {
            if walk.state.loops_used == recorded {
                break;
            }
            crate::proofs::ask_again_from(&before);
            recorded = walk.state.loops_used.clone();
            (walk, answers) = walked(parsed, own, library, claims, state(&recorded));
            count_invariants(&walk.state);
        }
        crate::proofs::record_invariants(&walk.state.loops_used);
        SECOND_QUERIES.with(|n| n.set(n.get() + walk.state.second_queries as usize));
        TOTAL_SECOND.fetch_add(
            walk.state.second_queries as usize,
            std::sync::atomic::Ordering::Relaxed,
        );
        TOTAL_DIVISORS.fetch_add(
            walk.state.divisor_queries as usize,
            std::sync::atomic::Ordering::Relaxed,
        );
        TOTAL_LOOP_CLAIMS.fetch_add(
            walk.state.loop_claims as usize,
            std::sync::atomic::Ordering::Relaxed,
        );
        TOTAL_LOOP_QUERIES.fetch_add(
            walk.state.loop_queries as usize,
            std::sync::atomic::Ordering::Relaxed,
        );
        round += 1;
        let wanted: Vec<String> = walk
            .state
            .wanted
            .iter()
            .filter(|call| {
                !pinned.contains_key(*call)
                    && !not_run.contains_key(*call)
                    && !stopped.contains_key(*call)
            })
            .cloned()
            .collect();
        let Some(run) = run.filter(|_| !wanted.is_empty() && round <= RUN_ROUNDS) else {
            return finished(walk, answers);
        };
        for call in wanted {
            let (Some(site), Some((callee, args))) = (
                walk.state.call_sites.get(&call),
                walk.state.call_args.get(&call),
            ) else {
                continue;
            };
            let key = (callee.clone(), args.clone());
            let came = match answered.get(&key) {
                Some(came) => came.clone(),
                None => {
                    CLAIM_RUNS.with(|runs| runs.set(runs.get() + 1));
                    TOTAL_RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let came = run(site, callee, args);
                    answered.insert(key, came.clone());
                    came
                }
            };
            match came {
                ClaimRun::Value(value) => {
                    pinned.insert(call, value);
                }
                ClaimRun::NotRun(why) => {
                    not_run.insert(call, why);
                }
                ClaimRun::Stopped(said) => {
                    stopped.insert(call, said);
                }
            }
        }
    }
}

/// **The passes, and what they make** (`tools/prover_passes.nika`): the walk
/// repeated until the preconditions, then the postconditions, stop changing
/// (ADR-269 D15, D17); the checked entries (D20), what the ledger publishes
/// (D18), and how a claim no walk reached is held.
fn count_invariants(state: &ProverState) {
    use std::sync::atomic::Ordering::Relaxed;
    let searched = match crate::proofs::book_open() {
        true => &TOTAL_LOOP_SEARCHED,
        false => &TOTAL_LOOP_OUTSIDE,
    };
    searched.fetch_add(state.loops_searched as usize, Relaxed);
    TOTAL_LOOP_READ.fetch_add(state.loops_read as usize, Relaxed);
    TOTAL_LOOP_UNFOUND.fetch_add(state.loops_unfound as usize, Relaxed);
}

fn walked(
    parsed: &Parsed,
    own: &Ledger,
    library: &Ledger,
    claims: &BTreeSet<(usize, usize)>,
    state: ProverState,
) -> (ProverWalk, prover_passes::ProverAnswers) {
    let mut walk = ProverWalk {
        state,
        arena: TermArena::empty(),
        held: BTreeMap::new(),
        reaches: BTreeMap::new(),
        findings: Vec::new(),
        claims: claims
            .iter()
            .map(|(at, node)| (*at as i64, *node as i64))
            .collect(),
    };
    let answers = prover_passes::prover_passes(
        &mut walk,
        &parsed.program,
        &parsed.interner,
        own,
        library,
        &|arena, facts, goal| answer_of(arena, facts, goal),
        &|arena, facts, goal| model_of(arena, facts, goal),
        &|name| crate::emit::escaped(name).into_owned(),
        &|e| crate::emit::literal_expressions(parsed, e),
        &|name| parsed.unaliased(name),
        &|e| crate::check::written(parsed, e),
        &|text, names| condition_nodes(text, names),
    );
    (walk, answers)
}

fn finished(walk: ProverWalk, answers: prover_passes::ProverAnswers) -> Proved {
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
    }
}

// **The walk and its passes** are `tools/prover_walk.nika` and
// `tools/prover_passes.nika` (ADR-294, #436): this file hands them what only
// the compiler can answer.
use nikaia_std::tools::prover_passes;
use nikaia_std::tools::prover_solver::SolverAnswer;
use nikaia_std::tools::prover_state::ProverState;
use nikaia_std::tools::prover_walk::ProverWalk;

use nikaia_std::tools::prove_terms;

// --- A program's numbers as terms -----------------------------------------

/// **What a `pub` function's contract changed in the breaking direction**
/// since the committed ledger (ADR-269 D19), as warnings: a precondition the
/// old one does not imply - a caller that showed the old may now stop where
/// it is checked - and a postcondition the new one does not imply - a caller's
/// proof that leaned on it may no longer hold. The solver decides each
/// direction; where it cannot show the implication, the change is said.
/// `mine` says which entries are this package's own.
pub fn changed_contracts(now: &Ledger, committed: &Ledger, mine: impl Fn(&str) -> bool) -> String {
    nikaia_std::tools::contract_changes::changed_contracts(
        now,
        committed,
        &|key| mine(key),
        &|from, to, names| implies(from, to, names),
    )
}

/// Whether the conditions `from` imply every one of `to`, by the reference
/// solver with a checked certificate; `false` where it cannot show it, or a
/// condition does not read back.
fn implies(from: &[String], to: &[String], names: &BTreeSet<String>) -> bool {
    if to.iter().all(|c| from.contains(c)) {
        return true;
    }
    let mut arena = TermArena::empty();
    let Some(facts) = from
        .iter()
        .map(|c| condition(&mut arena, c, names))
        .collect::<Option<Vec<i64>>>()
    else {
        return false;
    };
    let Some(goals) = to
        .iter()
        .map(|c| condition(&mut arena, c, names))
        .collect::<Option<Vec<i64>>>()
    else {
        return false;
    };
    let goal = arena.and(goals);
    crate::proofs::ask(&arena, &facts, goal) == crate::proofs::Asked::Proved
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
    match crate::proofs::ask(held, facts, goal) {
        crate::proofs::Asked::Proved => SolverAnswer::Proved,
        crate::proofs::Asked::Rejected(why) => SolverAnswer::Rejected(why),
        crate::proofs::Asked::Refuted(_) | crate::proofs::Asked::Unknown => SolverAnswer::NotProved,
    }
}

/// A checked model of `facts` and `goal`'s negation: the values it gives.
fn model_of(held: &TermArena, facts: &[i64], goal: i64) -> Option<BTreeMap<String, i64>> {
    let crate::proofs::Asked::Refuted(model) = crate::proofs::ask(held, facts, goal) else {
        return None;
    };
    lg_verify_model(held, facts, goal, &model).then_some(model.values)
}

/// **The ledger with what the prover published of each function** (ADR-269
/// D17, #776): its postconditions are facts about a call's result for the
/// bounds walk, as a described callee's are (ADR-314 D3), and its
/// preconditions goals at a call. The ledger file gets
/// them only after the program is lowered.
pub fn with_published(own: &Ledger, published: &BTreeMap<String, Published>) -> Ledger {
    let mut ledger = own.clone();
    for (key, found) in published {
        if let Some(contract) = ledger.functions.get_mut(key) {
            contract.ensures.extend(found.ensures.iter().cloned());
            // Its preconditions, which the bounds walk proves at a method's
            // call that the prover does not see prove them (ADR-269 D20).
            for required in &found.requires {
                if !contract.requires.contains(required) {
                    contract.requires.push(required.clone());
                }
            }
        }
    }
    ledger
}
