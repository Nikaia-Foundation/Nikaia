// crates/nikaia-std/src/text_float.rs
//
// `text::parse_f64`, the one entry of `std::text` written in Rust: the shape
// of the text is checked by `runtime/text_float.nika`, against ADR-285 D35's
// rule, and the number it writes is read by Rust's `str::parse`, whose
// conversion rounds correctly (the nearest `f64`, ties to even). A conversion
// that rounds right is a library of its own, and this one is already in every
// program's core.

/// The number this text writes as an `f64`, or nothing where it writes none.
///
/// An optional `-` or `+`, decimal digits, an optional `.` followed by digits,
/// and an optional exponent: `e` or `E`, an optional sign, and digits. Or one
/// of `inf` and `nan` after the optional sign, in any case (`NaN`, `-Inf`).
/// Nothing else: no space, no `_`, no `.5`, no `1.`, no `infinity`.
pub fn parse_f64(text: &str) -> Option<f64> {
    if crate::runtime::writes_float(text) {
        text.parse().ok()
    } else {
        None
    }
}
