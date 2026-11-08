// crates/nikaia-std/src/concat.rs
//
// `a + b` where one side is text.
//
// **Rust's own rule reached the user unchanged**, and only one of the four
// shapes a program can write compiled:
//
// ```text
// "a" + "b"  →  error[E0369]: cannot add `&str` to `&str`
// "a" + s    →  error[E0369]: cannot add `String` to `&str`
// s + "b"    →  compiles
// s + s2     →  error[E0308]: mismatched types
// ```
//
// — about a file nobody wrote, which is Part III C.1's class. This language's
// `+` says nothing about which side is owned and should not have to.
//
// **A trait, and the reason is the one `index::At` gives.** This emitter does
// not know types (ADR-296 D17), so it writes one call and the language below
// picks. What that buys here is not merely *working*: it is that each shape
// keeps the lowering that suits it. Measured over three million
// concatenations, best of three — `String + &str` as it lowers today, 42 ms;
// `format!("{}{}")` for every shape, 250 ms; through this trait, **42 ms**. So
// the rule *is* the table of four cases rather than a replacement for it, and
// the obvious single rule would have been a sixfold regression on the one form
// that already worked.
//
// **Only text comes through here**, and that is a hard line rather than a
// scope choice. A number's `+` may not leave this language's own crates:
// [ADR-285 D5 turns `overflow-checks` on **per Nikaia crate** while the
// profile turns them off, because a foreign crate's hash function wraps on
// purpose — and this crate is on the foreign side of that split. Measured with
// the same profile shape: `a + b` in a checked crate aborts, and the same
// `a + b` through an `#[inline]` helper in an unchecked one wraps silently to
// `-9223372036854775808`. Inlining does not carry the check across; it is
// decided where the code is **written**. So the checker says which `+` is a
// concatenation and every other one stays exactly where it is
// ([ADR-081](../../../docs/specification/adr/adr-081.md) D2).
//
// [ADR-285]: ../../../docs/specification/adr/adr-285.md

/// One shape of `a + b` over text, and what it comes to.
///
/// `Out` is always `String`: a concatenation makes a new value, and there is no
/// borrow to hand back.
pub trait Plus<Rhs> {
    /// `String`, or a block's [`Text`](crate::block::Text) where a side is
    /// one (#490).
    type Out;
    fn plus(self, rhs: Rhs) -> Self::Out;
}

/// The shape that already worked, and it keeps what it had: the left side's
/// buffer is written into rather than replaced. This is what `String + &str`
/// lowers to today and what the measurement above says costs nothing.
impl Plus<&str> for String {
    type Out = String;
    #[inline]
    fn plus(mut self, rhs: &str) -> String {
        self.push_str(rhs);
        self
    }
}

/// The same, with the right side owned and dropped after its bytes are taken.
impl Plus<String> for String {
    type Out = String;
    #[inline]
    fn plus(mut self, rhs: String) -> String {
        self.push_str(&rhs);
        self
    }
}

/// Neither side owns anything, so something has to be allocated — once, at the
/// size the result needs, rather than twice by growing.
impl Plus<&str> for &str {
    type Out = String;
    #[inline]
    fn plus(self, rhs: &str) -> String {
        let mut out = String::with_capacity(self.len() + rhs.len());
        out.push_str(self);
        out.push_str(rhs);
        out
    }
}

/// **The right side's buffer is reused**, which is the shape Rust has no
/// operator for at all. `insert_str` moves the existing bytes up rather than
/// allocating a second time, so this costs a copy where the obvious lowering
/// would cost an allocation and a copy.
impl Plus<String> for &str {
    type Out = String;
    #[inline]
    fn plus(self, mut rhs: String) -> String {
        rhs.insert_str(0, self);
        rhs
    }
}

/// **A side lent out of a list** (#1268): the element of a `for` over a
/// `Vec[String]` is a `&String` below, and the emitter, which does not know
/// types, writes it as it is. Each shape reads it as the `&str` it holds, so
/// the owned side keeps the buffer reuse above.
impl Plus<&String> for String {
    type Out = String;
    #[inline]
    fn plus(self, rhs: &String) -> String {
        self.plus(rhs.as_str())
    }
}

/// The same, with the left side borrowed: one allocation, at the size needed.
impl Plus<&String> for &str {
    type Out = String;
    #[inline]
    fn plus(self, rhs: &String) -> String {
        self.plus(rhs.as_str())
    }
}

/// A lent `&String` on the left is borrowed text, as a `&str` is.
impl Plus<&str> for &String {
    type Out = String;
    #[inline]
    fn plus(self, rhs: &str) -> String {
        self.as_str().plus(rhs)
    }
}

/// The right side's buffer is reused, as for `&str + String`.
impl Plus<String> for &String {
    type Out = String;
    #[inline]
    fn plus(self, rhs: String) -> String {
        self.as_str().plus(rhs)
    }
}

/// Both sides lent.
impl Plus<&String> for &String {
    type Out = String;
    #[inline]
    fn plus(self, rhs: &String) -> String {
        self.as_str().plus(rhs.as_str())
    }
}

/// **A side that is a block's text makes the sum one** (ADR-327 D4, #490):
/// an owned left side keeps its buffer, as `String`'s does; otherwise one
/// allocation at the size the sum needs, outside every block (D9).
macro_rules! text_plus {
    ($($left:ty, $right:ty;)+) => {$(
        impl Plus<$right> for $left {
            type Out = crate::block::Text;
            #[inline]
            fn plus(self, rhs: $right) -> crate::block::Text {
                let (left, right): (&str, &str) = (&self, &rhs);
                let mut out = crate::block::Text::with_capacity(left.len() + right.len());
                out.push_str(left);
                out.push_str(right);
                out
            }
        }
    )+};
}

text_plus! {
    &str, crate::block::Text;
    &str, &crate::block::Text;
    &crate::block::Text, &str;
    &crate::block::Text, crate::block::Text;
    &crate::block::Text, &crate::block::Text;
}

impl Plus<&str> for crate::block::Text {
    type Out = crate::block::Text;
    #[inline]
    fn plus(mut self, rhs: &str) -> crate::block::Text {
        self.push_str(rhs);
        self
    }
}

impl Plus<crate::block::Text> for crate::block::Text {
    type Out = crate::block::Text;
    #[inline]
    fn plus(mut self, rhs: crate::block::Text) -> crate::block::Text {
        self.push_str(&rhs);
        self
    }
}

impl Plus<&crate::block::Text> for crate::block::Text {
    type Out = crate::block::Text;
    #[inline]
    fn plus(mut self, rhs: &crate::block::Text) -> crate::block::Text {
        self.push_str(rhs);
        self
    }
}

/// The emitted spelling: `nikaia_std::concat::plus(a, b)`.
#[inline]
pub fn plus<L: Plus<R>, R>(left: L, right: R) -> L::Out {
    left.plus(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All four shapes, which is the whole point: the language below has an
    /// operator for exactly one of them.
    #[test]
    fn every_shape_concatenates() {
        let owned = || "left-".to_string();
        assert_eq!(plus(owned(), "right"), "left-right");
        assert_eq!(plus(owned(), "right".to_string()), "left-right");
        assert_eq!(plus("left-", "right"), "left-right");
        assert_eq!(plus("left-", "right".to_string()), "left-right");
    }

    /// The left side's buffer is kept where it is big enough - the property the
    /// measurement in this file's head is about, held to rather than described.
    #[test]
    fn an_owned_left_side_keeps_its_buffer() {
        let mut roomy = String::with_capacity(64);
        roomy.push_str("left-");
        let before = roomy.as_ptr();
        let joined = plus(roomy, "right");
        assert_eq!(joined.as_ptr(), before, "no reallocation, so no copy");
    }

    /// And the right side's is, where the left is borrowed.
    #[test]
    fn a_borrowed_left_side_reuses_the_right() {
        let mut roomy = String::with_capacity(64);
        roomy.push_str("right");
        let before = roomy.as_ptr();
        let joined = plus("left-", roomy);
        assert_eq!(joined, "left-right");
        assert_eq!(
            joined.as_ptr(),
            before,
            "the bytes moved, the buffer did not"
        );
    }
}
