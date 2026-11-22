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

#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::parser::Parsed;

/// What happens to an index or overflow check the walk proved (ADR-306
/// D14): `tools/emit_build.nika`. The default of both is `Kept`.
pub use nikaia_std::compiler::emit_build::{BoundsChecks, OverflowChecks};

/// What the walk proved: index nodes ([`crate::check::value_node`] of the
/// indexed `base`) and arithmetic operators (by the byte their operator
/// starts at).
#[derive(Debug, Default, Clone)]
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
    /// What keeps each function from its callers' bounds (#1863).
    pub gates: BTreeMap<String, String>,
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

/// **A unit's proof, made once for its ledger and taken by its lowering**
/// ([ADR-361](../../docs/specification/adr/adr-361.md) D16): the ledger's
/// panics inference proves a unit through loops ([`proven_for_lowering`]),
/// and the lowering takes that proof ([`proven_taken`]) where it would ask
/// the same question, by the same parsed file, the same files beside it, the
/// same checker facts the walks read and the same contracts. Where any of
/// them differs (a build that leaves values, a ledger without the unit, a
/// published `ensures` the ledger did not hold) the lowering asks again.
static LEDGER_PROOFS: std::sync::Mutex<Option<HashMap<usize, Kept>>> = std::sync::Mutex::new(None);

/// A kept proof: the question it answered ([`question_of`]) and its answer.
type Kept = (Vec<u64>, Proven);

/// [`proven`] through loops, with the answer kept for the unit's lowering.
pub fn proven_for_lowering(
    parsed: &Parsed,
    beside: &[&Parsed],
    facts: &crate::check::CheckFacts,
    ledgers: &[&crate::contracts::Ledger],
) -> Proven {
    let out = proven(parsed, beside, facts, ledgers, &BTreeSet::new(), true);
    let question = question_of(parsed, beside, facts, ledgers);
    if let Ok(mut kept) = LEDGER_PROOFS.lock() {
        kept.get_or_insert_with(HashMap::new)
            .insert(unit_of(parsed), (question, out.clone()));
    }
    out
}

/// [`proven`] through loops, or the ledger's answer to the same question.
pub fn proven_taken(
    parsed: &Parsed,
    beside: &[&Parsed],
    facts: &crate::check::CheckFacts,
    ledgers: &[&crate::contracts::Ledger],
) -> Proven {
    let held = LEDGER_PROOFS
        .lock()
        .ok()
        .and_then(|mut kept| kept.as_mut()?.remove(&unit_of(parsed)));
    if let Some((question, proven)) = held
        && question == question_of(parsed, beside, facts, ledgers)
    {
        return proven;
    }
    proven(parsed, beside, facts, ledgers, &BTreeSet::new(), true)
}

/// **The ledger's answer, if it answered this question**, left kept: the
/// `--bounds` report asks through loops and without a trace what
/// [`proven_for_lowering`] asked (#2711), and the lowering after it may still
/// take the same proof ([`proven_taken`]). Where the questions differ (the
/// ledger's check runs no prover, so a unit whose `assert`s publish a
/// `requires` or an `ensures` is asked with them only here) it is `None`.
pub fn proven_kept(
    parsed: &Parsed,
    beside: &[&Parsed],
    facts: &crate::check::CheckFacts,
    ledgers: &[&crate::contracts::Ledger],
) -> Option<Proven> {
    let kept = LEDGER_PROOFS.lock().ok()?;
    let (question, proven) = kept.as_ref()?.get(&unit_of(parsed))?;
    (*question == question_of(parsed, beside, facts, ledgers)).then(|| proven.clone())
}

/// A unit by its items, whose addresses the proof's counters are keyed by.
fn unit_of(parsed: &Parsed) -> usize {
    parsed.program.items.as_ptr() as usize
}

/// **What a proof of `parsed` reads**, as one number: the unit and the files
/// beside it by address, the facts `tools/bounds_proven.nika` reads, and of
/// each ledger, in order, what `tools/bounds_matched.nika` reads of a
/// function (its signature, `ensures`, `ensures_each`, `requires`, `from`).
fn question_of(
    parsed: &Parsed,
    beside: &[&Parsed],
    facts: &crate::check::CheckFacts,
    ledgers: &[&crate::contracts::Ledger],
) -> Vec<u64> {
    use std::fmt::Write as _;
    use std::hash::Hasher as _;
    let mut files = Digest(std::hash::DefaultHasher::new());
    let _ = write!(files, "{} {}", unit_of(parsed), parsed.program.items.len());
    for other in beside.iter().filter(|p| !std::ptr::eq(**p, parsed)) {
        let _ = write!(files, " {}", unit_of(other));
    }
    let mut read = Digest(std::hash::DefaultHasher::new());
    let _ = write!(
        read,
        "{:?}",
        (
            (
                &facts.std_lengths,
                &facts.sized_lengths,
                &facts.char_codes,
                &facts.std_receivers,
                &facts.simd_loads,
                &facts.shifts,
                &facts.divisions,
                &facts.arithmetic,
                &facts.negations,
                &facts.whole_fields,
            ),
            (
                &facts.whole_types,
                &facts.whole_loops,
                &facts.whole_parts,
                &facts.array_lengths,
                &facts.comptime_values,
                &facts.list_indices,
                &facts.text_bytes,
                &facts.text_scalars,
                &facts.summaries,
                &facts.wide_calls,
            ),
        )
    );
    [files.0.finish(), read.0.finish()]
        .into_iter()
        .chain(ledgers.iter().map(|ledger| ledger_digest(ledger)))
        .collect()
}

/// What [`question_of`] reads of one ledger; `std`'s, the same in every
/// question, worked out once.
fn ledger_digest(ledger: &crate::contracts::Ledger) -> u64 {
    static STD: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    let digest = || {
        use std::fmt::Write as _;
        use std::hash::Hasher as _;
        let mut out = Digest(std::hash::DefaultHasher::new());
        for (key, c) in &ledger.functions {
            let _ = write!(
                out,
                "{key:?}{:?}",
                (
                    &c.signature,
                    &c.ensures,
                    &c.ensures_each,
                    &c.requires,
                    &c.from
                )
            );
        }
        out.0.finish()
    };
    if std::ptr::eq(ledger, crate::contracts::std_ledger()) {
        *STD.get_or_init(digest)
    } else {
        digest()
    }
}

/// A hasher written to as text, so a `Debug` form is hashed without being
/// built.
struct Digest(std::hash::DefaultHasher);

impl std::fmt::Write for Digest {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        std::hash::Hasher::write(&mut self.0, s.as_bytes());
        Ok(())
    }
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
    let all = nikaia_std::compiler::bounds_proven::bounds_proven(
        &parsed.program,
        &parsed.interner,
        // **Only the other files** (ADR-391 D1): a caller that hands the
        // whole package (`modules.rs`, the emitter's `ContractsHost`) names
        // this file among them, and read beside itself each field it declares
        // is "declared in a file beside", no rule's candidate (#2447).
        &beside
            .iter()
            .filter(|p| !std::ptr::eq(**p, parsed))
            .map(|p| p.bounds_beside())
            .collect::<Vec<_>>(),
        facts,
        ledgers,
        nonnegative,
        through_loops,
        trace,
        &mut |text, names| crate::prove::condition_nodes(text, names),
        &mut |arena, known, goal| {
            crate::proofs::ask(arena, known, goal) == crate::proofs::Asked::Proved
        },
        &mut |function, at, arena, known, goal| {
            crate::proofs::in_function(function, at as usize, || {
                crate::proofs::ask(arena, known, goal) == crate::proofs::Asked::Proved
            })
        },
        &mut |used| crate::proofs::record_bound_invariants(used),
        &mut |body, symbols| {
            counters.borrow_mut().insert(
                std::ptr::from_ref(body) as usize,
                symbols.iter().copied().collect(),
            );
        },
        &mut || crate::fold_bound::proven_counters(parsed, &facts.arithmetic),
    );
    Proven {
        indices: all.indices.into_iter().collect(),
        arithmetic: all.arithmetic.into_iter().collect(),
        nonnegative: all.nonnegative.into_iter().collect(),
        negations: all.negations.into_iter().collect(),
        nonnegative_locals: counters.into_inner(),
        calls: all.calls,
        derived: all.derived,
        gates: all.gates,
        trace: all.trace,
    }
}
