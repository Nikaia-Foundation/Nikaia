// A position in a text, `text::Pos`, and the cuts at one
// ([ADR-374](../../../docs/specification/adr/adr-374.md) D1, D3).
//
// Included into `text` (see `lib.rs`), so the items below are `text::Pos` and
// `text::TextPositions`.

/// **A boundary in one text** (ADR-374 D1): between two scalars, or at either
/// end. Below the language it is a byte offset, a machine word; no number
/// becomes one, and it takes no arithmetic. Two positions of one text compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Pos(usize);

impl Pos {
    /// The position at byte `at` of a text, which the caller has found on a
    /// scalar's start or at an end: only `std` makes one.
    #[inline]
    pub(crate) fn at(at: usize) -> Pos {
        Pos(at)
    }
}

/// `s.start()`, `s.end()` and the cuts `before`, `after` and `between`
/// (ADR-374 D1, D3): each cut a view of the text.
pub trait TextPositions {
    /// The position before the first scalar.
    fn start(&self) -> Pos;
    /// The position after the last scalar.
    fn end(&self) -> Pos;
    /// The text up to `p`.
    fn before(&self, p: Pos) -> &str;
    /// The text from `p` on.
    fn after(&self, p: Pos) -> &str;
    /// The text from `p` up to `q`; `q` before `p` stops the program.
    fn between(&self, p: Pos, q: Pos) -> &str;
}

impl TextPositions for str {
    #[inline]
    fn start(&self) -> Pos {
        Pos(0)
    }

    #[inline]
    fn end(&self) -> Pos {
        Pos(self.len())
    }

    // Until the compiler tracks which text a position belongs to (ADR-374
    // D5, #743), a cut keeps Rust's check of both ends; D8 drops it there.
    #[inline]
    fn before(&self, p: Pos) -> &str {
        &self[..p.0]
    }

    #[inline]
    fn after(&self, p: Pos) -> &str {
        &self[p.0..]
    }

    #[inline]
    fn between(&self, p: Pos, q: Pos) -> &str {
        if q.0 < p.0 {
            between_backwards(p.0, q.0)
        }
        &self[p.0..q.0]
    }
}

/// **The first match of `needle` at or after `from`** (ADR-374 D2), a
/// `text::Match` of `text`: its `end` is where a scanner goes on. `from` is a
/// position of `text`; until the compiler tracks that (D5), Rust's cut checks
/// it.
#[inline]
pub fn find<'s, N: crate::search::TextNeedle + ?Sized>(text: &'s str, needle: &N, from: Pos) -> Option<Match<'s>> {
    let at = from.0 + needle.first_in(&text[from.0..])?;
    Some(Match::of(text, at..at + needle.byte_len()))
}

/// **The first match of `needle`** (ADR-374 D2): `find` from the start.
#[inline]
pub fn find_first<'s, N: crate::search::TextNeedle + ?Sized>(text: &'s str, needle: &N) -> Option<Match<'s>> {
    let at = needle.first_in(text)?;
    Some(Match::of(text, at..at + needle.byte_len()))
}

/// **The last match of `needle`** (ADR-374 D2), a `text::Match` of `text`.
#[inline]
pub fn rfind<'s, N: crate::search::TextNeedle + ?Sized>(text: &'s str, needle: &N) -> Option<Match<'s>> {
    let at = needle.last_in(text)?;
    Some(Match::of(text, at..at + needle.byte_len()))
}

/// **`text` cut in two at `at`** (ADR-374 D3): `before(at)` and `after(at)`
/// at once, each a view of `text`.
#[inline]
pub fn split_at(text: &str, at: Pos) -> (&str, &str) {
    text.split_at(at.0)
}

#[cold]
#[inline(never)]
#[track_caller]
fn between_backwards(start: usize, end: usize) -> ! {
    panic!("`between` was handed an end (byte {end}) before its start (byte {start})")
}

/// **A text and a position in it** (ADR-374 D6): what a scanner that keeps its
/// place holds, since a position is no field. `text::Cursor(t)` stands at
/// `t.start()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor<'s> {
    text: &'s str,
    at: usize,
}

/// `text::Cursor(t)`: a cursor at the start of `t`.
#[allow(non_snake_case)]
#[inline]
pub fn Cursor(text: &str) -> Cursor<'_> {
    Cursor { text, at: 0 }
}

impl<'s> Cursor<'s> {
    /// The position the cursor stands at.
    #[inline]
    pub fn at(&self) -> Pos {
        Pos(self.at)
    }

    /// Stands at `p`, a position of the same text.
    #[inline]
    pub fn go(&mut self, p: Pos) {
        // Until the compiler tracks which text a position belongs to (D5),
        // a position off this text's boundaries stops the program here.
        if !self.text.is_char_boundary(p.0) {
            go_off_the_text(p.0, self.text.len())
        }
        self.at = p.0;
    }

    /// The text from the cursor to the next `needle`, the cursor moved past
    /// it; `None`, the cursor where it was, where no `needle` follows.
    #[inline]
    pub fn until<N: crate::search::TextNeedle>(&mut self, needle: N) -> Option<&'s str> {
        let rest = &self.text[self.at..];
        let found = needle.first_in(rest)?;
        let piece = &rest[..found];
        self.at += found + needle.byte_len();
        Some(piece)
    }

    /// The text from the cursor on.
    #[inline]
    pub fn rest(&self) -> &'s str {
        &self.text[self.at..]
    }

    /// The whole text.
    #[inline]
    pub fn text(&self) -> &'s str {
        self.text
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn go_off_the_text(at: usize, len: usize) -> ! {
    panic!("byte position {at} is no position of the cursor's text, which has {len} bytes")
}
