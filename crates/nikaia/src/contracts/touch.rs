#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};
// crates/nikaia/src/contracts/touch.rs
//
// What an operation reaches, and whether it changes it (ADR-292).
//
// Part I 8.1.1's rule is one sentence - *two operations whose touch sets are
// disjoint have no order between them* - and everything hard about it is in
// what a "touch set" is allowed to say:
//
//     touches = ["file(path) read"]      the file named by the `path` parameter
//     touches = ["stdout write"]         a resource with no parameter
//
// **An absent `touches` means it touches everything.** That is not a default
// chosen for convenience; it is the same fail-closed polarity ADR-010 D1 set
// for provenance and ADR-288 D2 for `sync`, and it is what makes this
// adoptable: a program built against libraries that describe nothing keeps
// exactly the order it has today, and gets faster only where somebody wrote
// enough down for the compiler to prove it may.
//
// **The kinds are a closed list, and that is a soundness property rather than
// tidiness.** A kind this file did not know used to parse into a resource
// nothing else could name - so `stdoutt write` was disjoint from `stdout
// write`, and a typo in a hand-maintained ledger bought an overlap instead of
// failing to buy one. A vocabulary whose *mistakes* are permissions is a
// vocabulary in the wrong polarity. [`Touch::kind_is_known`] is what says
// which names mean something (`touch_kind_is_known` in
// `nikaia-std/src/compiler/touch.nika`, with the list of kinds), and
// `contracts::order` answers a `touches` entry naming anything else the way
// ADR-292 D3 answers every other thing it cannot read: it reaches
// everything, and stays where it was written.

use anyhow::{Result, anyhow};
pub use nikaia_std::compiler::ty::Touch;

/// What stays Rust of a [`Touch`]: reading one back and asking whether its
/// kind is known, as methods for the Rust callers. Both answers are
/// `tools/touch.nika`'s and `tools/ledger.nika`'s (ADR-294 step (b)).
pub trait TouchOps: Sized {
    fn parse(text: &str) -> Result<Self>;
    fn kind_is_known(&self) -> bool;
}

impl TouchOps for Touch {
    /// Read one back from the text a ledger writes.
    fn parse(text: &str) -> Result<Touch> {
        nikaia_std::compiler::ledger::touch_of(text).map_err(|refusal| anyhow!("{refusal}"))
    }

    /// Whether this compiler knows what the named resource *is* (ADR-292 D3).
    fn kind_is_known(&self) -> bool {
        nikaia_std::compiler::touch::touch_kind_is_known(self)
    }
}

/// Which resource a *call* reaches, with the parameter filled in, and when
/// two of them force an order: `tools/touch.nika`'s (#125).
pub use nikaia_std::compiler::touch::Reached;

// --- what a body reaches (ADR-288 D22) ---------------------------------------

use std::collections::BTreeMap;

use crate::ast::Expr;
use crate::check::MethodCalls;
use crate::contracts::Ledger;
use crate::parser::Parsed;
use nikaia_std::compiler::touch as nika;

/// Give every function in the ledger the `touches` its body earns
/// ([ADR-288](../../../../docs/specification/adr/adr-288.md) D22): the walk,
/// the fixpoint (callees first, ADR-100 D7) and the answer written only where
/// nobody said are `touch.nika`'s `touches_infer`. What stays here is building
/// each unit's view from the Rust `Parsed` and the holes the emitter reads in
/// a unit's expression, both Rust until `Parsed` itself is Nikaia's.
pub fn infer(
    ledger: &mut Ledger,
    units: &[&Parsed],
    library: &Ledger,
    resolved: &BTreeMap<String, MethodCalls>,
) {
    let views: Vec<_> = units
        .iter()
        .map(|parsed| crate::check::parsed_view_of(parsed))
        .collect();
    nika::touches_infer(
        ledger,
        &views,
        library,
        resolved,
        &mut |unit: i64, expr: &Expr| crate::emit::literal_expressions(units[unit as usize], expr),
    );
}
