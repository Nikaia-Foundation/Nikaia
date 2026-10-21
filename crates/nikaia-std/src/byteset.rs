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

use byte_blocks::Tables;
pub use byte_blocks::{BLOCK, HALF};

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

    /// **The block's mask**: bit `i` where `block[i]` is in the set, for a
    /// whole block or half of one. A set of one or two compares only its
    /// members; an empty one is looked up in its empty tables, which find
    /// nothing (a search never reads for it).
    #[inline(always)]
    fn mask<const L: usize>(&self, block: &[u8; L]) -> u64 {
        match self.way {
            Way::One => byte_blocks::any_of(block, [self.few[0]]),
            Way::Two => byte_blocks::any_of(block, [self.few[0], self.few[1]]),
            Way::Three => byte_blocks::any_of(block, self.few),
            Way::Empty | Way::Table => byte_blocks::in_tables(block, &self.tables),
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

/// **What a search compares each block with**: a [`ByteSet`], whose way is
/// chosen when it is built and looked up at each block, or a [`Few`], a set
/// written in the program whose way is fixed when the program is compiled.
pub trait Blocks: Copy {
    /// Whether it holds no byte: then nothing is found and nothing is read.
    fn holds_none(&self) -> bool;
    /// The block's mask: bit `i` where `block[i]` is in the set.
    fn mask(&self, block: &[u8; BLOCK]) -> u64;
    /// The mask of the last, short block, its bits past the end cleared.
    fn short_mask(self, run: &[u8]) -> u64;
    /// The mask of half a block: where the next hit is near, it is found
    /// at half the cost of a block.
    fn half_mask(&self, half: &[u8; HALF]) -> u64;
}

impl Blocks for ByteSet {
    #[inline(always)]
    fn holds_none(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn mask(&self, block: &[u8; BLOCK]) -> u64 {
        ByteSet::mask(self, block)
    }

    #[inline(always)]
    fn short_mask(self, run: &[u8]) -> u64 {
        ByteSet::short_mask(self, run)
    }

    #[inline(always)]
    fn half_mask(&self, half: &[u8; HALF]) -> u64 {
        ByteSet::mask(self, half)
    }
}

/// **A set of one to three bytes written in the program**, compared directly
/// in every block (ADR-375 D2): the lowering hands one to `find_all` where
/// the set is written there as a list of byte literals,
/// `b.find_all(bytes::ByteSet([';']))`, so that the search has no way to look
/// up at each block. It finds what the [`ByteSet`] of the same bytes finds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Few<const N: usize>(pub [u8; N]);

impl<const N: usize> Blocks for Few<N> {
    #[inline(always)]
    fn holds_none(&self) -> bool {
        N == 0
    }

    #[inline(always)]
    fn mask(&self, block: &[u8; BLOCK]) -> u64 {
        byte_blocks::any_of(block, self.0)
    }

    #[inline(always)]
    fn half_mask(&self, half: &[u8; HALF]) -> u64 {
        byte_blocks::any_of(half, self.0)
    }

    /// Off the hot loop, as [`ByteSet`]'s is.
    #[cold]
    #[inline(never)]
    fn short_mask(self, run: &[u8]) -> u64 {
        let mut padded = [0u8; BLOCK];
        padded[..run.len()].copy_from_slice(run);
        self.mask(&padded) & ((1u64 << run.len()) - 1)
    }
}

/// **What `find_all` takes**: a [`ByteSet`], one by reference, or a [`Few`].
pub trait Searched {
    /// What each block is compared with.
    type Blocks: Blocks;
    /// It, ready for the blocks.
    fn blocks(self) -> Self::Blocks;
}

impl Searched for ByteSet {
    type Blocks = ByteSet;
    #[inline(always)]
    fn blocks(self) -> ByteSet {
        self
    }
}

impl Searched for &ByteSet {
    type Blocks = ByteSet;
    #[inline(always)]
    fn blocks(self) -> ByteSet {
        *self
    }
}

impl<const N: usize> Searched for Few<N> {
    type Blocks = Few<N>;
    #[inline(always)]
    fn blocks(self) -> Few<N> {
        self
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

/// **How `find_all` goes from one hit to the next** (ADR-375 D6): both
/// yield the same positions; which costs less depends on how far apart the
/// hits lie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Scan {
    /// The mask of a block is kept, and the next block read once its hits
    /// are spent: a hit costs a lowest bit and a cleared one, and the test
    /// whether the mask is spent is mispredicted about once a block.
    #[default]
    ByBlock,
    /// The half block right after each hit is compared and its first hit
    /// taken; whole blocks are read only where that half holds none. A hit
    /// costs a compare, and where the next hit is mostly within half a block
    /// the test whether one was found is predicted.
    AfterHit,
}

/// **Every position of a byte of the set, rising** (ADR-375 D3): what
/// `find_all` hands back, by block or after each hit ([`Scan`]). By block it
/// keeps the mask of the block it is in and the bytes no mask has covered
/// yet, so `next` takes the lowest bit of a mask held in a register and reads
/// the next block only when the mask is spent: one length test a block, and
/// an offset that never passes the end. Its own `count`, `nth` and `last`
/// (D4) read whole blocks under either: a count adds the bits of each mask
/// and makes no position, and `next` hands out what `nth` left in the mask
/// before it reads again.
#[derive(Clone, Debug)]
pub struct FindAll<'a, S: Blocks = ByteSet> {
    /// The whole run searched: it does not change, so only `at` and `mask`
    /// change from one position to the next.
    hay: &'a [u8],
    set: S,
    /// Where the bytes no mask has covered yet start, never past the end;
    /// while `mask` holds hits, its bit 0 stands a block before it. After a
    /// hit ([`Scan::AfterHit`]) it is right after the hit.
    at: usize,
    /// The hits of the current block not handed out yet.
    mask: u64,
    scan: Scan,
}

impl<'a, S: Blocks> FindAll<'a, S> {
    /// The search of `hay` for `set` from `from`: a position before the start
    /// searches from the start, one past the end finds nothing. Inlined, so
    /// that the iterator's fields live in registers at the caller.
    #[inline]
    pub fn new(hay: &'a [u8], set: S, from: i64) -> FindAll<'a, S> {
        let start = usize::try_from(from.max(0))
            .unwrap_or(usize::MAX)
            .min(hay.len());
        let start = if set.holds_none() { hay.len() } else { start };
        FindAll {
            hay,
            set,
            at: start,
            mask: 0,
            scan: Scan::ByBlock,
        }
    }

    /// The same search, going from one hit to the next as `scan` says.
    #[inline]
    pub fn scan(self, scan: Scan) -> FindAll<'a, S> {
        FindAll { scan, ..self }
    }

    /// The bytes no mask has covered yet.
    #[inline(always)]
    fn rest(&self) -> &'a [u8] {
        &self.hay[self.at..]
    }

    /// The lowest bit of `mask`, a mask of the bytes from `at`, as a position;
    /// the search goes on right after it.
    #[inline(always)]
    fn hit(&mut self, mask: u64) -> i64 {
        let hit = self.at + mask.trailing_zeros() as usize;
        self.at = hit + 1;
        hit as i64
    }

    /// The next block's mask into `mask`; `false` at the end. A whole block
    /// costs one length test; the last, short one is the cold path.
    #[inline(always)]
    fn advance(&mut self) -> bool {
        let rest = self.rest();
        match rest.first_chunk::<BLOCK>() {
            Some(block) => {
                self.mask = self.set.mask(block);
                self.at += BLOCK;
            }
            None if rest.is_empty() => return false,
            // The short block's bits are moved to the top of a block that
            // ends where the run does, so that `at` stays inside the run.
            None => {
                self.mask = self.set.short_mask(rest) << (BLOCK - rest.len());
                self.at = self.hay.len();
            }
        }
        true
    }
}

impl<S: Blocks> Iterator for FindAll<'_, S> {
    type Item = i64;

    #[inline]
    fn next(&mut self) -> Option<i64> {
        if self.scan == Scan::ByBlock || self.mask != 0 {
            while self.mask == 0 {
                if !self.advance() {
                    return None;
                }
            }
            let bit = self.mask.trailing_zeros() as usize;
            self.mask &= self.mask - 1;
            return Some(self.at.wrapping_sub(BLOCK).wrapping_add(bit) as i64);
        }
        if let Some(half) = self.rest().first_chunk::<HALF>() {
            let mask = self.set.half_mask(half);
            if mask != 0 {
                return Some(self.hit(mask));
            }
        }
        loop {
            let rest = self.rest();
            let mask = match rest.first_chunk::<BLOCK>() {
                Some(block) => self.set.mask(block),
                None if rest.is_empty() => return None,
                None => self.set.short_mask(rest),
            };
            if mask != 0 {
                return Some(self.hit(mask));
            }
            self.at += rest.len().min(BLOCK);
        }
    }

    /// **A count adds the bits of each block's mask** (ADR-375 D4): no
    /// position is made.
    fn count(self) -> usize {
        let mut n = self.mask.count_ones() as usize;
        let mut rest = self.rest();
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
        let rest = self.rest();
        if !rest.is_empty() {
            let mut from = (rest.len() - 1) / BLOCK * BLOCK;
            loop {
                let run = &rest[from..];
                let mask = match run.first_chunk::<BLOCK>() {
                    Some(block) => self.set.mask(block),
                    None => self.set.short_mask(run),
                };
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
            mask => Some(
                self.at
                    .wrapping_sub(BLOCK)
                    .wrapping_add(63 - mask.leading_zeros() as usize) as i64,
            ),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.rest().len() + self.mask.count_ones() as usize;
        (0, Some(left))
    }
}

impl<S: Blocks> std::iter::FusedIterator for FindAll<'_, S> {}

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
    use super::{Blocks, ByteSet, Few, FindAll, Needle, Scan, Split};

    /// What a [`Few`] finds, counts and finds last against the [`ByteSet`] of
    /// the same bytes.
    fn same_as_the_set<S: Blocks>(few: S, members: &[u8]) {
        let set = ByteSet(members);
        let mut seed = 11u64;
        let alphabet: Vec<u8> = vec![b';', b'\n', b'a', 0x00, 0x80, 0xFF, b'x'];
        for n in 0..200 {
            let hay = bytes(&mut seed, n, &alphabet);
            for from in [0i64, -1, 1, 31, 63, 64, 65, 130, 250] {
                let want: Vec<i64> = FindAll::new(&hay, set, from).collect();
                let got: Vec<i64> = FindAll::new(&hay, few, from).collect();
                assert_eq!(got, want, "{members:?} {n} {from}");
                let after: Vec<i64> = FindAll::new(&hay, few, from).scan(Scan::AfterHit).collect();
                assert_eq!(after, want, "{members:?} {n} {from}");
                assert_eq!(FindAll::new(&hay, few, from).count(), want.len());
                assert_eq!(FindAll::new(&hay, few, from).last(), want.last().copied());
                assert_eq!(FindAll::new(&hay, few, from).nth(2), want.get(2).copied());
            }
        }
    }

    /// A set written in the program finds what the built set finds.
    #[test]
    fn a_few_bytes_find_what_their_set_finds() {
        same_as_the_set(Few(*b";"), b";");
        same_as_the_set(Few(*b";\n"), b";\n");
        same_as_the_set(Few([0x00, 0x80, 0xFF]), &[0x00, 0x80, 0xFF]);
        same_as_the_set(Few(*b";;"), b";");
        same_as_the_set(Few::<0>([]), &[]);
    }

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
                    for scan in [Scan::ByBlock, Scan::AfterHit] {
                        let mut walk = FindAll::new(&hay, set, from).scan(scan);
                        let got: Vec<i64> = walk.clone().collect();
                        assert_eq!(got, want, "{members:?} {n} {from} {scan:?}");
                        walk.next();
                        assert_eq!(walk.clone().count(), want.len().saturating_sub(1));
                        assert_eq!(walk.nth(1), want.get(2).copied());
                        assert_eq!(walk.next(), want.get(3).copied());
                    }
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

    /// Hits further apart than a half block or a block: `next` goes on
    /// over whole blocks, and `nth`, `count` and `last` after it agree.
    #[test]
    fn hits_far_apart_are_found_as_near_ones() {
        for gap in [31usize, 32, 33, 63, 64, 65, 97, 200] {
            for n in [0usize, 1, 31, 32, 64, 65, 129, 300, 700] {
                let hay: Vec<u8> = (0..n)
                    .map(|i| if i % gap == gap - 1 { b';' } else { b'x' })
                    .collect();
                let want: Vec<i64> = (0..n as i64).filter(|&i| hay[i as usize] == b';').collect();
                for set in [
                    ByteSet(b";"),
                    ByteSet((0..=255u8).filter(|&b| b != b'x').collect::<Vec<u8>>()),
                ] {
                    let got: Vec<i64> = FindAll::new(&hay, set, 0).collect();
                    assert_eq!(got, want, "{gap} {n}");
                    let few = FindAll::new(&hay, Few(*b";"), 0).scan(Scan::AfterHit);
                    assert_eq!(few.collect::<Vec<i64>>(), want, "{gap} {n}");
                    let mut walk = FindAll::new(&hay, set, 0).scan(Scan::AfterHit);
                    assert_eq!(walk.next(), want.first().copied());
                    assert_eq!(walk.clone().nth(1), want.get(2).copied());
                    assert_eq!(walk.clone().count(), want.len().saturating_sub(1));
                    assert_eq!(walk.last(), want.get(1..).and_then(|w| w.last()).copied());
                }
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
