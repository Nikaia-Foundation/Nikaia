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

#[cold]
#[inline(never)]
#[track_caller]
fn between_backwards(start: usize, end: usize) -> ! {
    panic!("`between` was handed an end (byte {end}) before its start (byte {start})")
}
