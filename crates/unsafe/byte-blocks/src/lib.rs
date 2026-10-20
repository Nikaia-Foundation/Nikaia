//! **Which bytes of a 64-byte block belong to a set**, one bit a byte, at the
//! vector width the build's CPU level allows
//! ([ADR-375](../../../docs/specification/adr/adr-375.md) D1,
//! [ADR-002](../../../docs/specification/adr/adr-002.md) D6).
//!
//! The caller is `nikaia-std`'s `bytes::ByteSet`, which reads a buffer block
//! by block and hands each block here. The width is chosen by `cfg`, never at
//! run time:
//!
//! * AVX2 (`target_feature = "avx2"`, `--cpu=x86-64-v3`): two 32-byte
//!   compares a block;
//! * SSE2, x86-64's baseline: four 16-byte compares a block;
//! * any other target: eight 8-byte words a block.
//!
//! A set of one to three bytes is compared directly, one compare a member. A larger one is a
//! *shufti* lookup ([`Tables`]): a table over the low four bits and one over
//! the high four, and a byte is in the set where the two rows it picks share a
//! bit. That takes a byte shuffle (SSSE3, in AVX2 too); where the build has
//! none, the set's 256-bit map is read a byte at a time.
//!
//! The `unsafe` is the intrinsics: `core::arch` asks for it at every call from
//! a function that does not list the feature, even where the build enables it
//! for every function. Every `unsafe` here, and the argument for it, is listed
//! in `README.md`.

#![deny(unsafe_op_in_unsafe_fn)]
#![no_std]

/// How many bytes one mask covers: one bit a byte of a `u64`.
pub const BLOCK: usize = 64;

/// **A set of bytes as the lookup of a larger set reads it.**
///
/// Each high nibble picks a row of low nibbles. Every distinct row takes a bit:
/// `low[l]` holds the bits of the rows with `l` in them, `high[h]` the bit of
/// `h`'s row, and `x` is in the set where `low[x & 15] & high[x >> 4] != 0`. A
/// set whose high nibbles show more than eight distinct rows does not fit one
/// table pair; it takes a second (`two`), and with 16 high nibbles two pairs
/// fit every set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tables {
    low: [[u8; 16]; 2],
    high: [[u8; 16]; 2],
    two: bool,
    map: [u64; 4],
}

impl Tables {
    /// The tables of the set whose members are the bits of `map`: byte `b` is
    /// bit `b % 64` of `map[b / 64]`.
    pub fn new(map: [u64; 4]) -> Tables {
        let mut rows = [0u16; 16];
        for b in 0..=255u8 {
            if map[usize::from(b >> 6)] >> (b & 63) & 1 == 1 {
                rows[usize::from(b >> 4)] |= 1 << (b & 15);
            }
        }
        let mut distinct = [0u16; 16];
        let mut count = 0;
        for &row in &rows {
            if row != 0 && !distinct[..count].contains(&row) {
                distinct[count] = row;
                count += 1;
            }
        }
        let mut low = [[0u8; 16]; 2];
        let mut high = [[0u8; 16]; 2];
        for (bucket, &row) in distinct[..count].iter().enumerate() {
            let pair = bucket / 8;
            let bit = 1u8 << (bucket % 8);
            for nibble in 0..16 {
                if row >> nibble & 1 == 1 {
                    low[pair][nibble] |= bit;
                }
                if rows[nibble] == row {
                    high[pair][nibble] |= bit;
                }
            }
        }
        Tables {
            low,
            high,
            two: count > 8,
            map,
        }
    }

    /// Whether `byte` is in the set.
    #[inline(always)]
    pub fn contains(&self, byte: u8) -> bool {
        self.map[usize::from(byte >> 6)] >> (byte & 63) & 1 == 1
    }
}

/// **The bytes of `block` equal to one of `few`**: bit `i` where `block[i]`
/// is. One compare a member, so a set of one or two pays for no third; meant
/// for one to three members (none gives `0`).
#[inline(always)]
pub fn any_of<const N: usize>(block: &[u8; BLOCK], few: [u8; N]) -> u64 {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    // SAFETY: the build enables AVX2 for every function (`cfg`).
    return unsafe { avx2::any(block, few) };
    #[cfg(all(target_arch = "x86_64", not(target_feature = "avx2")))]
    // SAFETY: SSE2 is x86-64's baseline (`cfg`).
    return unsafe { sse2::any(block, few) };
    #[cfg(not(target_arch = "x86_64"))]
    return by_words(block, few);
}

/// **The bytes of `block` in the set `tables` holds**: bit `i` where
/// `block[i]` is.
#[inline(always)]
pub fn in_tables(block: &[u8; BLOCK], tables: &Tables) -> u64 {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    // SAFETY: the build enables AVX2 for every function (`cfg`).
    return unsafe { avx2::tables(block, tables) };
    #[cfg(all(
        target_arch = "x86_64",
        target_feature = "ssse3",
        not(target_feature = "avx2")
    ))]
    // SAFETY: the build enables SSSE3 for every function (`cfg`).
    return unsafe { sse2::tables(block, tables) };
    #[cfg(not(all(target_arch = "x86_64", target_feature = "ssse3")))]
    return by_map(block, tables);
}

/// The bytes of `block` in the set, a byte at a time through its map: the
/// way without a byte shuffle, and the reference the others are tested
/// against.
#[inline(always)]
pub fn by_map(block: &[u8; BLOCK], tables: &Tables) -> u64 {
    // A copy of the map: the loop reads it at a computed index, and a read
    // into the caller's `Tables` at one would keep the whole struct around
    // it (a search's iterator) in memory.
    let map = tables.map;
    let mut mask = 0u64;
    for (i, &b) in block.iter().enumerate() {
        mask |= (map[usize::from(b >> 6)] >> (b & 63) & 1) << i;
    }
    mask
}

/// The bytes of `block` equal to one of `few`, eight bytes a word: a byte
/// equal to a member leaves a zero byte behind `^`, which is found exactly.
#[inline(always)]
pub fn by_words<const N: usize>(block: &[u8; BLOCK], few: [u8; N]) -> u64 {
    const LOW7: u64 = 0x7f7f_7f7f_7f7f_7f7f;
    const ONES: u64 = 0x0101_0101_0101_0101;
    let zero = |v: u64| !(((v & LOW7).wrapping_add(LOW7)) | v | LOW7);
    let splat = few.map(|b| ONES.wrapping_mul(u64::from(b)));
    let mut mask = 0u64;
    for (w, chunk) in block.chunks_exact(8).enumerate() {
        let mut word = [0u8; 8];
        word.copy_from_slice(chunk);
        let v = u64::from_le_bytes(word);
        let hits = splat.iter().fold(0, |hits, &s| hits | zero(v ^ s));
        // The high bit of byte `i` to bit `i`: no two partial products
        // meet, so nothing carries.
        let packed = ((hits >> 7).wrapping_mul(0x0102_0408_1020_4080)) >> 56;
        mask |= packed << (8 * w);
    }
    mask
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
mod avx2 {
    use super::{BLOCK, Tables};
    use core::arch::x86_64::*;

    /// The two 32-byte halves of the block.
    #[inline]
    #[target_feature(enable = "avx2")]
    fn halves(block: &[u8; BLOCK]) -> (__m256i, __m256i) {
        let p = block.as_ptr();
        // SAFETY: each load reads 32 of the block's 64 bytes, unaligned.
        unsafe {
            (
                _mm256_loadu_si256(p.cast::<__m256i>()),
                _mm256_loadu_si256(p.add(32).cast::<__m256i>()),
            )
        }
    }

    #[inline]
    #[target_feature(enable = "avx2")]
    fn bits(a: __m256i, b: __m256i) -> u64 {
        u64::from(_mm256_movemask_epi8(a) as u32)
            | (u64::from(_mm256_movemask_epi8(b) as u32) << 32)
    }

    #[inline]
    #[target_feature(enable = "avx2")]
    pub(super) fn any<const N: usize>(block: &[u8; BLOCK], few: [u8; N]) -> u64 {
        let (a, b) = halves(block);
        let splat = few.map(|m| _mm256_set1_epi8(m as i8));
        let any = |v: __m256i| {
            let mut hits = _mm256_setzero_si256();
            for s in splat {
                hits = _mm256_or_si256(hits, _mm256_cmpeq_epi8(v, s));
            }
            hits
        };
        bits(any(a), any(b))
    }

    #[inline]
    #[target_feature(enable = "avx2")]
    fn table(t: &[u8; 16]) -> __m256i {
        // SAFETY: the load reads the 16 bytes of `t`, unaligned.
        _mm256_broadcastsi128_si256(unsafe { _mm_loadu_si128(t.as_ptr().cast::<__m128i>()) })
    }

    #[inline]
    #[target_feature(enable = "avx2")]
    pub(super) fn tables(block: &[u8; BLOCK], tables: &Tables) -> u64 {
        let (a, b) = halves(block);
        let nibble = _mm256_set1_epi8(0x0f);
        let lookup = |v: __m256i| {
            let lo = _mm256_and_si256(v, nibble);
            let hi = _mm256_and_si256(_mm256_srli_epi16(v, 4), nibble);
            let pair = |p: usize| {
                _mm256_and_si256(
                    _mm256_shuffle_epi8(table(&tables.low[p]), lo),
                    _mm256_shuffle_epi8(table(&tables.high[p]), hi),
                )
            };
            let mut hits = pair(0);
            if tables.two {
                hits = _mm256_or_si256(hits, pair(1));
            }
            _mm256_cmpeq_epi8(hits, _mm256_setzero_si256())
        };
        !bits(lookup(a), lookup(b))
    }
}

#[cfg(all(target_arch = "x86_64", not(target_feature = "avx2")))]
mod sse2 {
    use super::BLOCK;
    use core::arch::x86_64::*;

    /// The four 16-byte quarters of the block.
    #[inline]
    #[target_feature(enable = "sse2")]
    fn quarters(block: &[u8; BLOCK]) -> [__m128i; 4] {
        let p = block.as_ptr();
        // SAFETY: each load reads 16 of the block's 64 bytes, unaligned.
        [0, 16, 32, 48].map(|at| unsafe { _mm_loadu_si128(p.add(at).cast::<__m128i>()) })
    }

    #[inline]
    #[target_feature(enable = "sse2")]
    fn bits(parts: [__m128i; 4]) -> u64 {
        let mut mask = 0u64;
        for (i, part) in parts.into_iter().enumerate() {
            mask |= u64::from(_mm_movemask_epi8(part) as u16) << (16 * i);
        }
        mask
    }

    #[inline]
    #[target_feature(enable = "sse2")]
    pub(super) fn any<const N: usize>(block: &[u8; BLOCK], few: [u8; N]) -> u64 {
        let splat = few.map(|m| _mm_set1_epi8(m as i8));
        bits(quarters(block).map(|v| {
            let mut hits = _mm_setzero_si128();
            for s in splat {
                hits = _mm_or_si128(hits, _mm_cmpeq_epi8(v, s));
            }
            hits
        }))
    }

    #[cfg(target_feature = "ssse3")]
    #[inline]
    #[target_feature(enable = "ssse3")]
    pub(super) fn tables(block: &[u8; BLOCK], tables: &super::Tables) -> u64 {
        // SAFETY: the load reads the 16 bytes of `t`, unaligned.
        let table = |t: &[u8; 16]| unsafe { _mm_loadu_si128(t.as_ptr().cast::<__m128i>()) };
        let nibble = _mm_set1_epi8(0x0f);
        let lookup = |v: __m128i| {
            let lo = _mm_and_si128(v, nibble);
            let hi = _mm_and_si128(_mm_srli_epi16(v, 4), nibble);
            let pair = |p: usize| {
                _mm_and_si128(
                    _mm_shuffle_epi8(table(&tables.low[p]), lo),
                    _mm_shuffle_epi8(table(&tables.high[p]), hi),
                )
            };
            let mut hits = pair(0);
            if tables.two {
                hits = _mm_or_si128(hits, pair(1));
            }
            _mm_cmpeq_epi8(hits, _mm_setzero_si128())
        };
        !bits(quarters(block).map(lookup))
    }
}

#[cfg(test)]
mod tests {
    use super::{BLOCK, Tables, any_of, by_map, by_words, in_tables};

    fn block(seed: &mut u64) -> [u8; BLOCK] {
        let mut out = [0u8; BLOCK];
        for b in out.iter_mut() {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *b = (*seed >> 56) as u8;
        }
        out
    }

    fn map_of(members: &[u8]) -> [u64; 4] {
        let mut map = [0u64; 4];
        for &b in members {
            map[usize::from(b >> 6)] |= 1 << (b & 63);
        }
        map
    }

    /// Every way agrees with the byte-by-byte one, on sets of one row, of more
    /// than eight rows, of every byte and of none.
    #[test]
    fn every_way_agrees_with_the_map() {
        let mut seed = 3u64;
        let mut every = [0u8; 256];
        for (i, b) in every.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut wide = [0u8; 37];
        for (i, b) in wide.iter_mut().enumerate() {
            *b = (i * 7) as u8;
        }
        let sets: [&[u8]; 6] = [
            &[b';', b'\n', 0xFF, 0x80],
            &every,
            &wide,
            &[],
            &[0x00, 0x10, 0x20, 0x31],
            &[0x05; 4],
        ];
        let rounds = if cfg!(miri) { 4 } else { 200 };
        for members in sets {
            let tables = Tables::new(map_of(members));
            for _ in 0..rounds {
                let b = block(&mut seed);
                let want = by_map(&b, &tables);
                assert_eq!(in_tables(&b, &tables), want, "{members:?}");
                if (1..=3).contains(&members.len()) || members == [0x05; 4] {
                    let few = [
                        members[0],
                        members[members.len().min(2) - 1],
                        members[members.len() - 1],
                    ];
                    let want = by_map(&b, &Tables::new(map_of(&few)));
                    assert_eq!(any_of(&b, few), want);
                    assert_eq!(by_words(&b, few), want);
                    let one = [members[0]];
                    let two = [members[0], members[members.len() - 1]];
                    assert_eq!(any_of(&b, one), by_map(&b, &Tables::new(map_of(&one))));
                    assert_eq!(by_words(&b, one), any_of(&b, one));
                    assert_eq!(any_of(&b, two), by_map(&b, &Tables::new(map_of(&two))));
                    assert_eq!(by_words(&b, two), any_of(&b, two));
                }
            }
        }
        for few in [[0u8, 0x80, 0xFF], *b";\n\n", [7, 7, 7]] {
            for _ in 0..rounds {
                let b = block(&mut seed);
                let want = by_map(&b, &Tables::new(map_of(&few)));
                assert_eq!(any_of(&b, few), want);
                assert_eq!(by_words(&b, few), want);
            }
        }
    }
}
