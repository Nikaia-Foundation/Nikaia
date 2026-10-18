//! A text's graphemes: what a reader sees as one character each
//! ([ADR-320](../../../docs/specification/adr/adr-320.md) D9).
//!
//! `s.graphemes` lowers to `s.graphemes()`, this file's method, so the
//! segmentation is the standard library's and moves with it: Unicode's
//! extended grapheme clusters (UAX #29) as `unicode-segmentation` 1.13.3
//! implements them, which is **Unicode 17.0.0** ([`UNICODE_VERSION`]).

/// The Unicode version the segmentation follows, `(major, minor, update)`.
pub use unicode_segmentation::UNICODE_VERSION;

/// The graphemes of a text, each a `&str` into it, in order.
pub trait TextGraphemes {
    /// The extended grapheme clusters, borrowed from the text.
    fn graphemes(&self) -> unicode_segmentation::Graphemes<'_>;
}

impl TextGraphemes for str {
    fn graphemes(&self) -> unicode_segmentation::Graphemes<'_> {
        unicode_segmentation::UnicodeSegmentation::graphemes(self, true)
    }
}
