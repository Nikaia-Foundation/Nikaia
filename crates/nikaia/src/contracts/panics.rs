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
use nikaia_std::tools::panics::{self as nika, PanicFacts, PanicsGraph};
use nikaia_std::tools::throws::MethodsCalled;

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
    }
}

/// Give every function of this package its `panics`.
pub fn infer(
    ledger: &mut Ledger,
    units: &[&Parsed],
    library: &Ledger,
    resolved: &BTreeMap<String, MethodCalls>,
    checked: &[Checked],
    parallel: bool,
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
        }
    };
    let walk = |graph: &mut PanicsGraph, at: usize, facts: &PanicFacts, own: &Ledger| {
        let parsed = units[at];
        nika::unit_panics(
            &parsed.program,
            &parsed.interner,
            &|expr: &Expr| crate::emit::literal_expressions(parsed, expr),
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
    let first = nika::panics_of_graph(&graph, ledger);
    // The second: each check a site, but where the walk proved it. The units
    // worth proving are proved on every core where `parallel` (#1277).
    let worth: Vec<usize> = walked
        .iter()
        .enumerate()
        .filter(|(_, keys)| nika::panics_worth_proving(&graph, &first, keys))
        .map(|(at, _)| at)
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
        crate::proofs::in_unit(at, || {
            crate::bounds::proven(
                units[at],
                &beside,
                facts,
                &[own, library],
                &BTreeSet::new(),
                // What the ledger says does not depend on an optimization
                // (ADR-361 D10).
                false,
            )
        })
    };
    let threads = if parallel { None } else { Some(1) };
    let mut proofs: BTreeMap<usize, crate::bounds::Proven> = worth
        .iter()
        .copied()
        .zip(crate::sysroot::in_parallel(&worth, threads, prove))
        .collect();
    for at in 0..units.len() {
        let proven = proofs.remove(&at).unwrap_or_default();
        walk(&mut graph, at, &facts_of(at, Some(proven)), ledger);
    }
    for (name, panics) in nika::panics_of_graph(&graph, ledger) {
        if let Some(contract) = ledger.functions.get_mut(&name) {
            contract.panics = panics;
        }
    }
}
