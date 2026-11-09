//! **Which index checks a proof shows are not needed**
//! ([ADR-306](../../docs/specification/adr/adr-306.md)).
//!
//! An index into a list stops the program where it is outside (Part III A.2).
//! `--optimization=remove-bounds-checks:<level>` drops that check where it is
//! **proved** that the position is inside, every time the index is reached -
//! never on a guess, so a program that passes the option means what it meant
//! without it (D2). Two levels:
//!
//! * **`basic`** (D3) reads one shape and asks no solver: `xs[k]` where `k` is
//!   the binding of an enclosing `for k in lo..<xs.len()`, `lo` is not
//!   negative, and nothing in the loop's body can change `xs`'s length.
//! * **`aggressive`** (D4) walks each body forward, keeping the linear facts
//!   that hold at each point - ranges, branch and loop conditions, the
//!   operands of `&&` and `||`, `let`s and assignments, the lengths a list is
//!   built or resized to - and asks the solver whether `0 <= i < xs.len()`
//!   follows. A check is dropped only where the solver's certificate passes
//!   the checker ([ADR-270](../../docs/specification/adr/adr-270.md) D5).
//!
//! What either finds is a set of index nodes, by the address of the indexed
//! expression ([`crate::check::value_node`] of the `base`), which the emitter
//! reads where it writes an index of a list.
//!
//! **`--optimization=remove-overflow-checks:aggressive`**
//! ([ADR-306](../../docs/specification/adr/adr-306.md) D6) asks the same walk
//! whether a `+`, `-` or `*` stays inside its type, and the walk knows more
//! than ADR-306 gave it: a bound on **every value a list holds**, the join of
//! what each write puts in, proved where it is written (D2); `x % n` (D3); and
//! the length a loop that pushes once per turn leaves (D4).

//! The walks and what they read are Nikaia (`tools/bounds_proven.nika`,
//! #1431); here is only the call, with what only the compiler can do handed
//! in as code: the solver, the book of `nikaia.proofs`, the emitter's
//! counters and the fold's proof.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::parser::Parsed;

/// What happens to an index or overflow check the walk proved (ADR-306
/// D14): `tools/emit_build.nika`. The default of both is `Kept`.
pub use nikaia_std::tools::emit_build::{BoundsChecks, OverflowChecks};

/// What the walk proved: index nodes ([`crate::check::value_node`] of the
/// indexed `base`) and arithmetic operators (by the byte their operator
/// starts at).
#[derive(Debug, Default)]
pub struct Proven {
    pub indices: HashSet<i64>,
    pub arithmetic: HashSet<i64>,
    /// Of the `nonnegative` asked, the values proved `>= 0` where they stand
    /// (ADR-285 D32), by node.
    pub nonnegative: HashSet<i64>,
    /// Of the negations asked, those proved to stay inside their type
    /// (ADR-314 D5), by node.
    pub negations: HashSet<i64>,
    /// **Each walked body's counters that are never negative**
    /// (`tools/emit_locals.nika`), by the body's address in the `Parsed`
    /// both walks borrow: the emitter reads them back rather than walking the
    /// body a second time.
    pub nonnegative_locals: HashMap<usize, HashSet<winnow_grammar::Symbol>>,
    /// **The method calls whose callee's precondition holds where they
    /// stand** ([ADR-269](../../docs/specification/adr/adr-269.md) D20, #776),
    /// by the receiver's node, with the callee's ledger key.
    pub calls: BTreeMap<i64, String>,
    /// The derived preconditions, for `--bounds` (ADR-391 D4).
    pub derived: Vec<String>,
    /// What `--bounds-trace` shows (#1495): each goal of the line asked.
    pub trace: Vec<String>,
}

/// **Every index and every operation the walks prove**
/// (`tools/bounds_proven.nika`): `facts` are the checker's, what may be read
/// and proved; `ledgers` answer what a callee ensures (ADR-314 D3);
/// `nonnegative` are the signed values asked `>= 0` (ADR-285 D32); `beside`
/// are the package's other files (ADR-391 D1).
pub fn proven(
    parsed: &Parsed,
    beside: &[&Parsed],
    facts: &crate::check::CheckFacts,
    ledgers: &[&crate::contracts::Ledger],
    nonnegative: &BTreeSet<i64>,
    through_loops: bool,
) -> Proven {
    proven_traced(
        parsed,
        beside,
        facts,
        ledgers,
        nonnegative,
        through_loops,
        &[],
    )
}

/// [`proven`], with the goals of the bytes `trace` (`[from, to]` of the
/// line `--bounds-trace` asks about, #1495) written into `Proven::trace`.
pub fn proven_traced(
    parsed: &Parsed,
    beside: &[&Parsed],
    facts: &crate::check::CheckFacts,
    ledgers: &[&crate::contracts::Ledger],
    nonnegative: &BTreeSet<i64>,
    through_loops: bool,
    trace: &[i64],
) -> Proven {
    let counters = std::cell::RefCell::new(HashMap::new());
    let all = nikaia_std::tools::bounds_proven::bounds_proven(
        &parsed.program,
        &parsed.interner,
        &beside.iter().map(|p| &p.program).collect::<Vec<_>>(),
        &beside.iter().map(|p| &p.interner).collect::<Vec<_>>(),
        facts,
        ledgers,
        nonnegative,
        through_loops,
        trace,
        &|text, names| crate::prove::condition_nodes(text, names),
        &|arena, known, goal| {
            crate::proofs::ask(arena, known, goal) == crate::proofs::Asked::Proved
        },
        &|function, at, arena, known, goal| {
            crate::proofs::in_function(function, at as usize, || {
                crate::proofs::ask(arena, known, goal) == crate::proofs::Asked::Proved
            })
        },
        &|used| crate::proofs::record_bound_invariants(used),
        &|body, symbols| {
            counters.borrow_mut().insert(
                std::ptr::from_ref(body) as usize,
                symbols.iter().copied().collect(),
            );
        },
        &|| crate::fold_bound::proven_counters(parsed, &facts.arithmetic),
    );
    Proven {
        indices: all.indices.into_iter().collect(),
        arithmetic: all.arithmetic.into_iter().collect(),
        nonnegative: all.nonnegative.into_iter().collect(),
        negations: all.negations.into_iter().collect(),
        nonnegative_locals: counters.into_inner(),
        calls: all.calls,
        derived: all.derived,
        trace: all.trace,
    }
}
