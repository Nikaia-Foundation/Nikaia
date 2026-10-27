// crates/nikaia-std/src/arm.rs
//
// **A choice lent whole whose arms are views and values made there** (#894,
// ADR-282 D13). `look(if c { t } else { P { .. } })`, where `look` only reads
// its argument: the arm `t` is a view, the other arm makes a value, and a
// reference to that value dies with the arm. The emitter puts every arm into
// one [`Arm`] instead and lends that, `look(&*if c { Arm::View(t) } else {
// Arm::Own(P { .. }) })`: the `Arm` is a value of the argument, so it lives
// through the call. Text takes `EitherText` for the same (`either_text`).

use std::borrow::Borrow;
use std::ops::Deref;

/// One arm's value: a view, or a value the arm made.
pub enum Arm<'a, B: ?Sized, O> {
    View(&'a B),
    Own(O),
}

impl<B: ?Sized, O: Borrow<B>> Deref for Arm<'_, B, O> {
    type Target = B;
    fn deref(&self) -> &B {
        match self {
            Arm::View(view) => view,
            Arm::Own(own) => own.borrow(),
        }
    }
}
