//! `Bytes`: one shared buffer, and the thing a view of a file points into.
//!
//! Part III 17.2 says what it is: *`read` returns `Bytes`, not a `Vec[u8]`: it
//! is one shared buffer, and slices that outlive its scope are tethered to it
//! (Part I 6.6)*. Both halves of that sentence matter here and only the first
//! is this file's: a `Bytes` is a reference-counted run of bytes, so handing
//! one on costs a count rather than a copy. The second half — a slice that
//! *outlives* the buffer's scope — is the tether, which is not built, and the
//! compiler refuses the programs that would need it (`NK2303`) rather than
//! letting them reach this type.
//!
//! **A `Bytes` carries what owns its memory**
//! ([ADR-373](../../../docs/specification/adr/adr-373.md) D1): where its bytes
//! are and how many, and a counted handle on the owner - memory it allocated,
//! a `Vec[u8]` handed over (`Bytes(v)`, D2, nothing copied), or a mapped file
//! (D3). Reading is the same for each, the buffer is immutable once it exists,
//! and a clone is a count. The pointer handling is `shared_bytes`'s
//! (ADR-218), which says why it is sound.

use shared_bytes::Shared;

pub use crate::byteset::{ByteSet, FindAll, Needle, Split, split};

/// A run of bytes that several holders may share
/// ([ADR-283](../../../docs/specification/adr/adr-283.md) D20, ADR-373 D1):
/// equal, ordered and hashed by content, whoever owns it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Bytes {
    buffer: Shared,
}

/// Bytes are a buffer a view of text may be cut from (ADR-283 D15).
impl crate::tether::Viewed for Bytes {
    fn bytes(&self) -> &[u8] {
        self.buffer.as_slice()
    }
}

impl Bytes {
    /// No bytes at all.
    pub fn new() -> Bytes {
        Bytes {
            buffer: Shared::default(),
        }
    }

    /// The bytes `owner` lends, owned by the `Bytes` from now on: nothing is
    /// copied (ADR-373 D1). What a mapped file hands over.
    pub fn from_owner<O: AsRef<[u8]> + Send + Sync + 'static>(owner: O) -> Bytes {
        Bytes {
            buffer: Shared::from_owner(owner),
        }
    }

    /// The bytes an owner counted elsewhere lends, at one more count.
    pub fn from_arc<O: AsRef<[u8]> + Send + Sync + 'static>(owner: std::sync::Arc<O>) -> Bytes {
        Bytes {
            buffer: Shared::from_arc(owner),
        }
    }

    /// The bytes `range` names, as a `Bytes` of their own that shares this
    /// one's owner: it keeps the memory alive as long as it lives. Stops the
    /// program as a cut does where the range is not inside.
    #[track_caller]
    pub fn cut(&self, range: impl std::ops::RangeBounds<usize>) -> Bytes {
        Bytes {
            buffer: self.buffer.cut(range),
        }
    }

    /// How many bytes there are.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// The bytes, as the language below spells a run of them.
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        self.buffer.as_slice()
    }

    /// The bytes as text, where they are text (ADR-320 D7).
    ///
    /// A **view** and not a copy: it points into the buffer and lives as long
    /// as the borrow does. `NotText` where the bytes are not UTF-8, the
    /// failure `fs::read_to_string` reports for the same bytes.
    pub fn text(&self) -> Result<&str, crate::io::IoError> {
        ByteReads::text(self.as_slice())
    }

    /// The first `byte` at or after the position `from`, or `None`
    /// (ADR-320 D6): a byte search of the kind `memchr` does. A `from` before
    /// the start searches from the start; one past the end finds nothing.
    /// `byte` may be a [`ByteSet`]: the first byte of the set (ADR-375 D3).
    #[inline]
    pub fn find<N: Needle>(&self, byte: N, from: i64) -> Option<i64> {
        byte.first_in(self.as_slice(), from)
    }

    /// Every position at or after `from` that holds a byte of `set`, rising
    /// (ADR-375 D3).
    #[inline]
    pub fn find_all(&self, set: impl Into<ByteSet>, from: i64) -> FindAll<'_> {
        FindAll::new(self.as_slice(), set.into(), from)
    }

    /// The pieces between the bytes of `set`, by text's `split` rules
    /// (ADR-375 D3).
    #[inline]
    pub fn split(&self, set: impl Into<ByteSet>) -> Split<'_> {
        Split::new(self.as_slice(), set.into())
    }

    /// The 4 bytes from `at` as an unsigned number, least significant byte
    /// first (ADR-320 D12): [`ByteReads::u32_le`].
    #[inline]
    #[track_caller]
    pub fn u32_le(&self, at: i64) -> u32 {
        ByteReads::u32_le(self.as_slice(), at)
    }

    /// The 4 bytes from `at` as an unsigned number, most significant byte
    /// first (ADR-320 D12): [`ByteReads::u32_be`].
    #[inline]
    #[track_caller]
    pub fn u32_be(&self, at: i64) -> u32 {
        ByteReads::u32_be(self.as_slice(), at)
    }

    /// The 8 bytes from `at` as an unsigned number, least significant byte
    /// first (ADR-320 D12): [`ByteReads::u64_le`].
    #[inline]
    #[track_caller]
    pub fn u64_le(&self, at: i64) -> u64 {
        ByteReads::u64_le(self.as_slice(), at)
    }

    /// The 8 bytes from `at` as an unsigned number, most significant byte
    /// first (ADR-320 D12): [`ByteReads::u64_be`].
    #[inline]
    #[track_caller]
    pub fn u64_be(&self, at: i64) -> u64 {
        ByteReads::u64_be(self.as_slice(), at)
    }

    /// A copy of the bytes, owned by the caller.
    ///
    /// The one place this type allocates on the way out, and it says so: Part I
    /// 6.6's promotion to `Owned` is never automatic.
    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

/// **The reads of a `Bytes` on any run of bytes** (ADR-320 D7, D11): a cut
/// `b[a..<b]`, a read-only `Bytes` parameter and a text's own bytes
/// (`s.bytes`) are all a `&[u8]`, and each is read as a `Bytes` is.
pub trait ByteReads {
    /// The bytes as text, where they are text: [`Bytes::text`].
    fn text(&self) -> Result<&str, crate::io::IoError>;
    /// The first `byte`, or byte of a set, at or after `from`, or `None`:
    /// [`Bytes::find`].
    fn find<N: Needle>(&self, byte: N, from: i64) -> Option<i64>;
    /// Every position of a byte of `set` from `from`: [`Bytes::find_all`].
    /// (`split` is [`split`]: a slice's own `split` would be the one called.)
    fn find_all(&self, set: impl Into<ByteSet>, from: i64) -> FindAll<'_>;
    /// The 4 bytes from `at`, least significant first: [`Bytes::u32_le`].
    fn u32_le(&self, at: i64) -> u32;
    /// The 4 bytes from `at`, most significant first: [`Bytes::u32_be`].
    fn u32_be(&self, at: i64) -> u32;
    /// The 8 bytes from `at`, least significant first: [`Bytes::u64_le`].
    fn u64_le(&self, at: i64) -> u64;
    /// The 8 bytes from `at`, most significant first: [`Bytes::u64_be`].
    fn u64_be(&self, at: i64) -> u64;
}

impl ByteReads for [u8] {
    fn text(&self) -> Result<&str, crate::io::IoError> {
        std::str::from_utf8(self).map_err(|at| {
            crate::io::IoError::NotText(format!(
                "bytes that are not UTF-8 from byte {}",
                at.valid_up_to()
            ))
        })
    }

    #[inline]
    fn find<N: Needle>(&self, byte: N, from: i64) -> Option<i64> {
        byte.first_in(self, from)
    }

    #[inline]
    fn find_all(&self, set: impl Into<ByteSet>, from: i64) -> FindAll<'_> {
        FindAll::new(self, set.into(), from)
    }

    #[inline]
    #[track_caller]
    fn u32_le(&self, at: i64) -> u32 {
        u32::from_le_bytes(word(self, at))
    }

    #[inline]
    #[track_caller]
    fn u32_be(&self, at: i64) -> u32 {
        u32::from_be_bytes(word(self, at))
    }

    #[inline]
    #[track_caller]
    fn u64_le(&self, at: i64) -> u64 {
        u64::from_le_bytes(word(self, at))
    }

    #[inline]
    #[track_caller]
    fn u64_be(&self, at: i64) -> u64 {
        u64::from_be_bytes(word(self, at))
    }
}

/// **The `N` bytes from `at`, read as the cut `b[at..<at + N]` is**
/// (ADR-320 D12): a position outside stops the program with the cut's own
/// message, at the caller's line.
#[inline(always)]
#[track_caller]
fn word<const N: usize>(bytes: &[u8], at: i64) -> [u8; N] {
    let cut = crate::index::get(bytes, crate::index::at(at..at.wrapping_add(N as i64)));
    match <[u8; N]>::try_from(cut) {
        Ok(word) => word,
        Err(_) => unreachable!("a cut of {N} bytes has {N} bytes"),
    }
}

/// **A copy of bytes is a `Bytes` of its own** (ADR-320 D11): `clone()` on a
/// `Bytes` shares its buffer (ADR-283 D20), and on a `&[u8]` - a read-only
/// `Bytes` parameter, a text's own bytes - copies the run into one.
pub trait ByteCopy {
    /// A `Bytes` of its own with these bytes.
    fn copy_of(&self) -> Bytes;
}

impl ByteCopy for Bytes {
    fn copy_of(&self) -> Bytes {
        self.clone()
    }
}

impl ByteCopy for [u8] {
    fn copy_of(&self) -> Bytes {
        Bytes::from(self)
    }
}

/// **A `Bytes` handed to a parameter that keeps it** (ADR-373 D1, D4): a
/// `ref Bytes` the callee keeps is a `&Bytes`, and each caller hands over what
/// it has - a `Bytes` as it is, a mapping at a count, a run of bytes (a text's,
/// a cut) copied into one.
pub trait KeptBytes {
    /// This value as a `Bytes` with its owner, borrowed where it is one.
    fn kept_bytes(&self) -> std::borrow::Cow<'_, Bytes>;
}

impl KeptBytes for Bytes {
    fn kept_bytes(&self) -> std::borrow::Cow<'_, Bytes> {
        std::borrow::Cow::Borrowed(self)
    }
}

impl KeptBytes for [u8] {
    fn kept_bytes(&self) -> std::borrow::Cow<'_, Bytes> {
        std::borrow::Cow::Owned(Bytes::from(self))
    }
}

impl<T: KeptBytes + ?Sized> KeptBytes for &T {
    fn kept_bytes(&self) -> std::borrow::Cow<'_, Bytes> {
        (**self).kept_bytes()
    }
}

/// What the lowering writes at such an argument: `&*kept(&m)` (the `&`
/// the call writes for a lent argument stays, and is looked through).
pub fn kept<T: KeptBytes + ?Sized>(value: &T) -> std::borrow::Cow<'_, Bytes> {
    value.kept_bytes()
}

impl std::ops::Deref for Bytes {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &[u8] {
        self.buffer.as_slice()
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        self.buffer.as_slice()
    }
}

/// **`Bytes(v)`** (ADR-373 D2): the call a program writes, the type's name
/// with a `Vec[u8]` it hands over. A function in the value namespace beside
/// the struct in the type namespace, so the generated Rust writes the call as
/// the program does; it is the `From` below, which copies nothing.
#[allow(non_snake_case)]
#[inline]
pub fn Bytes(bytes: Vec<u8>) -> Bytes {
    Bytes::from(bytes)
}

/// **`Bytes(v)`: the vector is handed over and nothing is copied**
/// (ADR-373 D2): its memory becomes the buffer.
impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Bytes {
        Bytes::from_owner(bytes)
    }
}

/// A run of bytes copied into a `Bytes` of its own: one allocation of exactly
/// its length.
impl From<&[u8]> for Bytes {
    fn from(bytes: &[u8]) -> Bytes {
        Bytes::from_owner(Box::<[u8]>::from(bytes))
    }
}

impl From<String> for Bytes {
    fn from(text: String) -> Bytes {
        Bytes::from(text.into_bytes())
    }
}

impl From<&str> for Bytes {
    fn from(text: &str) -> Bytes {
        Bytes::from(text.as_bytes())
    }
}

impl FromIterator<u8> for Bytes {
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Bytes {
        Bytes::from(iter.into_iter().collect::<Vec<u8>>())
    }
}

impl<'a> IntoIterator for &'a Bytes {
    type Item = &'a u8;
    type IntoIter = std::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

#[cfg(test)]
mod tests {
    use super::Bytes;

    /// **Handing one on is a count**, which is the whole claim of the type: two
    /// holders, one allocation.
    #[test]
    fn a_clone_shares_the_buffer() {
        let one = Bytes::from(vec![1u8, 2, 3]);
        let two = one.clone();
        assert_eq!(one.as_slice().as_ptr(), two.as_slice().as_ptr());
        assert_eq!(&*two, &[1u8, 2, 3]);
    }

    /// **`Bytes(v)` hands the vector over** (ADR-373 D2): the buffer is the
    /// vector's memory. And **a `Bytes` is its content**, whatever owns it
    /// (D1): a vector handed over, a copy of a run, text, a cut.
    #[test]
    fn a_vec_is_handed_over_and_equal_by_content_across_owners() {
        use std::hash::{BuildHasher, RandomState};
        let v = b"Oslo;3".to_vec();
        let at = v.as_ptr();
        let handed = Bytes::from(v);
        assert_eq!(handed.as_slice().as_ptr(), at, "nothing copied");
        let copied = Bytes::from(&b"Oslo;3"[..]);
        let text = Bytes::from(String::from("Oslo;3"));
        let cut = Bytes::from("x Oslo;3 x").cut(2..8);
        let hashes = RandomState::new();
        for other in [&copied, &text, &cut] {
            assert_eq!(&handed, other);
            assert_eq!(hashes.hash_one(&handed), hashes.hash_one(other));
        }
        assert!(handed < Bytes::from("Oslo;4"));
        assert_eq!(Bytes::default(), Bytes::new());
    }

    /// **It reads as a run of bytes**, so everything written against `&[u8]`
    /// takes one without knowing what it is.
    #[test]
    fn it_derefs_to_a_slice() {
        let bytes = Bytes::from("Ada");
        assert_eq!(bytes.len(), 3);
        assert!(!bytes.is_empty());
        assert_eq!(bytes.first(), Some(&b'A'));
    }

    /// **Text is a view of it**, and only where the bytes are text.
    #[test]
    fn text_is_a_view_and_may_not_be_there() {
        assert_eq!(Bytes::from("Ada").text().ok(), Some("Ada"));
        assert!(matches!(
            Bytes::from(vec![0xff_u8]).text(),
            Err(crate::io::IoError::NotText(_))
        ));
        let b = Bytes::from("a;b;c");
        assert_eq!(b.find(b';', 0), Some(1));
        assert_eq!(b.find(b';', 2), Some(3));
        assert_eq!(b.find(b';', 4), None);
        assert_eq!(b.find(b';', 9), None);
    }

    /// **`find` finds a byte wherever it is** (#701): at every offset of a
    /// 64-byte buffer, from every start, where `memchr`'s word-wide search
    /// and its tail meet; and nothing where it is absent or there are no bytes.
    #[test]
    fn find_at_every_offset() {
        for at in 0..64usize {
            let mut v = vec![b'.'; 64];
            v[at] = b';';
            let b = Bytes::from(v);
            for from in -1..=65i64 {
                let want = (from <= at as i64).then_some(at as i64);
                assert_eq!(b.find(b';', from), want, "needle {at}, from {from}");
            }
            assert_eq!(b.find(b'x', 0), None);
        }
        assert_eq!(Bytes::new().find(b';', 0), None);
        assert_eq!(Bytes::from(vec![b';'; 64]).find(b';', 63), Some(63));
    }

    /// **A word is read in the order its name says** (ADR-320 D12), on a
    /// `Bytes` and on a text's own bytes alike, up to the last whole word.
    #[test]
    fn words_in_both_orders() {
        let b = Bytes::from((1u8..=9).collect::<Vec<u8>>());
        assert_eq!(b.u32_le(0), 0x0403_0201);
        assert_eq!(b.u32_be(0), 0x0102_0304);
        assert_eq!(b.u64_le(1), 0x0908_0706_0504_0302);
        assert_eq!(b.u64_be(1), 0x0203_0405_0607_0809);
        assert_eq!(b.u32_le(5), 0x0908_0706);
        let text: &[u8] = b"abcd";
        assert_eq!(super::ByteReads::u32_be(text, 0), 0x6162_6364);
    }

    /// **And outside, it stops as the cut does**: with the cut's message.
    #[test]
    #[should_panic(expected = "range end index 9 out of range for slice of length 8")]
    fn a_word_past_the_end_stops() {
        let _ = Bytes::from(vec![0u8; 8]).u64_le(1);
    }

    #[test]
    #[should_panic(expected = "index out of bounds: the index is -1")]
    fn a_word_before_the_start_stops() {
        let _ = Bytes::from(vec![0u8; 8]).u32_le(-1);
    }

    /// **And a copy is asked for**, never taken.
    #[test]
    fn a_copy_is_asked_for() {
        let bytes = Bytes::from(vec![7u8]);
        assert_eq!(bytes.to_vec(), vec![7u8]);
        assert_eq!(Bytes::new().len(), 0);
        assert_eq!((1u8..=3).collect::<Bytes>().to_vec(), vec![1, 2, 3]);
    }
}
