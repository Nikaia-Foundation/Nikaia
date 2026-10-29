// crates/nikaia/src/view_changes.rs
//
// **What happens to data while a view of it is still used**
// ([ADR-353](../../docs/specification/adr/adr-353.md) D3, D4; Part I 6.6).
// The walk is Nikaia's (`tools/view_changes.nika`, ADR-294); what is left
// here hands it the unit and a literal's holes, which only the parser reads.

use std::collections::BTreeSet;

use crate::check::Finding;
use crate::contracts::Ledger;
use crate::parser::Parsed;

/// What the lowering does at a give-up: an assignment written as a binding of
/// its own, the `let`s that no longer need `mut`, and the `let`s whose buffer
/// lives in the frame's keep (`tools/view_changes.nika`).
pub type GiveUps = nikaia_std::tools::view_changes::VcGiveUps;

/// The refusals (`NK2305`) and the give-ups of one unit.
pub fn analyse(
    parsed: &Parsed,
    own: &Ledger,
    library: &Ledger,
    lent: &BTreeSet<i64>,
    coalesced: &BTreeSet<i64>,
) -> (Vec<Finding>, GiveUps) {
    use nikaia_std::tools::view_changes::{VcAsk, vc_analyse};
    let ask = VcAsk {
        names: &parsed.interner,
        items: &parsed.program.items,
        own,
        library,
        lent,
        coalesced,
        refused: &parsed.refused_moves,
    };
    let answer = vc_analyse(&ask, &|expr| crate::emit::literal_expressions(parsed, expr));
    (
        answer
            .findings
            .into_iter()
            .map(crate::traits::from_nikaia)
            .collect(),
        answer.give_ups,
    )
}

/// **The `??`s whose answer is a view of their left side** (#739): the
/// checker's facts, by the id of the `??` or of its fallback.
pub fn coalesced(facts: &crate::check::CheckFacts) -> BTreeSet<i64> {
    nikaia_std::tools::view_changes::vc_coalesced(facts)
}
