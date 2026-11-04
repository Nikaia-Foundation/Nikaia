// crates/nikaia/src/bounds_report.rs
//
// `--bounds`: every index into a list and every `+`, `-` and `*` the
// optimizations of ADR-306 may drop the check of, and what became of each
// (#389).
//
// An author could see what `remove-bounds-checks` and `remove-overflow-checks`
// proved only by reading the lowered Rust for `proven::read` and
// `<T>::wrapping_*`. `--asserts` already answers the same question for written
// claims (ADR-269 D7); this is that report for the checks the compiler writes
// itself. It explains a decision and changes none, as `--trust` and `--tethers`
// do.
//
// **The reason a check stayed is the shape that stopped the walk, read off the
// site**, not a trace of the solver: a field, a list of lists, a product of two
// unknowns, a call's result, or none of those - a value the walk has no bound
// for, which is a counter, a loop or a length. Those are `solver-workload.md`
// §8.4's categories, each with the issue that would supply the fact. It names
// where the next fact pays off; it does not prove that the fact would be
// enough.

use crate::bounds::{BoundsChecks, OverflowChecks};
use crate::contracts::Ledger;
use crate::parser::Parsed;
use nikaia_std::tools::bounds_report::{BoundsReportFacts, bounds_report_checked};

/// The report, for one file. What is proved does not depend on what the
/// build drops (ADR-306 D14), only on `through_loops`
/// (`prove-through-loops:on`, ADR-361 D10); the first line says whether this
/// build writes it without its check.
#[allow(clippy::too_many_arguments)]
pub fn report(
    parsed: &Parsed,
    source: &str,
    path: &str,
    own: &Ledger,
    library: &Ledger,
    bounds: BoundsChecks,
    overflow: OverflowChecks,
    through_loops: bool,
) -> String {
    let checked = crate::check::check(parsed, own, library);
    report_checked(
        parsed,
        source,
        path,
        &checked,
        own,
        library,
        bounds,
        overflow,
        through_loops,
    )
}

/// The same over a check the caller ran: `lower-std --bounds` checks each file
/// of the tools package with the others beside it (#1053), as `lower-std`
/// does, where [`report`] checks the file alone.
#[allow(clippy::too_many_arguments)]
pub fn report_checked(
    parsed: &Parsed,
    source: &str,
    path: &str,
    checked: &crate::check::Checked,
    own: &Ledger,
    library: &Ledger,
    bounds: BoundsChecks,
    overflow: OverflowChecks,
    through_loops: bool,
) -> String {
    let published = crate::prove::with_published(own, &checked.facts.published);
    let proven = crate::bounds::proven(
        parsed,
        &crate::bounds::Sites::of(&checked.facts),
        &[&published, library],
        &std::collections::BTreeSet::new(),
        through_loops,
    );
    let facts = BoundsReportFacts {
        lists: checked.facts.list_indices.clone(),
        arithmetic: checked.facts.arithmetic.clone(),
        negations: checked.facts.negations.clone(),
        proven_indices: proven.indices.iter().copied().collect(),
        proven_arithmetic: proven.arithmetic.iter().copied().collect(),
        proven_negations: proven.negations.iter().copied().collect(),
    };
    bounds_report_checked(
        &parsed.program,
        &parsed.interner,
        source,
        path,
        &facts,
        &bounds,
        &overflow,
        &|e: &crate::ast::Expr| crate::emit::literal_expressions(parsed, e),
    )
}
