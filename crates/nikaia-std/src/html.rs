//! `std::html` - the escaping a template's contract rests on.
//!
//! [ADR-017](../../../docs/specification/adr/adr-017.md) makes escaping a
//! property of the template grammar rather than of the caller's discipline: a
//! hole is escaped, every hole, every time, and the one way to say "this is
//! already markup" is a type. Both halves need the same function, and it lives
//! here rather than inside the code generator so that there is one
//! implementation, testable on its own, and a hand-written renderer gets
//! exactly what a template gets.

use std::borrow::Cow;
use std::fmt;

/// `html::is_plain(part)`: its Nikaia half, `runtime/raw_sinks.nika`, the
/// precondition of `html::Raw` (ADR-raw-sinks-1 D2, #1253).
pub use crate::runtime::is_plain;

/// Text that is already markup, and may be placed in a template without being
/// escaped.
///
/// [ADR-017](../../../docs/specification/adr/adr-017.md) D2: the one way to say
/// "this is already markup" is a **type**, not a flag at the hole. A flag is a
/// property of the call site and the same value reaches many call sites; a type
/// travels with the value and is decided where the value is built.
///
/// `new` is the only constructor, so "every place this program declared
/// something already-escaped" is one grep rather than a type search. There is
/// deliberately no `From<String>`, no `Deref<Target = str>` that would let it
/// pass as text by accident, and no way back out that hides where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raw(String);

impl Raw {
    /// Promise that `markup` is already HTML.
    ///
    /// The one line to grep for, and the one to review: everything downstream
    /// of it trusts this call.
    pub fn new(markup: impl Into<String>) -> Self {
        Raw(markup.into())
    }

    /// The markup, for whoever writes it out.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `text`, escaped - the safe way to build markup out of something that is
    /// not markup yet.
    pub fn escaped(text: &str) -> Self {
        Raw(escape(text).into_owned())
    }
}

impl fmt::Display for Raw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The five characters that change what HTML *means* in text and in a quoted
/// attribute value.
///
/// `&` first, and that is not a style choice: escaping it after the others
/// would escape the ampersands the others just produced.
const ESCAPES: [(char, &str); 5] = [
    ('&', "&amp;"),
    ('<', "&lt;"),
    ('>', "&gt;"),
    ('"', "&quot;"),
    ('\'', "&#39;"),
];

/// The five, as a bit per byte value.
///
/// Built from `ESCAPES` rather than written out, so the characters are declared
/// once and this cannot drift from them. Every one of them is below 64 - `"` is
/// 34, `&` is 38, `'` is 39, `<` is 60, `>` is 62 - so the whole question fits
/// in **one machine word** and asking it touches no memory at all.
///
/// That is the second thing measured here and it is the one that mattered: a
/// 256-entry table of `&str` was **16 % slower** than the five compares it
/// replaced, because a fat pointer per byte is 4 KB of cache to walk where a
/// mask is a register (ADR-178 §1).
///
/// **A byte scan is sound because all five are ASCII.** A byte of a multi-byte
/// UTF-8 character is always `0x80` or above, so it can never be one of these -
/// which is what lets the scan work on bytes and copy whole runs, rather than
/// decode every character to compare it with five others.
const NEEDS_ESCAPE: u64 = {
    let mut mask = 0u64;
    let mut i = 0;
    while i < ESCAPES.len() {
        let from = ESCAPES[i].0 as u32;
        assert!(from < 64, "an escape must fit the mask");
        mask |= 1 << from;
        i += 1;
    }
    mask
};

/// Whether this byte is one of the five. No memory is touched.
#[inline]
fn needs_escape(byte: u8) -> bool {
    byte < 64 && NEEDS_ESCAPE & (1 << byte) != 0
}

/// What it becomes. Only reached on a hit, which is the rare case.
#[inline]
fn replacement(byte: u8) -> &'static str {
    match byte {
        b'&' => "&amp;",
        b'<' => "&lt;",
        b'>' => "&gt;",
        b'"' => "&quot;",
        b'\'' => "&#39;",
        _ => unreachable!("only a byte the mask matched reaches here"),
    }
}

/// `text`, safe to place in an HTML text node or a quoted attribute value.
///
/// Returns the input unchanged - and unallocated - when nothing needs
/// escaping, which is the common case for the column of a database table. The
/// scan is one pass either way.
///
/// **65 % cheaper than the version this replaced**, measured under callgrind on
/// 40 000 holes (`crates/nikaia/tests/measure.rs`,
/// `docs/history/staging-candidates.md` §3.2). Two of the three things tried made it
/// *worse*, which is why both are written down where they were tried.
///
/// It does **not** make text safe for every position: a `<script>` body, a CSS
/// block, an unquoted attribute and a URL each need something else, which is
/// why ADR-017 D3 makes a hole in those positions a compile error rather than a
/// call to this function.
pub fn escape(text: &str) -> Cow<'_, str> {
    let bytes = text.as_bytes();
    let Some(first) = bytes.iter().position(|&b| needs_escape(b)) else {
        return Cow::Borrowed(text);
    };

    // Room for the prefix that needs nothing plus a little: a message with one
    // apostrophe is the ordinary case, not a message that is all apostrophes.
    let mut escaped = String::with_capacity(text.len() + 16);
    escaped.push_str(&text[..first]);

    // One character at a time, and that was **measured** rather than assumed.
    // Copying the runs between two escapes with `push_str` - the shape that
    // looks obviously better, and that this function was first written with -
    // costs 12 % more here: the index arithmetic and the bounds check on each
    // slice outweigh what the copies save on text this size
    // (`docs/history/staging-candidates.md` §3).
    for c in text[first..].chars() {
        // `c as u32` first: a character above the mask's range is never one of
        // the five, and this is the ordinary case.
        let byte = c as u32;
        if byte < 64 && needs_escape(byte as u8) {
            escaped.push_str(replacement(byte as u8));
        } else {
            escaped.push(c);
        }
    }

    Cow::Owned(escaped)
}

/// What may be placed in a template hole, and how it is written.
///
/// [ADR-017](../../../docs/specification/adr/adr-017.md) D2 says the one way to
/// say "this is already markup" is a **type**. This is that decision expressed
/// where the compiler can act on it: a [`Raw`] renders itself, text renders
/// escaped, and a type with no impl cannot be put in a template at all.
///
/// The template compiler emits a call to this for every hole, unconditionally.
/// There is no flag at the hole and no way to ask for the other behaviour -
/// choosing is what the type does.
pub trait Render {
    /// The markup this value becomes.
    fn render(&self) -> Cow<'_, str>;
}

impl Render for Raw {
    /// Already markup, by the promise `Raw::new` made.
    fn render(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.as_str())
    }
}

impl Render for str {
    fn render(&self) -> Cow<'_, str> {
        escape(self)
    }
}

impl Render for String {
    fn render(&self) -> Cow<'_, str> {
        escape(self)
    }
}

impl<T: Render + ?Sized> Render for &T {
    fn render(&self) -> Cow<'_, str> {
        (**self).render()
    }
}

/// Numbers and `bool` write themselves: their `Display` cannot produce a
/// character that changes what HTML means, so escaping them would be a scan
/// over text that can never contain anything to escape.
macro_rules! render_by_display {
    ($($t:ty),*) => {$(
        impl Render for $t {
            fn render(&self) -> Cow<'_, str> {
                Cow::Owned(self.to_string())
            }
        }
    )*};
}

render_by_display!(
    bool, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

#[cfg(test)]
mod tests {
    use super::{Raw, escape};
    use std::borrow::Cow;

    // The escapes, a template's holes and `html::Raw` written out are
    // `tests/compiler/tests/html_text_is_escaped_and_markup_is_not.nika`
    // (#2082). What stays here no program sees: that text needing nothing is
    // not copied, and `Raw::escaped`, which the contracts do not name.

    #[test]
    fn text_that_needs_nothing_is_returned_unallocated() {
        assert!(matches!(
            escape("Additional fortune added at request time."),
            Cow::Borrowed(_)
        ));
        assert!(matches!(escape("日本語"), Cow::Borrowed(_)));
    }

    /// The safe way in: escape first, and the result is markup by construction.
    #[test]
    fn escaped_text_becomes_markup() {
        let markup = Raw::escaped("a < b & c");
        assert_eq!(markup.as_str(), "a &lt; b &amp; c");

        // …and it is not escaped twice, because it is no longer text.
        assert_eq!(Raw::new(markup.as_str()).as_str(), "a &lt; b &amp; c");
    }
}
