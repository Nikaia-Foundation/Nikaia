//! **`s.find(needle; from:)`** ([ADR-320](../../../docs/specification/adr/adr-320.md)
//! D6): the byte position of the first match at or after `from`, or `None`.
//!
//! The emitter writes `s.find(…)` on text as a call here, because the
//! language below's own `str::find` has another shape - one argument and a
//! `usize` - and an inherent method wins over anything a library adds.

/// The first match of `needle` in `text` at or after the byte `from`.
///
/// `from` may be any byte position: UTF-8 is self-synchronising, so a match
/// of a valid needle always starts on a scalar, and a search begun inside one
/// finds the next match. A `from` before the start searches from the start; one
/// past the end finds nothing. A one-byte needle is searched as the `char` it
/// is, which is `memchr`; the language below builds a two-way searcher for any
/// `&str`, one byte or not (#617).
#[inline]
pub fn find(text: &str, needle: &str, from: i64) -> Option<i64> {
    let mut start = usize::try_from(from.max(0)).ok()?;
    if start > text.len() {
        return None;
    }
    while !text.is_char_boundary(start) {
        start += 1;
    }
    let rest = &text[start..];
    let at = match one_byte(needle) {
        Some(byte) => rest.find(byte)?,
        None => rest.find(needle)?,
    };
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
#[inline]
pub fn split_at(text: &str, at: i64) -> (&str, &str) {
    let Ok(at) = usize::try_from(at) else {
        panic!("byte index {at} is before the start of the text")
    };
    text.split_at(at)
}

/// The byte position of the last match of `needle` in `text`.
#[inline]
pub fn rfind(text: &str, needle: &str) -> Option<i64> {
    let at = match one_byte(needle) {
        Some(byte) => text.rfind(byte)?,
        None => text.rfind(needle)?,
    };
    i64::try_from(at).ok()
}

/// `text` with the first `count` matches of `from` replaced by `to`; a `count`
/// of 0 or less replaces nothing.
#[inline]
pub fn replacen(text: &str, from: &str, to: &str, count: i64) -> String {
    let count = usize::try_from(count).unwrap_or(0);
    match one_byte(from) {
        Some(byte) => text.replacen(byte, to, count),
        None => text.replacen(from, to, count),
    }
}

#[cfg(test)]
mod tests {
    use super::{find, replacen, rfind, split_at};

    #[test]
    fn a_cut_at_a_position_in_i64() {
        assert_eq!(split_at("Hamburg;12.0", 7), ("Hamburg", ";12.0"));
        assert_eq!(split_at("abc", 0), ("", "abc"));
        assert_eq!(split_at("abc", 3), ("abc", ""));
    }

    #[test]
    #[should_panic(expected = "before the start")]
    fn a_cut_before_the_start_stops() {
        let _ = split_at("abc", -1);
    }

    #[test]
    fn a_one_byte_needle_finds_what_the_text_needle_finds() {
        assert_eq!(rfind("a;b;c", ";"), Some(3));
        assert_eq!(replacen("1.2.3", ".", "", 1), "12.3");
        assert_eq!(find("äa;", ";", 0), Some(3));
    }

    #[test]
    fn the_last_match_and_a_counted_replace() {
        assert_eq!(rfind("a::b::c", "::"), Some(4));
        assert_eq!(rfind("abc", "x"), None);
        assert_eq!(replacen("aaa", "a", "b", 2), "bba");
        assert_eq!(replacen("aaa", "a", "b", -1), "aaa");
    }

    #[test]
    fn a_match_at_or_after_the_position() {
        assert_eq!(find("Hamburg;12.0;x", ";", 0), Some(7));
        assert_eq!(find("Hamburg;12.0;x", ";", 8), Some(12));
        assert_eq!(find("Hamburg;12.0;x", ";", 13), None);
        assert_eq!(find("abc", "", 3), Some(3));
        assert_eq!(find("abc", "x", 4), None);
        assert_eq!(find("abc", "a", -5), Some(0));
    }

    #[test]
    fn a_search_begun_inside_a_scalar_finds_the_next_match() {
        // `ä` is two bytes; byte 1 is inside it.
        assert_eq!(find("äa;ä;", ";", 1), Some(3));
        assert_eq!(find("äöü", "ü", 1), Some(4));
    }
}
