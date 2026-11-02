// crates/nikaia/src/contracts/send.rs
//
// Whether a value may cross a thread (ADR-005 §1 Group B, `NK25xx`).
//
// One question, asked of a **type and a destination** (ADR-312 D6):
//
//     may a value of this type go to *this* destination?
//
// ADR-303 D7 states the first half as "a value may only cross into a foreign
// thread if it may cross any thread", and the reason the answer is a property of
// the type rather than of the particular crossing is ADR-005 Group B: it has to
// be **the same at both settings of `user_parallelism`**, so that a library
// written at one setting cannot turn out to be un-compilable where it is used. A
// check that consulted the switch would answer `yes` for a type whose expansion
// is safe at one setting and not at the other, and `no` for the same type at the
// other - which is exactly the asymmetry Group B, `NK25xx` and ADR-037 §3 were
// all written to prevent. So **no switch reaches this file, and none may** - the
// same sentence `order.rs` opens with, for the same reason.
//
// ## Why the destination is not the switch coming back in
//
// [`Destination`] has two values and **each gets an answer that is the same at
// both settings**, which is the whole of what Group B asks - its own note says
// "'At every setting' is a claim about the verdict, not about the severity". The
// justification may differ per setting; the answer may not. Before ADR-312 the
// lock had to take the worse of the two settings and answer `MayNot`
// everywhere, which made Part II 12.2's counter - the program
// `user_parallelism = yes` exists to serve - impossible to write: at `yes` a real
// operating-system lock stands in the emitted program, under which that counter
// is entirely safe, and it was refused on account of an implementation that does
// not occur in it.
//
// So the destination is the axis the verdict was missing, and not a reading of
// the switch. Nothing below asks what `user_parallelism` is.
//
// ## What ADR-037 D6 changed here, and what it did not
//
// `Shared` was that type, and it was the file's only `MayNot`. D6 removed the
// cause rather than the symptom: `Shared` expands to an atomic count at **both**
// settings, so its expansion no longer moves with the switch and there is
// nothing for the verdict to take the worse of. `Shared` therefore joins
// `CONTAINERS` and is answered by what it holds.
//
// **The rule above is untouched.** The verdict still never reads
// `user_parallelism`, and it is still a property of the type. What changed is
// one row of a table.
//
// **And one row came back** ([ADR-312](../../../../docs/specification/adr/adr-312.md)
// D1): ADR-037 D7's per-value inference made the count's shape a property of
// the *value* again, so there is no one representation for a foreign signature
// to name, and a `Shared` may not go into code nothing describes after all.
// That row was written into [`CHOSEN`] and never reached, because `Shared` is
// in [`CONTAINERS`] too and the container row is answered first. It is reached
// now, after the contents have had their say - see the comment at that fork,
// which is where the order is argued.
//
// ## Three answers and not two
//
// [`Crossing`] has a third arm, and it is the whole design. `Undecided` is not
// `May`: ADR-010 D1 says an analysis that fails open is a vulnerability
// generator, and "nothing is written down about this type" is the absence of an
// answer rather than permission. But it is not a refusal either, because the
// one thing this compiler may never do is reject a program that is correct
// (Part III C.4), and Stage 0 knows the type of rather less than half of what a
// program writes.
//
// What the two non-`May` answers cost therefore depends on **who chose the
// crossing**, and the split is the reason the polarity and the no-false-refusal
// promise can both be kept:
//
//   * A crossing **the compiler chose** - the `task::both` that statement
//     overlapping emits (ADR-292) - is a step the compiler was never obliged to
//     take. Anything but `May` means it does not take it: the statements keep
//     the order they were written in. Nothing is refused and only speed is
//     spent, which is `order.rs`'s own polarity ("every `false` is either a real
//     dependency or an admission of ignorance, and the two are deliberately
//     worth the same").
//   * A crossing **the program wrote** - `spawn`, a value handed to a call this
//     compiler cannot see the end of - is refused on `MayNot` and left to the
//     backend on `Undecided`. A refusal on `Undecided` would reject correct
//     programs; silence is not what it buys instead, because `rustc` still
//     type-checks the emitted crate (ADR-002 §3) and ADR-005 D7's `E0277`
//     translation now reports that refusal against the `.nika` line. So an
//     undecided crossing is never *accepted* here - it is handed on.
//
// ## What the table knows
//
// A closed list, like `touch::KINDS` and for the same reason: a name this file
// does not know is answered `Undecided`, and a name it knows wrongly would be a
// typo that bought a crossing. The list grows when a type needs it, never
// speculatively (ADR-288 D11).
//
// Every name in [`PLAIN`] and [`CONTAINERS`] is safe both to **move** to
// another thread and to be **looked at** from one, which is why a view (`&T`)
// asks the same question as the value. The day a name belongs in one column and
// not the other, this needs a second column; stating that here is cheaper than
// discovering it.
//
// **[`CHOSEN`] is that day, and the column it needed turned out to be the
// destination** (ADR-312 predicted this file would want two columns here). A
// lock at `user_parallelism = no` may be moved to another thread and may not be
// looked at from one, so on the move/look axis it does belong in one column and
// not the other. It never has to be asked that way, because both destinations
// answer before the distinction matters: into our own code the lock may go
// whichever of the two it is doing, and into code nothing describes it may do
// neither. So the two columns stay one, and the table splits by destination
// instead.

use std::collections::BTreeSet;

use crate::ast::Expr;
use crate::parser::Parsed;

use super::{Ledger, ty::Ty};

/// **The verdict is written in Nikaia** (`tools/threads.nika`, #125): what a
/// type is (`Crossing`), why a refusal is one (`Refusal`), where the value is
/// going (`Going`, here `Destination`), and the walk through the ledgers that
/// answers it - plain data, containers, the lock family whose answer is the
/// destination's, and a described type's own word or fields.
pub use nikaia_std::tools::threads::{Crossing, Going as Destination, Refusal};

/// The refused part and the field it was found in, where the answer is a
/// refusal - the two halves `tools/threads.nika` answers one at a time,
/// because a nullable pair is not a type Part I 2.3 writes.
pub trait CrossingOps {
    fn refused(&self) -> Option<(String, Option<String>)>;
}

impl CrossingOps for Crossing {
    fn refused(&self) -> Option<(String, Option<String>)> {
        self.refused_part().map(|part| (part, self.refused_at()))
    }
}

/// Every name a task's body mentions.
///
/// The ordering analysis's walk, asked a second question: it is **total and
/// over-approximate on purpose** (a field access `a.b` contributes `a`, a
/// method name counts as a name, the words of a `dsl` body count), and both
/// properties are what this needs. Over-approximate is the safe direction here
/// too - a name that is not a variable is not in scope, so it is looked up,
/// not found, and says nothing.
pub fn names_used(parsed: &Parsed, body: &Expr) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    super::order::names_in(parsed, body, &mut out);
    out
}

/// The same, for a **statement** — every name any expression in it mentions.
///
/// `names_in` walks an expression; a statement holds several, and the blocks a
/// statement holds are walked too (`tools/names.nika`, #125). Over-approximate in the direction that costs
/// nothing, exactly as [`names_used`] is: a function name and a field name are
/// in the answer, and a caller that looks a name up finds nothing for them.
pub fn names_used_in_stmt(parsed: &Parsed, stmt: &crate::ast::Stmt) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    nikaia_std::tools::names::names_in_statement(stmt, &parsed.interner, &mut out);
    out
}

/// Which of a task body's **own** bindings are held across a pause
/// ([ADR-055](../../../../docs/specification/adr/adr-055.md) §2 D6): bound
/// before a pause, and named after it (`tools/threads.nika`). Everything is a
/// byte position; `named` is the *last* place each name was mentioned.
pub fn held_across_a_pause(
    bound: &[(String, usize)],
    pauses: &[usize],
    named: &std::collections::BTreeMap<String, usize>,
) -> BTreeSet<String> {
    let bound: Vec<(String, i64)> = bound
        .iter()
        .map(|(name, at)| (name.clone(), *at as i64))
        .collect();
    let pauses: Vec<i64> = pauses.iter().map(|at| *at as i64).collect();
    let named: std::collections::BTreeMap<String, i64> = named
        .iter()
        .map(|(name, at)| (name.clone(), *at as i64))
        .collect();
    nikaia_std::tools::threads::held_across_a_pause(&bound, &pauses, &named)
}

/// Whether a value of `ty` may go to `into`.
///
/// `own` is the program's ledger and `library` is `std`'s: between them they
/// hold the fields of every type that is written down (ADR-024), which is what
/// makes this **structural and transitive** as Group B requires - a struct with
/// one lock field is no more crossable than the lock itself.
pub fn crossing(ty: &Ty, own: &Ledger, library: &Ledger, into: Destination) -> Crossing {
    nikaia_std::tools::threads::crossing(
        ty,
        &nikaia_std::tools::threads::Walking { own, library, into },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::LedgerOps;
    use crate::contracts::ty::TyOps;

    // The rest of this module's tests moved to `tools/threads.nika` (ADR-294,
    // #125). These two read `std`'s own ledger, which only Rust parses.
    fn of(text: &str) -> Crossing {
        let own = Ledger::empty();
        let library = Ledger::parse(super::super::STD).expect("std ships a ledger");
        crossing(&Ty::parse(text), &own, &library, Destination::Ours)
    }

    /// `std`'s own opaque types are undecided and not refused: their fields are
    /// Rust, and an entry with no fields is "nothing is written down" rather
    /// than "it holds nothing".
    #[test]
    fn a_library_type_whose_fields_are_rust_is_undecided() {
        assert!(matches!(of("Mapped"), Crossing::Undecided { .. }));
        assert!(matches!(of("Lines"), Crossing::Undecided { .. }));
    }

    /// **Nothing written down answers `MayNot` but a lock and a shared count**,
    /// at either destination: every type either ledger describes is asked.
    #[test]
    fn nothing_but_a_lock_or_a_count_is_refused_at_either_destination() {
        let own = Ledger::empty();
        let library = Ledger::parse(super::super::STD).expect("std ships a ledger");
        for into in [Destination::Ours, Destination::Foreign] {
            for name in own.types.keys().chain(library.types.keys()) {
                let answer = crossing(&Ty::named(name), &own, &library, into);
                assert!(
                    answer.refused().is_none()
                        || ["Locked", "SharedMut", "Shared"]
                            .contains(&crate::contracts::ty::base(name)),
                    "`{name}` into {into:?}: {answer:?}"
                );
            }
        }
    }
}
