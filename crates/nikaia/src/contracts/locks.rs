#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};
// crates/nikaia/src/contracts/locks.rs
//
// Whether a function **touches a lock** (ADR-281 D3): the adapter over
// `nikaia-std/src/compiler/locks.nika`, which holds the walk, the fixpoint, the
// answer written into the ledger and why it is built so (#1913).

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Block, Expr};
use crate::parser::Parsed;

use super::Ledger;
use nikaia_std::compiler::locks as nika;

/// The free calls a lambda's body makes, for the checker to remember with the
/// function field it is stored in (ADR-230 D1).
pub fn free_calls(parsed: &Parsed, body: &Block) -> BTreeSet<String> {
    nika::free_calls(
        body,
        &parsed.interner,
        &mut |name: &str| parsed.unaliased(name),
        &mut |expr: &Expr| crate::emit::literal_expressions(parsed, expr),
    )
}

/// Give every function of this package its `touches_a_lock`.
///
/// `resolved` is the checker's answer about method calls, keyed by the same
/// function names the ledger uses; what the walk reads of it is what was
/// resolved **outside** a `spawn` (D3) - the checker records which side each
/// call was on, because only it knows.
/// The `locks` column: `locks.nika`'s `locks_infer` (#1913), over each unit's
/// view, with the holes the emitter reads in a unit's expression.
pub fn infer(
    ledger: &mut Ledger,
    units: &[&Parsed],
    library: &Ledger,
    resolved: &BTreeMap<String, crate::check::MethodCalls>,
    stored: &BTreeMap<String, crate::check::StoredCode>,
) {
    let views: Vec<_> = units
        .iter()
        .map(|parsed| crate::check::parsed_view_of(parsed))
        .collect();
    nika::locks_infer(
        ledger,
        &views,
        library,
        resolved,
        stored,
        &mut |unit: i64, expr: &Expr| crate::emit::literal_expressions(units[unit as usize], expr),
    );
}
