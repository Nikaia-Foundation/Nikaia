//! Kap 7.1: which errors can leave a function.
//!
//! `throws` in the source says *that* a function can fail. Which errors is a
//! question about its body and about everything its body reaches, so it is
//! derived here rather than written down — the argument of
//! [ADR-005](../../../../docs/specification/adr/adr-005.md) D3, applied to the
//! second kind of contract in
//! [ADR-023](../../../../docs/specification/adr/adr-023.md) D1.
//!
//! This is [`super::sync`]'s walk with the lattice turned around. `sync` is a
//! **greatest** fixpoint over a boolean: everyone starts pure and loses the
//! claim on contact with something that pauses. An error set is a **least**
//! fixpoint: everyone starts with what they throw themselves, and grows by what
//! their callees throw, until nothing changes. Mutual recursion terminates for
//! the same reason it does there — the sets only grow, and there are finitely
//! many names to grow by.
//!
//! The call resolution is `sync`'s, not a second one. ADR-288 recorded what the
//! alternative costs: two analyses that have to agree about what `stats.add(5)`
//! goes to, and eventually do not. That includes the **method** half of it: the
//! type checker resolves a receiver and both walks read its answer, so
//! `a.add(v)` in ADR-288's `HashMap[&str, Stats]` chain reaches `Stats::add`
//! here for the same reason it reaches it in [`super::sync`]. Before that this
//! file answered every method call with `"?"` - fail-closed and therefore not a
//! bug, but a set that said "something I cannot name" about a call the compiler
//! had already named.

use std::collections::{BTreeMap, BTreeSet};

use super::sync::{Reached, reached};
use super::{Ledger, UNNAMED_ERROR};
use crate::ast::Expr;
use crate::check::MethodCalls;
use crate::parser::Parsed;
use nikaia_std::tools::throws::{self as nika, MethodsCalled, ThrowsGraph};

/// Give every `throws` function of this package its `fails_with`.
///
/// **The walk, the graph and the fixpoint are Nikaia** (`tools/throws.nika`,
/// #125). What stays here is handing in each unit, what one expression
/// reaches (`sync`'s single answer, ADR-288) and the type checker's method
/// calls as plain data, and writing the answer into the ledger. A callee the
/// graph does not hold still has the set the ledger gives it (ADR-296 D40).
pub fn infer(
    ledger: &mut Ledger,
    units: &[&Parsed],
    library: &Ledger,
    resolved: &BTreeMap<String, MethodCalls>,
    stored: &BTreeMap<String, crate::check::StoredCode>,
) {
    let mut graph = ThrowsGraph {
        methods: resolved
            .iter()
            .map(|(key, calls)| {
                let called = MethodsCalled {
                    resolved: calls.resolved.clone(),
                    unseen: calls.unresolved || calls.code_fails,
                    fields: calls.fields_called.clone(),
                };
                (key.clone(), called)
            })
            .collect(),
        direct: BTreeMap::new(),
        calls: BTreeMap::new(),
        fatal_calls: BTreeMap::new(),
    };
    let fatal = fatal_types(ledger, library);
    // **Each function field, name and parameter of code is a place of the
    // graph** (ADR-351 D1, D3, D4).
    let mut stored = stored.clone();
    let open = nika::params_as_places(ledger, &mut stored);
    let stored = &stored;
    let places: BTreeSet<String> = stored.keys().cloned().collect();
    for (field, code) in stored {
        nika::stored_throws(
            field,
            &code.resolved,
            code.unresolved,
            &code.free,
            ledger,
            library,
            &mut graph,
        );
    }
    for parsed in units.iter().copied() {
        let own: &Ledger = ledger;
        nika::unit_throws(
            &parsed.program,
            &parsed.interner,
            &|expr: &Expr| crate::emit::literal_expressions(parsed, expr),
            &|expr: &Expr| contribution(parsed, expr, own, library, &fatal),
            own,
            library,
            &mut graph,
        );
    }
    // **An open parameter place is `"?"` here and `"from(p)"` in the ledger**
    // (ADR-351 D4): the same graph, filled both ways.
    let mut given = graph.direct.clone();
    for key in &open {
        graph
            .direct
            .entry(key.clone())
            .or_default()
            .insert(UNNAMED_ERROR.to_string());
        given
            .entry(key.clone())
            .or_default()
            .insert(format!("from({key})"));
    }
    let given = nika::error_sets(
        &given,
        &graph.calls,
        &graph.fatal_calls,
        &fatal,
        ledger,
        &places,
    );
    for (name, errors) in nika::error_sets(
        &graph.direct,
        &graph.calls,
        &graph.fatal_calls,
        &fatal,
        ledger,
        &places,
    ) {
        if places.contains(&name) {
            ledger.code_throws.insert(name, errors);
            continue;
        }
        let from = nika::throws_from(&given, &name);
        if let Some(contract) = ledger.functions.get_mut(&name) {
            contract.fails_with = errors;
            contract.throws_from = from;
        }
    }
}

/// The ledger's key of a function, `Owner::name` for a method.
#[allow(dead_code)]
fn function_key(parsed: &Parsed, name: Option<&winnow_grammar::Symbol>, owner: &str) -> String {
    let own = name
        .map(|symbol| parsed.interner.resolve(*symbol).to_string())
        .unwrap_or_else(|| "new".to_string());
    match owner {
        "" => own,
        _ => format!("{owner}::{own}"),
    }
}

fn contribution(
    parsed: &Parsed,
    expr: &Expr,
    own: &Ledger,
    library: &Ledger,
    fatal: &BTreeSet<String>,
) -> nikaia_std::tools::throws::Contribution {
    let (call, errors) = match reached(parsed, expr, own, library) {
        Some(Reached::Own(name)) => (Some(name), Vec::new()),
        Some(Reached::Library { key, .. }) => (
            None,
            library
                .functions
                .get(&key)
                .map(|contract| contract.fails_with.clone())
                .unwrap_or_default(),
        ),
        Some(Reached::Opaque(_)) => (None, vec![UNNAMED_ERROR.to_string()]),
        Some(Reached::Method) | None => (None, Vec::new()),
    };
    let fatal = errors
        .iter()
        .filter(|e| fatal.contains(*e))
        .cloned()
        .collect();
    nikaia_std::tools::throws::Contribution {
        call,
        errors,
        fatal,
    }
}

/// **The fatal error types** (ADR-345 D1): every type an `impl Fatal` names,
/// in this package or a library.
pub fn fatal_types(own: &Ledger, library: &Ledger) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for ledger in [own, library] {
        if let Some(types) = ledger.implementations.get("Fatal") {
            out.extend(types.iter().cloned());
        }
    }
    out
}

/// The type a `throw` raises: `tools/throws.nika` answers it (#125).
pub(crate) fn error_type(parsed: &Parsed, thrown: &Expr) -> Option<String> {
    nikaia_std::tools::throws::error_type(&parsed.interner, thrown)
}
