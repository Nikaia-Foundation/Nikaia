//! **`bytes::ByteSet` and the searches over it** ([ADR-375](../../../docs/specification/adr/adr-375.md)):
//! a set of bytes, `0x00` to `0xFF`, prepared once for the search it serves,
//! and `find`, `find_all` and `split` over a run of bytes.
//!
//! **The width is the build's** (D1): the buffer is read in blocks of
//! [`BLOCK`] bytes and each block becomes a mask, one bit a byte, by
//! `byte-blocks` (`crates/unsafe/byte-blocks`, ADR-218) - AVX2 at
//! `--cpu=x86-64-v3`, SSE2 at x86-64's baseline, eight bytes a word on any
//! other target.
//!
//! **How a set is searched is decided when it is built** (D2): 1 to 3 bytes
//! are compared directly; more are a *shufti* lookup - a table over the low
//! four bits and one over the high four - in one table pair where the set's
//! high nibbles show at most eight distinct rows of low nibbles, in two
//! otherwise, which fit every set. Without a byte shuffle (SSSE3, which
//! x86-64's baseline lacks and `x86-64-v2` has, and the targets without a
//! vector path) a set of four or more is looked up a byte at a time in its
//! 256-bit map. An empty set finds nothing and reads nothing.
//!
//! **No read goes past the end** (ADR-320 D13): the last, short block is
//! copied into a block of zeros and the bits past the end are cleared.

pub use byte_blocks::BLOCK;
use byte_blocks::Tables;

/// The search a set was prepared for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Way {
    Empty,
    One,
    Two,
    Three,
    Table,
}

/// **A set of bytes, prepared once for the search it serves** (ADR-375 D2).
/// Built from a list of bytes; a byte written twice is in it once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ByteSet {
    way: Way,
    /// The members of a set of one to three, the last repeated into the
    /// slots it does not use.
    few: [u8; 3],
    tables: Tables,
}

/// **The set of `bytes`** (ADR-375 D2): `bytes::ByteSet([';', '\n'])`.
#[allow(non_snake_case)]
pub fn ByteSet(bytes: impl AsRef<[u8]>) -> ByteSet {
    ByteSet::of(bytes.as_ref())
}

impl ByteSet {
    /// The set of the bytes in `bytes`, prepared for its search.
    pub fn of(bytes: &[u8]) -> ByteSet {
        let mut map = [0u64; 4];
        for &b in bytes {
            map[usize::from(b >> 6)] |= 1u64 << (b & 63);
        }
        let tables = Tables::new(map);
        let mut few = [0u8; 3];
        let mut count = 0usize;
        for b in 0..=255u8 {
            if tables.contains(b) {
                few[count.min(2)] = b;
                count += 1;
            }
        }
        for slot in count.clamp(1, 3)..3 {
            few[slot] = few[slot - 1];
        }
        let way = match count {
            0 => Way::Empty,
            1 => Way::One,
            2 => Way::Two,
            3 => Way::Three,
            _ => Way::Table,
        };
        ByteSet { way, few, tables }
    }

    /// Whether `byte` is in the set.
    #[inline(always)]
    pub fn contains(&self, byte: u8) -> bool {
        self.tables.contains(byte)
    }

    /// Whether it holds none: then nothing is found.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.way == Way::Empty
    }

    /// **The block's mask**: bit `i` where `block[i]` is in the set. A set
    /// of one or two compares only its members; an empty one is looked up in
    /// its empty tables, which find nothing (a search never reads for it).
    #[inline(always)]
    fn mask(&self, block: &[u8; BLOCK]) -> u64 {
        match self.way {
            Way::One => byte_blocks::any_of(block, [self.few[0]]),
            Way::Two => byte_blocks::any_of(block, [self.few[0], self.few[1]]),
            Way::Three => byte_blocks::any_of(block, self.few),
            Way::Empty | Way::Table => byte_blocks::in_tables(block, &self.tables),
        }
    }

    /// The mask of the block at the start of `run`; a run shorter than a
    /// block is read from a copy with zeros behind it, and its bits past the
    /// end are cleared.
    #[inline(always)]
    fn mask_of(&self, run: &[u8]) -> u64 {
        match run.first_chunk::<BLOCK>() {
            Some(block) => self.mask(block),
            None => self.short_mask(run),
        }
    }

    /// The mask of the last, short block: off the hot loop, so that the copy
    /// takes no registers from it. The set comes by value, so that no
    /// reference into the caller's iterator escapes into the call.
    #[cold]
    #[inline(never)]
    fn short_mask(self, run: &[u8]) -> u64 {
        let mut padded = [0u8; BLOCK];
        padded[..run.len()].copy_from_slice(run);
        self.mask(&padded) & ((1u64 << run.len()) - 1)
    }
}

impl From<&ByteSet> for ByteSet {
    fn from(set: &ByteSet) -> ByteSet {
        *set
    }
}

/// **What `find` searches for** (ADR-375 D3): one byte, or a set.
pub trait Needle {
    /// The first position at or after `from` that holds it.
    fn first_in(self, hay: &[u8], from: i64) -> Option<i64>;
}

impl Needle for u8 {
    #[inline]
    fn first_in(self, hay: &[u8], from: i64) -> Option<i64> {
        let start = usize::try_from(from.max(0)).ok()?;
        let at = memchr::memchr(self, hay.get(start..)?)?;
        i64::try_from(start + at).ok()
    }
}

impl Needle for ByteSet {
    #[inline]
    fn first_in(self, hay: &[u8], from: i64) -> Option<i64> {
        let start = usize::try_from(from.max(0)).ok()?;
        let rest = hay.get(start..)?;
        let at = match self.way {
            Way::Empty => None,
            Way::One => memchr::memchr(self.few[0], rest),
            Way::Two => memchr::memchr2(self.few[0], self.few[1], rest),
            Way::Three => memchr::memchr3(self.few[0], self.few[1], self.few[2], rest),
            Way::Table => FindAll::new(rest, self, 0).next().map(|p| p as usize),
        }?;
        i64::try_from(start + at).ok()
    }
}

impl Needle for &ByteSet {
    #[inline]
    fn first_in(self, hay: &[u8], from: i64) -> Option<i64> {
        (*self).first_in(hay, from)
    }
}

/// **Every position of a byte of the set, rising** (ADR-375 D3): what
/// `find_all` hands back. It keeps the mask of the block it is in and the
/// bytes no mask has covered yet, so `next` takes the lowest bit of a mask
/// held in a register and reads the next block only when the mask is spent:
/// one length test a block, no bounds check. Its own `count`, `nth` and
/// `last` (D4) read whole blocks: a count adds the bits of each mask and
/// makes no position.
#[derive(Clone, Debug)]
pub struct FindAll<'a> {
    /// The bytes no mask has covered yet.
    rest: &'a [u8],
    set: ByteSet,
    /// Where bit 0 of `mask` stands.
    base: usize,
    /// Where `rest` starts, while it holds a byte.
    at: usize,
    /// The hits of the current block not handed out yet.
    mask: u64,
}

impl<'a> FindAll<'a> {
    /// The search of `hay` for `set` from `from`: a position before the start
    /// searches from the start, one past the end finds nothing. Inlined, so
    /// that the iterator's fields live in registers at the caller.
    #[inline]
    pub fn new(hay: &'a [u8], set: ByteSet, from: i64) -> FindAll<'a> {
        let start = usize::try_from(from.max(0))
            .unwrap_or(usize::MAX)
            .min(hay.len());
        let start = if set.is_empty() { hay.len() } else { start };
        let (_, rest) = hay.split_at(start);
        FindAll {
            rest,
            set,
            base: start,
            at: start,
            mask: 0,
        }
    }

    /// The next block's mask into `mask`; `false` at the end. A whole block
    /// costs one length test; the last, short one is the cold path.
    #[inline(always)]
    fn advance(&mut self) -> bool {
        match self.rest.split_first_chunk::<BLOCK>() {
            Some((block, tail)) => {
                self.mask = self.set.mask(block);
                self.rest = tail;
            }
            None if self.rest.is_empty() => return false,
            None => {
                self.mask = self.set.short_mask(self.rest);
                self.rest = &[];
            }
        }
        self.base = self.at;
        self.at += BLOCK;
        true
    }
}

impl Iterator for FindAll<'_> {
    type Item = i64;

    #[inline]
    fn next(&mut self) -> Option<i64> {
        while self.mask == 0 {
            if !self.advance() {
                return None;
            }
        }
        let bit = self.mask.trailing_zeros() as usize;
        self.mask &= self.mask - 1;
        Some((self.base + bit) as i64)
    }

    /// **A count adds the bits of each block's mask** (ADR-375 D4): no
    /// position is made.
    fn count(self) -> usize {
        let mut n = self.mask.count_ones() as usize;
        let mut rest = self.rest;
        while let Some((block, tail)) = rest.split_first_chunk::<BLOCK>() {
            n += self.set.mask(block).count_ones() as usize;
            rest = tail;
        }
        if !rest.is_empty() {
            n += self.set.short_mask(rest).count_ones() as usize;
        }
        n
    }

    /// **Whole blocks are skipped by their count** (ADR-375 D4).
    fn nth(&mut self, n: usize) -> Option<i64> {
        let mut left = n;
        loop {
            let here = self.mask.count_ones() as usize;
            if left < here {
                for _ in 0..left {
                    self.mask &= self.mask - 1;
                }
                return self.next();
            }
            left -= here;
            self.mask = 0;
            if !self.advance() {
                return None;
            }
        }
    }

    /// **The last is looked for from the end**, block by block (ADR-375 D4).
    fn last(self) -> Option<i64> {
        if !self.rest.is_empty() {
            let mut from = (self.rest.len() - 1) / BLOCK * BLOCK;
            loop {
                let mask = self.set.mask_of(&self.rest[from..]);
                if mask != 0 {
                    return Some((self.at + from + 63 - mask.leading_zeros() as usize) as i64);
                }
                if from == 0 {
                    break;
                }
                from -= BLOCK;
            }
        }
        match self.mask {
            0 => None,
            mask => Some((self.base + 63 - mask.leading_zeros() as usize) as i64),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.rest.len() + self.mask.count_ones() as usize;
        (0, Some(left))
    }
}

impl std::iter::FusedIterator for FindAll<'_> {}

/// **The pieces between the bytes of a set** (ADR-375 D3), by text's `split`
/// rules: two neighbouring bytes of the set give an empty piece, one at the
/// end an empty last piece, an empty run one empty piece, and an empty set the
/// whole run as its one piece.
#[derive(Clone, Debug)]
pub struct Split<'a> {
    hay: &'a [u8],
    at: FindAll<'a>,
    start: usize,
    done: bool,
}

impl<'a> Split<'a> {
    pub fn new(hay: &'a [u8], set: ByteSet) -> Split<'a> {
        Split {
            hay,
            at: FindAll::new(hay, set, 0),
            start: 0,
            done: false,
        }
    }
}

impl<'a> Iterator for Split<'a> {
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<&'a [u8]> {
        if self.done {
            return None;
        }
        match self.at.next() {
            Some(p) => {
                let p = p as usize;
                let piece = &self.hay[self.start..p];
                self.start = p + 1;
                Some(piece)
            }
            None => {
                self.done = true;
                Some(&self.hay[self.start..])
            }
        }
    }
}

impl std::iter::FusedIterator for Split<'_> {}

/// **`b.split(set)` on any run of bytes** (ADR-375 D3): what the emitter
/// writes for a `Bytes`, a cut and a text's own bytes alike, since a slice's
/// own `split` takes a closure and would be the one Rust picks.
pub fn split<'a, R: AsRef<[u8]> + ?Sized>(run: &'a R, set: impl Into<ByteSet>) -> Split<'a> {
    Split::new(run.as_ref(), set.into())
}

#[cfg(test)]
mod tests {
    use super::{ByteSet, FindAll, Needle, Split};

    /// A small generator, the same on every run.
    fn bytes(seed: &mut u64, n: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..n)
            .map(|_| {
                *seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                alphabet[(*seed >> 33) as usize % alphabet.len()]
            })
            .collect()
    }

    fn sets() -> Vec<Vec<u8>> {
        let mut wide: Vec<u8> = (0..=255u8).step_by(7).collect();
        wide.push(0xFF);
        vec![
            vec![],
            vec![b';'],
            vec![b';', b'\n'],
            vec![0x00, 0x80, 0xFF],
            vec![b'a', b'e', b'i', b'o', b'u'],
            (0..=255u8).collect(),
            wide,
            (0x10..0x20).chain([0x05, 0x95, 0xF3]).collect(),
        ]
    }

    /// Every search against a byte-by-byte reference, lengths 0..200 and every
    /// `from` near a block border.
    #[test]
    fn every_search_is_the_byte_by_byte_one() {
        let mut seed = 7u64;
        let alphabet: Vec<u8> = vec![
            b';', b'\n', b'a', b'e', 0x00, 0x80, 0xFF, 0x7F, 0x15, 0x95, b'x', b'y',
        ];
        for members in sets() {
            let set = ByteSet(members.clone());
            for n in 0..200 {
                let hay = bytes(&mut seed, n, &alphabet);
                for from in [0i64, -3, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 130, 250] {
                    let start = from.max(0) as usize;
                    let want: Vec<i64> = (start..n)
                        .filter(|&i| members.contains(&hay[i]))
                        .map(|i| i as i64)
                        .collect();
                    let got: Vec<i64> = FindAll::new(&hay, set, from).collect();
                    assert_eq!(got, want, "{members:?} {n} {from}");
                    assert_eq!(FindAll::new(&hay, set, from).count(), want.len());
                    assert_eq!(FindAll::new(&hay, set, from).last(), want.last().copied());
                    assert_eq!(set.first_in(&hay, from), want.first().copied());
                    for k in [0, 1, 2, 5, 40, 70] {
                        let mut walk = FindAll::new(&hay, set, from);
                        assert_eq!(walk.nth(k), want.get(k).copied());
                        assert_eq!(walk.next(), want.get(k + 1).copied());
                    }
                    let mut walk = FindAll::new(&hay, set, from);
                    walk.next();
                    assert_eq!(walk.clone().count(), want.len().saturating_sub(1));
                    assert_eq!(
                        walk.last(),
                        if want.len() > 1 {
                            want.last().copied()
                        } else {
                            None
                        }
                    );
                }
                let pieces: Vec<&[u8]> = Split::new(&hay, set).collect();
                let want: Vec<&[u8]> = hay.split(|b| members.contains(b)).collect();
                assert_eq!(pieces, want);
            }
        }
    }

    #[test]
    fn split_follows_text() {
        let set = ByteSet(vec![b';']);
        let pieces = |s: &str| -> Vec<String> {
            Split::new(s.as_bytes(), set)
                .map(|p| String::from_utf8_lossy(p).into_owned())
                .collect()
        };
        assert_eq!(pieces(""), vec![""]);
        assert_eq!(pieces("a;;b;"), vec!["a", "", "b", ""]);
        assert_eq!(pieces(";"), vec!["", ""]);
        let none: Vec<&[u8]> = Split::new(b"a;b", ByteSet(vec![])).collect();
        assert_eq!(none, vec![b"a;b".as_slice()]);
    }
}
