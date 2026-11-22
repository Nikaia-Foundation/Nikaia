//! Whether a function can panic (ADR-async-5 D35, #1355).
//!
//! **The walk, the graph and the fixpoint are Nikaia** (`tools/panics.nika`).
//! What stays here is handing in each unit with what its check and the bounds
//! walk (ADR-306) found, which only Rust holds, and writing the answer into
//! the ledger.
//!
//! The bounds walk is asked only where it can change an answer: a first round
//! leaves every check out, and a unit is proved only where one of its
//! functions then answers "cannot panic" and has a check. A unit not proved
//! counts each of its checks as a site.

#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};
use std::collections::{BTreeMap, BTreeSet};

use super::Ledger;
use crate::ast::Expr;
use crate::check::{Checked, MethodCalls};
use crate::parser::Parsed;
use nikaia_std::compiler::panics::{self as nika, PanicFacts, PanicsGraph};
use nikaia_std::compiler::throws::MethodsCalled;

/// **What a task's walk is handed** (D35): the unit's checks, each a site,
/// nothing proved; the checker asks the same (`check_core_async_crash`).
pub fn task_facts(facts: Option<&crate::check::CheckFacts>) -> PanicFacts {
    let empty = crate::check::CheckFacts::empty();
    let facts = facts.unwrap_or(&empty);
    PanicFacts {
        checked: true,
        arithmetic: facts.arithmetic.clone(),
        negations: facts.negations.clone(),
        shifts: facts.shifts.clone(),
        map_keys: facts.map_keys.keys().map(|(key, _)| *key).collect(),
        proven_indices: BTreeSet::new(),
        proven_arithmetic: BTreeSet::new(),
        proven_negations: BTreeSet::new(),
        placed: facts.placed_in_block.clone(),
    }
}

/// Give every function of this package its `panics`.
pub fn infer(
    ledger: &mut Ledger,
    units: &[&Parsed],
    library: &Ledger,
    resolved: &BTreeMap<String, MethodCalls>,
    checked: &[Checked],
) {
    let mut graph = PanicsGraph {
        methods: resolved
            .iter()
            .map(|(key, calls)| {
                let called = MethodsCalled {
                    resolved: calls.resolved.clone(),
                    unseen: calls.unresolved,
                    fields: calls.fields_called.clone(),
                    swallowed: Default::default(),
                };
                (key.clone(), called)
            })
            .collect(),
        reaches: BTreeMap::new(),
    };
    let facts_of = |at: usize, proven: Option<crate::bounds::Proven>| -> PanicFacts {
        let empty = crate::check::CheckFacts::empty();
        let facts = checked.get(at).map(|c| &c.facts).unwrap_or(&empty);
        let checked = proven.is_some();
        let proven = proven.unwrap_or_default();
        PanicFacts {
            checked,
            arithmetic: facts.arithmetic.clone(),
            negations: facts.negations.clone(),
            shifts: facts.shifts.clone(),
            map_keys: facts.map_keys.keys().map(|(key, _)| *key).collect(),
            proven_indices: proven.indices.iter().copied().collect(),
            proven_arithmetic: proven.arithmetic.iter().copied().collect(),
            proven_negations: proven.negations.iter().copied().collect(),
            placed: facts.placed_in_block.clone(),
        }
    };
    let walk = |graph: &mut PanicsGraph, at: usize, facts: &PanicFacts, own: &Ledger| {
        let parsed = units[at];
        nika::unit_panics(
            &parsed.program,
            &parsed.interner,
            &mut |expr: &Expr| crate::emit::literal_expressions(parsed, expr),
            facts,
            own,
            library,
            graph,
        )
    };
    // The first round: every check left out.
    let mut walked = Vec::new();
    for at in 0..units.len() {
        walked.push(walk(&mut graph, at, &facts_of(at, None), ledger));
    }
    let first = nika::panics_callees_first(&graph, ledger);
    // The second: each check a site, but where the walk proved it. The units
    // worth proving are proved on every core (#1277), in a project's build as
    // in `lower-std`'s (#1949): `proofs::in_parallel` carries the build's
    // thread-local state to the workers.
    let worth: Vec<usize> = nika::panics_worth(&graph, &first, &walked)
        .into_iter()
        .map(|at| at as usize)
        .collect();
    let own: &Ledger = ledger;
    let prove = |at: usize| {
        let beside: Vec<&Parsed> = units
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != at)
            .map(|(_, other)| *other)
            .collect();
        let empty = crate::check::CheckFacts::empty();
        let facts = checked.get(at).map(|c| &c.facts).unwrap_or(&empty);
        // **Through loops, with the unit's published contracts, as its
        // lowering asks** (ADR-361 D16): the lowering takes this proof
        // rather than proving the unit a second time.
        let published =
            nikaia_std::compiler::prove_run::prove_with_published(own, &facts.published);
        crate::proofs::in_unit(at, || {
            let summed = with_summaries(units[at], facts, own, library);
            crate::bounds::proven_for_lowering(units[at], &beside, &summed, &[&published, library])
        })
    };
    // The largest first, so no core is left with a long file at the end; the
    // answers come back in the units' order either way.
    let mut order = worth.clone();
    order.sort_by_key(|at| std::cmp::Reverse(units[*at].program.items.len()));
    let mut proofs: BTreeMap<usize, crate::bounds::Proven> = worth
        .iter()
        .copied()
        .zip(crate::proofs::in_parallel(&order, None, prove))
        .collect();
    for at in 0..units.len() {
        let proven = proofs.remove(&at).unwrap_or_default();
        walk(&mut graph, at, &facts_of(at, Some(proven)), ledger);
    }
    // **Whether it builds a list in its task's block** (ADR-327 D13) and
    // its `panics`, callees first (ADR-100 D7).
    nika::panics_settle(&graph, ledger);
}

/// **The unit's facts with the result summaries its check proves** (ADR-359
/// D14, #2684): the ledger's check proves nothing (#727), so the private
/// functions' summaries the program's check hands its lowering are proved
/// here, by the same prover, and the lowering's question matches the
/// ledger's.
fn with_summaries(
    parsed: &Parsed,
    facts: &crate::check::CheckFacts,
    own: &Ledger,
    library: &Ledger,
) -> crate::check::CheckFacts {
    let claims: BTreeSet<(i64, i64)> = facts.claims.keys().cloned().collect();
    let comptimes = crate::prove::comptimes_of(
        parsed,
        &facts.comptime_values,
        &facts.whole_lets,
        &facts.method_callees,
        &facts.field_owners,
        &facts.outside,
        &facts.zero_divisors,
        &facts.arithmetic,
        &facts.divisions,
        &facts.unfit_numbers,
        &facts.map_indices,
    );
    let mut summed = facts.clone();
    summed.summaries =
        crate::prove::prove(parsed, own, library, &claims, &comptimes, None).summaries;
    summed
}
