#[allow(unused_imports)]
use nikaia_std::block::names::{String, ToText, Vec, format, vec};
// crates/nikaia/src/contracts/tether.rs
//
// Which of Part I 6.6's three states each view in a signature is in
// ([ADR-tether-v1](../../../../docs/specification/adr/adr-tether-v1.md) D2, D5).
//
// ## What this is, and what it deliberately is not
//
// D2 gives every view one of three states and says the compiler picks the least
// one that makes the program valid:
//
//     Borrowed ⊑ Tethered ⊑ Owned
//     Transient = borrow. Escaping = tether. Copying = yours to ask for.
//
// **Only the first of those is built as a representation.** Borrowed is the
// language below's own lifetime and costs nothing; Owned is `.to_owned()`,
// written by the program; and **Tethered is not built at all** — where a value
// would tether, the program is refused today (`NK2302`, or the backend on the
// Nikaia line for a view of a local that escapes). So this file computes the
// state and **changes no lowering**. It writes a column, which is D7, and the
// column is what a later change reads when the representation exists.
//
// That order is deliberate: an analysis whose answers nothing depends on can be
// held against the whole corpus and read, and being wrong costs a wrong line in
// a file rather than a wrong program.
//
// ## Owned is never this file's word
//
// D5: *promotion to `Owned` is never automatic*. A `.to_owned()` hands back a
// `String`, which is not a view at all — so no **view** position is ever Owned,
// and the lattice this file solves over is the two states below it. The top of
// the lattice exists for the program to reach, not for the analysis to assign.
//
// ## Which way it errs
//
// **Towards Tethered**, which is D7's own polarity for the case it names: a
// state barrier *widens* to Tethered, *"the widest representation, never to
// Owned"*. A position this file cannot decide is therefore Tethered, and the
// cost of being wrong that way is a representation wider than it had to be —
// never a program that does the wrong thing. The other direction would tell a
// later reader that a value borrows when it escapes, which is the use-after-free
// the state exists to prevent.
//
// ## What is not here, and why
//
// **D7's state barrier is not applied.** *Crossing a `dyn` boundary or a
// published non-generic API is a state barrier* is a consequence of there being
// three representations and one ABI to choose; with one representation there is
// no barrier to cross. It belongs with the layouts rather than with the
// analysis, and marking every `pub` signature Tethered today would fill the
// column with a state nothing has.
//
// **The buffer table is not here either** (D4). Its shape is a fact about a
// container's representation, and this file writes no representation.

// **The pass is written in Nikaia** (`compiler/tether.nika`, #125): the
// states one function's views solve to, the whole-package entry
// (`tether_infer`) and the per-function one (`tether_of_function`,
// ADR-100 D7), whether a parse's result may point into its input, and whether
// an expression makes a buffer of its own. What stays here is handing each
// unit's view over from the Rust `Parsed`, the `--tethers` report and the
// refusal, both of which read the keep plan.

pub use nikaia_std::compiler::tether::Buffer;
pub use nikaia_std::compiler::ty::{Held, State};

use crate::contracts::LedgerOps;
use crate::parser::Parsed;

use super::Ledger;

/// The position a function's result occupies in the column.
///
/// The same spelling `sharing` uses for the same thing, because they are the
/// same position read by two analyses.
pub const RESULT: &str = "<result>";

/// Give every entry of this package the states its views solved to (D7):
/// `tether.nika`'s `tether_infer`, over each unit's view. Then which positions
/// really leave, over the whole package, to a fixpoint (`keep::infer`,
/// [ADR-tether-v1](../../../../docs/specification/adr/adr-tether-v1.md) D6).
pub fn infer(ledger: &mut Ledger, units: &[&Parsed], library: &Ledger) {
    let views: Vec<_> = units
        .iter()
        .map(|parsed| crate::check::parsed_view_of(parsed))
        .collect();
    nikaia_std::compiler::tether::tether_infer(ledger, &views, library);
    super::keep::infer(ledger, units, library);
}

/// Whether this expression hands back a buffer of its own
/// (`tools/tether.nika`).
pub(crate) fn makes_a_buffer(
    parsed: &Parsed,
    expr: &crate::ast::Expr,
    own: &Ledger,
    library: &Ledger,
) -> Buffer {
    nikaia_std::compiler::tether::makes_a_buffer(
        expr,
        &nikaia_std::compiler::views::Asked {
            names: &parsed.interner,
            own,
            library,
        },
        &mut |name: &str| parsed.unaliased(name),
    )
}

/// `--tethers`: what the analysis solved, per function
/// ([ADR-tether-v1](../../../../docs/specification/adr/adr-tether-v1.md) D4).
///
/// *The inverse tool is inspection, not assertion*, and it is the half of D6
/// that survived: the assertion beside it is gone
/// ([ADR-tether-v1](../../../../docs/specification/adr/adr-tether-v1.md) D4) and this shows
/// what was chosen without being asked. Read off the ledger the build produced
/// rather than computed again here, so that what a person reads is what the file
/// records.
pub fn report(parsed: &Parsed, ledger: &Ledger) -> String {
    // **The words are Nikaia** (`tools/tether.nika`, ADR-294, #125). This
    // file's own entries, because a report is about one file and the ledger is
    // the package's: a key names a function of this unit exactly when this
    // unit's own inference produced it.
    let here: std::collections::BTreeSet<String> =
        Ledger::infer(parsed).functions.keys().cloned().collect();
    let library = crate::contracts::std_ledger();
    let plans = super::keep::buffer_plans(parsed, None, ledger, library);
    nikaia_std::compiler::tether::tethers_report(&parsed.text_tiers, ledger, &here, &plans, library)
}

// ---------------------------------------------------------------------------
// The refusal: where the tether would be needed and is not built
// ---------------------------------------------------------------------------

/// A view handed back that points into a buffer the body **owns** (`NK2303`).
///
/// This is the one shape Part I 6.6's `Tethered` exists for, and
/// [ADR-tether-v1](../../../../docs/specification/adr/adr-tether-v1.md) D9 is what to do
/// about it while the state is not built: refuse on the Nikaia line, naming the
/// buffer and the mechanism, rather than lower a function whose result outlives
/// the buffer it points into and let `rustc` speak about the generated file
/// ([Part III C.1](../../../docs/specification/30-nikaia-tooling.md)).
///
/// **It stands on a named buffer and never on a doubt.** [`Buffer::Unknown`] is
/// a call no ledger describes; the *column* errs towards Tethered for it,
/// because a wide state costs a wide representation. A refusal cannot err that
/// way — refusing a correct program is the worse of the two mistakes
/// ([Part III C.4](../../../docs/specification/30-nikaia-tooling.md)) — so this
/// walk asks for the buffer **by name** and for the returned expression to be
/// derived from it. Where either is missing the program is lowered exactly as
/// it was before this check existed.
pub fn check(parsed: &Parsed, own: &Ledger, library: &Ledger) -> Vec<crate::check::Finding> {
    // **Where a view outlives its buffer is the keep plan's answer**
    // ([ADR-tether-v1](../../../../docs/specification/adr/adr-tether-v1.md)): it follows
    // views through `push`, `for`, fields and calls, which the walk that stood
    // here did not, and it refuses only what it cannot lower or what no
    // declaration permits.
    refusals_of(super::keep::buffer_plans(parsed, None, own, library))
}

/// [`check`]'s refusals, of plans already made.
pub fn refusals_of(
    plans: Vec<nikaia_std::compiler::buffers::BufferPlan>,
) -> Vec<crate::check::Finding> {
    plans
        .into_iter()
        .map(super::keep::from_nikaia)
        .flat_map(|plan| plan.refusals)
        .collect()
}
