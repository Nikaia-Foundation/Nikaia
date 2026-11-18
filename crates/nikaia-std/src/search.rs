//! **`s.find(needle; from:)`** ([ADR-320](../../../docs/specification/adr/adr-320.md)
//! D6): the byte position of the first match at or after `from`, or `None`.
//!
//! The emitter writes `s.find(…)` on text as a call here, because the
//! language below's own `str::find` has another shape - one argument and a
//! `usize` - and an inherent method wins over anything a library adds.

/// **What a text is searched for** (#877): a text, or one `char`, as the
/// language below's own `contains` and `split` take either. A one-byte text
/// is searched as the `char` it is, which is `memchr`; the language below
/// builds a two-way searcher for any `&str`, one byte or not (#617).
pub trait TextNeedle {
    /// The byte position of the first match in `text`.
    fn first_in(&self, text: &str) -> Option<usize>;
    /// The byte position of the last match in `text`.
    fn last_in(&self, text: &str) -> Option<usize>;
    /// `text` with the first `count` matches replaced by `to`.
    fn replaced_in(&self, text: &str, to: &str, count: usize) -> String;
}

impl TextNeedle for str {
    #[inline]
    fn first_in(&self, text: &str) -> Option<usize> {
        match one_byte(self) {
            Some(byte) => text.find(byte),
            None => text.find(self),
        }
    }
    #[inline]
    fn last_in(&self, text: &str) -> Option<usize> {
        match one_byte(self) {
            Some(byte) => text.rfind(byte),
            None => text.rfind(self),
        }
    }
    #[inline]
    fn replaced_in(&self, text: &str, to: &str, count: usize) -> String {
        match one_byte(self) {
            Some(byte) => text.replacen(byte, to, count),
            None => text.replacen(self, to, count),
        }
    }
}

impl TextNeedle for String {
    #[inline]
    fn first_in(&self, text: &str) -> Option<usize> {
        self.as_str().first_in(text)
    }
    #[inline]
    fn last_in(&self, text: &str) -> Option<usize> {
        self.as_str().last_in(text)
    }
    #[inline]
    fn replaced_in(&self, text: &str, to: &str, count: usize) -> String {
        self.as_str().replaced_in(text, to, count)
    }
}

/// A block's text is searched for as a `String` is (#490).
impl TextNeedle for crate::block::Text {
    #[inline]
    fn first_in(&self, text: &str) -> Option<usize> {
        self.as_str().first_in(text)
    }
    #[inline]
    fn last_in(&self, text: &str) -> Option<usize> {
        self.as_str().last_in(text)
    }
    #[inline]
    fn replaced_in(&self, text: &str, to: &str, count: usize) -> String {
        self.as_str().replaced_in(text, to, count)
    }
}

impl TextNeedle for char {
    #[inline]
    fn first_in(&self, text: &str) -> Option<usize> {
        text.find(*self)
    }
    #[inline]
    fn last_in(&self, text: &str) -> Option<usize> {
        text.rfind(*self)
    }
    #[inline]
    fn replaced_in(&self, text: &str, to: &str, count: usize) -> String {
        text.replacen(*self, to, count)
    }
}

impl<T: TextNeedle + ?Sized> TextNeedle for &T {
    #[inline]
    fn first_in(&self, text: &str) -> Option<usize> {
        (**self).first_in(text)
    }
    #[inline]
    fn last_in(&self, text: &str) -> Option<usize> {
        (**self).last_in(text)
    }
    #[inline]
    fn replaced_in(&self, text: &str, to: &str, count: usize) -> String {
        (**self).replaced_in(text, to, count)
    }
}

/// The first match of `needle` in `text` at or after the byte `from`.
///
/// `from` may be any byte position: UTF-8 is self-synchronising, so a match
/// of a valid needle always starts on a scalar, and a search begun inside one
/// finds the next match. A `from` before the start searches from the start; one
/// past the end finds nothing.
#[inline]
pub fn find<N: TextNeedle + ?Sized>(text: &str, needle: &N, from: i64) -> Option<i64> {
    let mut start = usize::try_from(from.max(0)).ok()?;
    if start > text.len() {
        return None;
    }
    while !text.is_char_boundary(start) {
        start += 1;
    }
    let at = needle.first_in(&text[start..])?;
    i64::try_from(start + at).ok()
}

/// The `char` a one-byte needle is. A byte of valid UTF-8 that is a whole
/// scalar is ASCII, so the two match the same places.
#[inline]
fn one_byte(needle: &str) -> Option<char> {
    match needle.as_bytes() {
        [byte] => Some(char::from(*byte)),
        _ => None,
    }
}

/// `text` cut in two at the byte position `at` (#616): what stands before it
/// and what from it on. A position before the start, past the end or off a
/// scalar's start stops the program, as the language below's `split_at` does.
/// The stop names the program's line, not this one (ADR-300 D10), so it is
/// `#[track_caller]`.
#[inline]
#[track_caller]
pub fn split_at(text: &str, at: i64) -> (&str, &str) {
    let Ok(at) = usize::try_from(at) else {
        panic!("byte index {at} is before the start of the text")
    };
    text.split_at(at)
}

/// The byte position of the last match of `needle` in `text`.
#[inline]
pub fn rfind<N: TextNeedle + ?Sized>(text: &str, needle: &N) -> Option<i64> {
    i64::try_from(needle.last_in(text)?).ok()
}

/// `text` with the first `count` matches of `from` replaced by `to`; a `count`
/// of 0 or less replaces nothing.
#[inline]
pub fn replacen<N: TextNeedle + ?Sized>(
    text: &str,
    from: &N,
    to: &str,
    count: i64,
) -> crate::block::Text {
    crate::block::Text::from(from.replaced_in(text, to, usize::try_from(count).unwrap_or(0)))
}
