// crates/nikaia-std/src/boxed.rs
//
// **A field that holds its own type is in a box the program never names**
// ([ADR-246](../../../docs/specification/adr/adr-246.md)).
//
// The compiler puts a `Box` around each field on a ring of types that hold
// each other inline, and a program reads the field as the type it declared.
// Where a `match` binds such a part, the name is rebound through [`open`] at
// the start of the arm, which takes the box off whichever way the value was
// bound: a `Box<T>` taken by value is a `T`, and a `&Box<T>` lent is a `&T` -
// so the emitter need not know which the `match` did.

/// What a boxed binding becomes once the box is taken off.
pub trait Open {
    type Out;
    fn open(self) -> Self::Out;
}

impl<T> Open for Box<T> {
    type Out = T;
    fn open(self) -> T {
        *self
    }
}

impl<'a, T> Open for &'a Box<T> {
    type Out = &'a T;
    fn open(self) -> &'a T {
        self
    }
}

impl<'a, T> Open for &'a mut Box<T> {
    type Out = &'a mut T;
    fn open(self) -> &'a mut T {
        self
    }
}

/// The emitted spelling: `let a = nikaia_std::boxed::open(a);`.
#[inline]
pub fn open<B: Open>(boxed: B) -> B::Out {
    boxed.open()
}

/// **What a box holds, looked at and not taken** ([ADR-246](../../../docs/specification/adr/adr-246.md)
/// D5): the guard of an arm whose pattern looks inside a boxed part asks
/// whether the part has that shape before the arm opens it. A guard sees a
/// binding by reference, so this takes `&B` and hands back `&T` for a `Box<T>`
/// bound by value and for one bound by reference alike.
pub trait Peek {
    type Inside;
    fn peek(&self) -> &Self::Inside;
}

impl<T> Peek for Box<T> {
    type Inside = T;
    fn peek(&self) -> &T {
        self
    }
}

impl<T> Peek for &Box<T> {
    type Inside = T;
    fn peek(&self) -> &T {
        self
    }
}

impl<T> Peek for &mut Box<T> {
    type Inside = T;
    fn peek(&self) -> &T {
        self
    }
}

#[inline]
pub fn peek<B: Peek>(boxed: &B) -> &B::Inside {
    boxed.peek()
}
