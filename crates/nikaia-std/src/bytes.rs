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

pub use crate::byteset::{Blocks, ByteSet, Few, FindAll, Needle, Scan, Searched, Split, split};

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

    /// The bytes as text, each run that is not UTF-8 replaced by U+FFFD
    /// (#1182): an owned copy, never a refusal, for output a program only
    /// shows, such as a child process's.
    pub fn text_lossy(&self) -> crate::block::Text {
        crate::block::Text::from_utf8_lossy(self.as_slice())
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
    /// (ADR-375 D3), going from one hit to the next as `scan` says (D6).
    #[inline]
    pub fn find_all<S: Searched>(&self, set: S, from: i64, scan: Scan) -> FindAll<'_, S::Blocks> {
        FindAll::new(self.as_slice(), set.blocks(), from).scan(scan)
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
    /// The bytes as text, each run that is not UTF-8 U+FFFD:
    /// [`Bytes::text_lossy`].
    fn text_lossy(&self) -> crate::block::Text;
    /// The first `byte`, or byte of a set, at or after `from`, or `None`:
    /// [`Bytes::find`].
    fn find<N: Needle>(&self, byte: N, from: i64) -> Option<i64>;
    /// Every position of a byte of `set` from `from`: [`Bytes::find_all`].
    /// (`split` is [`split`]: a slice's own `split` would be the one called.)
    fn find_all<S: Searched>(&self, set: S, from: i64, scan: Scan) -> FindAll<'_, S::Blocks>;
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
    fn text_lossy(&self) -> crate::block::Text {
        crate::block::Text::from_utf8_lossy(self)
    }

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
    fn find_all<S: Searched>(&self, set: S, from: i64, scan: Scan) -> FindAll<'_, S::Blocks> {
        FindAll::new(self, set.blocks(), from).scan(scan)
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

/// **A cut of a text's own bytes read back as text** (Part I 2.6, #1870):
/// `s.bytes[a..<b].text()`. The text is UTF-8 already, so only the cut's two
/// ends can split a scalar: two boundary checks (`str::get`), not a
/// `from_utf8` over the whole cut. The emitter writes it where the cut's base
/// is a place it may read twice; a cut that is not of `text` is read the long
/// way, and a refusal is worded as [`ByteReads::text`] words it.
#[inline]
pub fn text_of_a_cut<'a>(text: &'a str, cut: &'a [u8]) -> Result<&'a str, crate::io::IoError> {
    let from = (cut.as_ptr() as usize).wrapping_sub(text.as_ptr() as usize);
    if from <= text.len()
        && cut.len() <= text.len() - from
        && let Some(part) = text.get(from..from + cut.len())
    {
        return Ok(part);
    }
    cut.text()
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

/// **Bytes equal a cut of bytes with the same content** (#776):
/// `name == buf[a..<b]`, either way round.
impl PartialEq<[u8]> for Bytes {
    #[inline(always)]
    fn eq(&self, other: &[u8]) -> bool {
        self.buffer.as_slice() == other
    }
}

impl PartialEq<&[u8]> for Bytes {
    #[inline(always)]
    fn eq(&self, other: &&[u8]) -> bool {
        self.buffer.as_slice() == *other
    }
}

impl PartialEq<Bytes> for [u8] {
    #[inline(always)]
    fn eq(&self, other: &Bytes) -> bool {
        self == other.buffer.as_slice()
    }
}

impl PartialEq<Bytes> for &[u8] {
    #[inline(always)]
    fn eq(&self, other: &Bytes) -> bool {
        *self == other.buffer.as_slice()
    }
}

/// **And are ordered against one as against another `Bytes`** (#2082):
/// `name < text.bytes`, either way round, byte by byte, as `==` compares.
impl PartialOrd<[u8]> for Bytes {
    #[inline]
    fn partial_cmp(&self, other: &[u8]) -> Option<std::cmp::Ordering> {
        self.buffer.as_slice().partial_cmp(other)
    }
}

impl PartialOrd<&[u8]> for Bytes {
    #[inline]
    fn partial_cmp(&self, other: &&[u8]) -> Option<std::cmp::Ordering> {
        self.buffer.as_slice().partial_cmp(*other)
    }
}

impl PartialOrd<Bytes> for [u8] {
    #[inline]
    fn partial_cmp(&self, other: &Bytes) -> Option<std::cmp::Ordering> {
        self.partial_cmp(other.buffer.as_slice())
    }
}

impl PartialOrd<Bytes> for &[u8] {
    #[inline]
    fn partial_cmp(&self, other: &Bytes) -> Option<std::cmp::Ordering> {
        (*self).partial_cmp(other.buffer.as_slice())
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        self.buffer.as_slice()
    }
}

/// **A map keyed by `Bytes` is looked up with a cut** (#1345): equal, ordered
/// and hashed by content, a `Bytes` borrows as the `[u8]` it holds.
impl std::borrow::Borrow<[u8]> for Bytes {
    fn borrow(&self) -> &[u8] {
        self.buffer.as_slice()
    }
}

/// **`Bytes(v)`** (ADR-373 D2): the call a program writes, the type's name
/// with a `Vec[u8]` it hands over. A function in the value namespace beside
/// the struct in the type namespace, so the generated Rust writes the call as
/// the program does; it is the `From` below, which copies nothing.
#[allow(non_snake_case)]
#[inline]
pub fn Bytes(bytes: impl Into<Vec<u8>>) -> Bytes {
    // A `Vec` as it is, or a block's list of bytes by its `From`, the same
    // memory where it is outside (ADR-327 D8, #490).
    Bytes::from(bytes.into())
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

/// **Two cuts compared, `a[i..<j] == b[k..<l]`** (#776 step 4 of the 1BRC
/// work): the lowering writes `nikaia_std::same!(a, b)` for an `==` or `!=`
/// whose sides are both cuts. It means `a == b`, whatever the types; two runs
/// of bytes, or two texts, are compared a word at a time in the caller's
/// code, so that a compare in a hot loop is no call into `memcmp`, whose
/// clobbered registers the loop would pay for on every row, even where the
/// call is taken on few of them (measured on `benches/brc`: 4.4 instructions
/// a row for a call kept on a cold path).
#[doc(hidden)]
#[macro_export]
macro_rules! same {
    ($a:expr, $b:expr) => {{
        #[allow(unused_imports)]
        use $crate::bytes::{SameAny as _, SameBytes as _};
        (&$crate::bytes::Same(&$a, &$b)).same()
    }};
}

/// The two sides of a [`same!`].
#[doc(hidden)]
pub struct Same<'a, A: ?Sized, B: ?Sized>(pub &'a A, pub &'a B);

/// [`same!`] of two runs of bytes or two texts: a word at a time.
#[doc(hidden)]
pub trait SameBytes {
    fn same(&self) -> bool;
}

impl SameBytes for Same<'_, &[u8], &[u8]> {
    #[inline(always)]
    fn same(&self) -> bool {
        same_bytes(self.0, self.1)
    }
}

impl SameBytes for Same<'_, &str, &str> {
    #[inline(always)]
    fn same(&self) -> bool {
        same_bytes(self.0.as_bytes(), self.1.as_bytes())
    }
}

/// [`same!`] of anything else: `==` itself.
#[doc(hidden)]
pub trait SameAny {
    fn same(&self) -> bool;
}

impl<A: ?Sized + PartialEq<B>, B: ?Sized> SameAny for &Same<'_, A, B> {
    #[inline(always)]
    fn same(&self) -> bool {
        *self.0 == *self.1
    }
}

/// Whether `a` and `b` hold the same bytes: a word at a time, the last word
/// read where it ends; below a word, two overlapping halves or three bytes.
#[inline(always)]
pub fn same_bytes(a: &[u8], b: &[u8]) -> bool {
    let n = a.len();
    if n != b.len() {
        return false;
    }
    if let (Some(x), Some(y)) = (a.last_chunk::<8>(), b.last_chunk::<8>()) {
        let (words_a, _) = a.as_chunks::<8>();
        let (words_b, _) = b.as_chunks::<8>();
        return words_a
            .iter()
            .zip(words_b)
            .all(|(p, q)| u64::from_ne_bytes(*p) == u64::from_ne_bytes(*q))
            && u64::from_ne_bytes(*x) == u64::from_ne_bytes(*y);
    }
    if let (Some(x), Some(y)) = (a.first_chunk::<4>(), b.first_chunk::<4>()) {
        let (p, q) = (a.last_chunk::<4>(), b.last_chunk::<4>());
        return u32::from_ne_bytes(*x) == u32::from_ne_bytes(*y) && p == q;
    }
    match n {
        0 => true,
        _ => a[0] == b[0] && a[n / 2] == b[n / 2] && a[n - 1] == b[n - 1],
    }
}
