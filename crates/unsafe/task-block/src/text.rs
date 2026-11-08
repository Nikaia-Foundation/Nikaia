//! **`String`'s ground** (ADR-327 D4, #490 step 1 of the switch): valid UTF-8
//! in a [`List`] of bytes, placed by its handle, growing only through
//! `try_reserve`. What only reads is the `str`'s ([`Deref`]).
//!
//! **Toward Rust it crosses as it is** where it lives outside the running
//! task's block (D8): `String::from(text)` hands over the same memory, as
//! `Text::from(string)` takes it; only text in the block is copied out.

use crate::{List, Place};
use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Deref, DerefMut, RangeBounds};

/// **Text whose growth may be refused**: valid UTF-8 in a [`List`] of bytes.
/// Every byte in it came from a `str`, so it is handed out as one without
/// being checked again, as `String` does.
pub struct Text {
    bytes: List<u8>,
}

impl Text {
    /// A new, empty text placed at `place`.
    #[inline]
    pub const fn new_in(place: Place) -> Text {
        Text {
            bytes: List::new_in(place),
        }
    }

    /// A new, empty text outside every block (D9).
    #[inline]
    pub const fn new() -> Text {
        Text::new_in(Place::Outside)
    }

    /// A new text with room for `room` bytes, placed at `place`.
    #[inline]
    pub fn with_capacity_in(room: usize, place: Place) -> Text {
        Text {
            bytes: List::with_capacity_in(room, place),
        }
    }

    /// A new text with room for `room` bytes, outside every block.
    #[inline]
    pub fn with_capacity(room: usize) -> Text {
        Text::with_capacity_in(room, Place::Outside)
    }

    /// `s`, copied into a new text placed at `place`.
    #[inline]
    pub fn from_str_in(s: &str, place: Place) -> Text {
        let mut text = Text::with_capacity_in(s.len(), place);
        text.push_str(s);
        text
    }

    /// Where the text lives.
    #[inline]
    pub fn place(&self) -> Place {
        self.bytes.place()
    }

    /// How many bytes it has room for.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.bytes.capacity()
    }

    /// Room for `more` bytes, or the task ends (D6).
    #[inline]
    pub fn reserve(&mut self, more: usize) {
        self.bytes.reserve(more);
    }

    /// Append `s`, with one copy.
    #[inline]
    pub fn push_str(&mut self, s: &str) {
        self.bytes.extend_copied(s.as_bytes());
    }

    /// Append one character.
    #[inline]
    pub fn push(&mut self, c: char) {
        if c.is_ascii() {
            self.bytes.push(c as u8);
        } else {
            self.push_str(c.encode_utf8(&mut [0; 4]));
        }
    }

    /// The text, unchecked: it only ever grew by whole `str`s.
    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: every append is a whole `str`, and every change keeps it one.
        unsafe { std::str::from_utf8_unchecked(&self.bytes) }
    }

    /// The text, to change as a `str` changes, which keeps it UTF-8.
    #[inline]
    pub fn as_mut_str(&mut self) -> &mut str {
        // SAFETY: as `as_str`; a `&mut str` can only be changed into UTF-8.
        unsafe { std::str::from_utf8_unchecked_mut(&mut self.bytes) }
    }

    /// Its bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Its bytes, as the list they are.
    #[inline]
    pub fn into_bytes(self) -> List<u8> {
        self.bytes
    }

    /// `bytes` as text where they are UTF-8, in their place.
    #[inline]
    pub fn from_utf8(bytes: List<u8>) -> Result<Text, std::str::Utf8Error> {
        match std::str::from_utf8(&bytes) {
            Ok(_) => Ok(Text { bytes }),
            Err(e) => Err(e),
        }
    }

    /// Drop every byte, keeping the room.
    #[inline]
    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    /// Keep the first `len` bytes; `len` is on a character's boundary.
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        if len < self.len() {
            assert!(
                self.is_char_boundary(len),
                "new_len does not lie on a char boundary"
            );
            self.bytes.truncate(len);
        }
    }

    /// The last character, taken out.
    #[inline]
    pub fn pop(&mut self) -> Option<char> {
        let c = self.chars().next_back()?;
        self.bytes.truncate(self.len() - c.len_utf8());
        Some(c)
    }

    /// Take out the character at byte `at`.
    #[inline]
    pub fn remove(&mut self, at: usize) -> char {
        let Some(c) = self[at..].chars().next() else {
            panic!("cannot remove a char from the end of a string")
        };
        self.bytes.drain(at..at + c.len_utf8());
        c
    }

    /// Put `c` at byte `at`, a character's boundary.
    #[inline]
    pub fn insert(&mut self, at: usize, c: char) {
        self.insert_str(at, c.encode_utf8(&mut [0; 4]));
    }

    /// Put `s` at byte `at`, a character's boundary.
    #[inline]
    pub fn insert_str(&mut self, at: usize, s: &str) {
        assert!(
            self.is_char_boundary(at),
            "assertion failed: self.is_char_boundary(idx)"
        );
        self.bytes.extend_copied(s.as_bytes());
        // The new bytes are at the end: turned to `at`, the rest after them.
        self.bytes[at..].rotate_right(s.len());
    }

    /// Keep the characters `keep` says yes to.
    #[inline]
    pub fn retain(&mut self, mut keep: impl FnMut(char) -> bool) {
        let mut kept = Text::with_capacity_in(self.len(), self.place());
        for c in self.chars() {
            if keep(c) {
                kept.push(c);
            }
        }
        *self = kept;
    }

    /// Take out the bytes of `range` (on characters' boundaries) as text
    /// outside.
    #[inline]
    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Text {
        let (start, end) = bounds(range, self.len());
        let taken = Text::from_str_in(&self[start..end], Place::Outside);
        self.bytes.drain(start..end);
        taken
    }

    /// The text from byte `at` on, as new text in the same place.
    #[inline]
    pub fn split_off(&mut self, at: usize) -> Text {
        assert!(
            self.is_char_boundary(at),
            "assertion failed: self.is_char_boundary(at)"
        );
        Text {
            bytes: self.bytes.split_off(at),
        }
    }

    /// A copy built at `place` (`Clone` builds outside, as the map's does).
    #[inline]
    pub fn clone_in(&self, place: Place) -> Text {
        Text::from_str_in(self, place)
    }

    /// Whether its memory is the global allocator's ([`List::is_outside`]).
    #[inline]
    pub fn is_outside(&self) -> bool {
        self.bytes.is_outside()
    }

    /// ASCII letters made lower case where they are, which keeps the UTF-8
    /// valid.
    #[inline]
    pub fn make_ascii_lowercase(&mut self) {
        self.bytes.make_ascii_lowercase();
    }
}

/// `range`'s start and end in a text of `len` bytes.
fn bounds<R: RangeBounds<usize>>(range: R, len: usize) -> (usize, usize) {
    use std::ops::Bound;
    let start = match range.start_bound() {
        Bound::Included(&n) => n,
        Bound::Excluded(&n) => n + 1,
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(&n) => n + 1,
        Bound::Excluded(&n) => n,
        Bound::Unbounded => len,
    };
    (start, end)
}

impl Default for Text {
    #[inline]
    fn default() -> Text {
        Text::new()
    }
}

impl Deref for Text {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl DerefMut for Text {
    #[inline]
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl Borrow<str> for Text {
    #[inline]
    fn borrow(&self) -> &str {
        self
    }
}

impl AsRef<str> for Text {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl AsRef<[u8]> for Text {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

/// **A clone is built outside** (D9), as the map's is; [`Text::clone_in`]
/// places it.
impl Clone for Text {
    #[inline]
    fn clone(&self) -> Text {
        self.clone_in(Place::Outside)
    }
}

impl Hash for Text {
    /// As the `str`'s, so text hashes as `String` does and a map keyed by
    /// text keeps its order.
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl PartialEq for Text {
    #[inline]
    fn eq(&self, other: &Text) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Text {}

impl PartialOrd for Text {
    #[inline]
    fn partial_cmp(&self, other: &Text) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Text {
    #[inline]
    fn cmp(&self, other: &Text) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

/// Text compared with the other forms of text, both ways round.
macro_rules! compared {
    ($($other:ty),*) => {$(
        impl PartialEq<$other> for Text {
            #[inline]
            fn eq(&self, other: &$other) -> bool {
                self.as_str() == &other[..]
            }
        }

        impl PartialEq<Text> for $other {
            #[inline]
            fn eq(&self, other: &Text) -> bool {
                &self[..] == other.as_str()
            }
        }
    )*};
}

compared!(str, &str, String);

impl fmt::Write for Text {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.push(c);
        Ok(())
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl From<&str> for Text {
    #[inline]
    fn from(s: &str) -> Text {
        Text::from_str_in(s, Place::Outside)
    }
}

impl From<&String> for Text {
    #[inline]
    fn from(s: &String) -> Text {
        Text::from_str_in(s, Place::Outside)
    }
}

impl From<char> for Text {
    #[inline]
    fn from(c: char) -> Text {
        let mut text = Text::new();
        text.push(c);
        text
    }
}

/// **A Rust `String` is taken over as it is** (D8), outside every block.
impl From<String> for Text {
    #[inline]
    fn from(s: String) -> Text {
        Text {
            bytes: List::from(s.into_bytes()),
        }
    }
}

/// **Toward Rust text crosses as it is** (D8) where its memory is the
/// global allocator's ([`From<List<T>>`](List) for `Vec<T>`).
impl From<Text> for String {
    #[inline]
    fn from(text: Text) -> String {
        let bytes: Vec<u8> = text.bytes.into();
        // SAFETY: the bytes are the text's, UTF-8.
        unsafe { String::from_utf8_unchecked(bytes) }
    }
}

impl Extend<char> for Text {
    #[inline]
    fn extend<I: IntoIterator<Item = char>>(&mut self, chars: I) {
        let chars = chars.into_iter();
        self.reserve(chars.size_hint().0);
        for c in chars {
            self.push(c);
        }
    }
}

impl<'a> Extend<&'a char> for Text {
    #[inline]
    fn extend<I: IntoIterator<Item = &'a char>>(&mut self, chars: I) {
        self.extend(chars.into_iter().copied());
    }
}

/// Every form of text appended whole.
macro_rules! appended {
    ($([$($lt:lifetime)?] $part:ty),*) => {$(
        impl<$($lt)?> Extend<$part> for Text {
            #[inline]
            fn extend<I: IntoIterator<Item = $part>>(&mut self, parts: I) {
                for part in parts {
                    self.push_str(&part);
                }
            }
        }

        impl<$($lt)?> FromIterator<$part> for Text {
            #[inline]
            fn from_iter<I: IntoIterator<Item = $part>>(parts: I) -> Text {
                let mut text = Text::new();
                text.extend(parts);
                text
            }
        }
    )*};
}

appended!(['a] &'a str, [] String, [] Text, ['a] &'a Text, ['a] &'a String);

/// **`.collect()` builds outside** (D9).
impl FromIterator<char> for Text {
    #[inline]
    fn from_iter<I: IntoIterator<Item = char>>(chars: I) -> Text {
        let mut text = Text::new();
        text.extend(chars);
        text
    }
}

impl<'a> FromIterator<&'a char> for Text {
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a char>>(chars: I) -> Text {
        chars.into_iter().copied().collect()
    }
}

impl Add<&str> for Text {
    type Output = Text;

    #[inline]
    fn add(mut self, s: &str) -> Text {
        self.push_str(s);
        self
    }
}

impl AddAssign<&str> for Text {
    #[inline]
    fn add_assign(&mut self, s: &str) {
        self.push_str(s);
    }
}

/// **`s` in upper case, built at `place`**, as `str::to_uppercase` makes it,
/// with no `String` between.
pub fn to_uppercase(s: &str, place: Place) -> Text {
    let mut text = Text::with_capacity_in(s.len(), place);
    for c in s.chars() {
        if c.is_ascii() {
            text.bytes.push(c.to_ascii_uppercase() as u8);
        } else {
            text.extend(c.to_uppercase());
        }
    }
    text
}

/// **`s` in lower case, built at `place`**, as `str::to_lowercase` makes it.
/// A capital sigma's lower case depends on the letters around it (a final
/// `ς`), by Unicode properties `std` does not hand out; text with one is
/// made by `str::to_lowercase` and copied in, once.
pub fn to_lowercase(s: &str, place: Place) -> Text {
    if s.contains('Σ') {
        return Text::from_str_in(&s.to_lowercase(), place);
    }
    let mut text = Text::with_capacity_in(s.len(), place);
    for c in s.chars() {
        if c.is_ascii() {
            text.bytes.push(c.to_ascii_lowercase() as u8);
        } else {
            text.extend(c.to_lowercase());
        }
    }
    text
}

/// **`s` with every `from` replaced by `to`, built at `place`**, as
/// `str::replace` makes it.
pub fn replace(s: &str, from: &str, to: &str, place: Place) -> Text {
    replacen(s, from, to, usize::MAX, place)
}

/// **`s` with the first `count` of `from` replaced by `to`, built at
/// `place`**, as `str::replacen` makes it.
pub fn replacen(s: &str, from: &str, to: &str, count: usize, place: Place) -> Text {
    let mut text = Text::with_capacity_in(s.len(), place);
    let mut last = 0;
    for (at, found) in s.match_indices(from).take(count) {
        text.push_str(&s[last..at]);
        text.push_str(to);
        last = at + found.len();
    }
    text.push_str(&s[last..]);
    text
}

/// **`s`, `times` over, built at `place`**, as `str::repeat` makes it.
pub fn repeat(s: &str, times: usize, place: Place) -> Text {
    let Some(len) = s.len().checked_mul(times) else {
        panic!("capacity overflow")
    };
    let mut text = Text::with_capacity_in(len, place);
    for _ in 0..times {
        text.push_str(s);
    }
    text
}

/// **`parts` with `between` between each two, built at `place`**, as
/// `[S]::join` makes it.
pub fn join<S: Borrow<str>>(parts: &[S], between: &str, place: Place) -> Text {
    let len = parts.iter().map(|part| part.borrow().len()).sum::<usize>()
        + between.len() * parts.len().saturating_sub(1);
    let mut text = Text::with_capacity_in(len, place);
    for (n, part) in parts.iter().enumerate() {
        if n > 0 {
            text.push_str(between);
        }
        text.push_str(part.borrow());
    }
    text
}

/// **`format!`, built at `place`**: `format(place, format_args!(...))`.
pub fn format(place: Place, args: fmt::Arguments<'_>) -> Text {
    match args.as_str() {
        Some(s) => Text::from_str_in(s, place),
        None => {
            let mut text = Text::new_in(place);
            fmt::Write::write_fmt(&mut text, args)
                .expect("a Display implementation returned an error unexpectedly");
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, enter};
    use std::hash::BuildHasher;

    #[test]
    fn text_changes_as_a_string_does() {
        let block = Block::default();
        let _in = enter(&block);
        let mut text = Text::from_str_in("Grüße", Place::Block);
        let mut string = String::from("Grüße");
        text.insert_str(2, "ab");
        string.insert_str(2, "ab");
        text.insert(0, 'ü');
        string.insert(0, 'ü');
        assert_eq!(text.remove(2), string.remove(2));
        assert_eq!(text.pop(), string.pop());
        text.push('!');
        string.push('!');
        assert_eq!(text, string);
        text.retain(|c| c != 'a');
        string.retain(|c| c != 'a');
        assert_eq!(text.drain(..3), string.drain(..3).collect::<String>());
        assert_eq!(text.split_off(3), string.split_off(3));
        text.truncate(1);
        string.truncate(1);
        assert_eq!(text, string);
        assert!(!text.is_outside());
        text.make_ascii_uppercase();
        assert_eq!(text.as_str(), string.to_ascii_uppercase());
    }

    #[test]
    fn text_hashes_compares_and_prints_as_a_string() {
        let text: Text = ["ab", "c"].into_iter().collect();
        let string = String::from("abc");
        let hasher = std::collections::hash_map::RandomState::new();
        assert_eq!(hasher.hash_one(&text), hasher.hash_one(&string));
        let next = Text::from("abd");
        assert!(text == string && text == "abc" && "abc" == text && text < next);
        assert_eq!(
            format!("{text:>5}|{text:?}"),
            format!("{string:>5}|{string:?}")
        );
        assert_eq!((text.clone() + "d").as_str(), "abcd");
        assert_eq!(&text[1..], "bc");
        let chars: Text = text.chars().rev().collect();
        assert_eq!(chars, "cba");
        assert!(Text::from_utf8(List::from(vec![0xff_u8])).is_err());
    }

    #[test]
    fn toward_rust_text_outside_crosses_as_it_is() {
        let string = String::from("crosses");
        let at = string.as_ptr();
        let back = String::from(Text::from(string));
        assert_eq!((back.as_ptr(), back.as_str()), (at, "crosses"));
        let block = Block::default();
        let _in = enter(&block);
        let text = Text::from_str_in("copied out", Place::Block);
        assert!(!text.is_outside());
        assert_eq!(String::from(text), "copied out");
    }

    #[test]
    fn the_allocating_text_methods_make_what_strs_make() {
        let block = Block::default();
        let _in = enter(&block);
        let at = Place::Block;
        for s in ["", "abc", "Grüße, ὈΔΥΣΣΕΎΣ ﬁ İ", "ΣΑΣ σ Σ.", "ǅ ß ŉ"] {
            assert_eq!(to_uppercase(s, at), s.to_uppercase());
            assert_eq!(to_lowercase(s, at), s.to_lowercase());
            assert_eq!(replace(s, "Σ", "<>", at), s.replace("Σ", "<>"));
            assert_eq!(replace(s, "", "-", at), s.replace("", "-"));
            assert_eq!(replacen(s, "s", "", 1, at), s.replacen("s", "", 1));
            assert_eq!(repeat(s, 3, at), s.repeat(3));
        }
        assert_eq!(join(&["a", "b", "c"], ", ", at), ["a", "b", "c"].join(", "));
        assert_eq!(join::<String>(&[], ", ", at), "");
        let n = 42;
        assert_eq!(
            format(at, format_args!("{n:>4}|{:?}", "x")),
            format!("{n:>4}|{:?}", "x")
        );
        assert_eq!(format(at, format_args!("plain")), "plain");
        assert!(!format(at, format_args!("{n}")).is_outside());
    }
}
