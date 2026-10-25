//! **Which bytes of a 64-byte block, or of half of one, belong to a set**,
//! one bit a byte, at the vector width the build's CPU level allows
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

/// Half a block: a search that expects its next hit soon compares these
/// first, at half the cost of a block.
pub const HALF: usize = BLOCK / 2;

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
/// for one to three members (none gives `0`). `block` is a whole block or
/// half of one ([`HALF`]).
#[inline(always)]
pub fn any_of<const L: usize, const N: usize>(block: &[u8; L], few: [u8; N]) -> u64 {
    const { assert!(L == BLOCK || L == HALF) };
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    // SAFETY: the build enables AVX2 for every function (`cfg`).
    return unsafe { avx2::any(block, few) };
    #[cfg(all(target_arch = "x86_64", not(target_feature = "avx2")))]
    // SAFETY: SSE2 is x86-64's baseline (`cfg`).
    return unsafe { sse2::any(block, few) };
    #[cfg(not(target_arch = "x86_64"))]
    return by_words(block, few);
}

/// **The bytes of `block` in a set held at run time**: the first `n` of
/// `few` compared directly where `n` is one to three, else the set `tables`
/// holds. One function for every way, so that under AVX2 the way is chosen
/// per 32-byte part and one `vpmovmskb` makes the mask: with a mask made in
/// each way and the ways joined after, LLVM joined them as vectors of single
/// bits and spilled the mask bit by bit (#780).
#[inline(always)]
pub fn few_or_tables<const L: usize>(
    block: &[u8; L],
    few: [u8; 3],
    n: usize,
    tables: &Tables,
) -> u64 {
    const { assert!(L == BLOCK || L == HALF) };
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    // SAFETY: the build enables AVX2 for every function (`cfg`).
    return unsafe { avx2::few_or_tables(block, few, n, tables) };
    #[cfg(not(all(target_arch = "x86_64", target_feature = "avx2")))]
    match n {
        1 => any_of(block, [few[0]]),
        2 => any_of(block, [few[0], few[1]]),
        3 => any_of(block, few),
        _ => in_tables(block, tables),
    }
}

/// **The bytes of `lanes` whose top bit is set**: bit `i` where `lanes[i]`
/// is at least 128, as a vector compare leaves a lane that holds. `L` is 16,
/// 32 or 64.
#[inline(always)]
pub fn high_bits<const L: usize>(lanes: &[u8; L]) -> u64 {
    const { assert!(L == 16 || L == 32 || L == 64) };
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    if L % 32 == 0 {
        // SAFETY: the build enables AVX2 for every function (`cfg`).
        return unsafe { avx2::high(lanes) };
    }
    #[cfg(target_arch = "x86_64")]
    // SAFETY: SSE2 is x86-64's baseline.
    return unsafe { sse2_high(lanes) };
    #[cfg(not(target_arch = "x86_64"))]
    {
        let mut bits = 0u64;
        for (i, &b) in lanes.iter().enumerate() {
            bits |= u64::from(b >> 7) << i;
        }
        bits
    }
}

/// [`high_bits`] 16 bytes a `pmovmskb`.
#[cfg(target_arch = "x86_64")]
#[inline]
#[target_feature(enable = "sse2")]
fn sse2_high<const L: usize>(lanes: &[u8; L]) -> u64 {
    use core::arch::x86_64::*;
    let mut mask = 0u64;
    for (i, part) in lanes.as_chunks::<16>().0.iter().enumerate() {
        // SAFETY: the load reads the 16 bytes of `part`, unaligned.
        let v = unsafe { _mm_loadu_si128(part.as_ptr().cast::<__m128i>()) };
        mask |= u64::from(_mm_movemask_epi8(v) as u16) << (16 * i);
    }
    mask
}

/// **The bytes of `block` in the set `tables` holds**: bit `i` where
/// `block[i]` is. `block` is a whole block or half of one ([`HALF`]).
#[inline(always)]
pub fn in_tables<const L: usize>(block: &[u8; L], tables: &Tables) -> u64 {
    const { assert!(L == BLOCK || L == HALF) };
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
pub fn by_map<const L: usize>(block: &[u8; L], tables: &Tables) -> u64 {
    const { assert!(L <= BLOCK) };
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
pub fn by_words<const L: usize, const N: usize>(block: &[u8; L], few: [u8; N]) -> u64 {
    const { assert!(L == BLOCK || L == HALF) };
    const LOW7: u64 = 0x7f7f_7f7f_7f7f_7f7f;
    const ONES: u64 = 0x0101_0101_0101_0101;
    let zero = |v: u64| !(((v & LOW7).wrapping_add(LOW7)) | v | LOW7);
    let splat = few.map(|b| ONES.wrapping_mul(u64::from(b)));
    let mut mask = 0u64;
    for (w, chunk) in block.as_chunks::<8>().0.iter().enumerate() {
        let v = u64::from_le_bytes(*chunk);
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
    use super::Tables;
    use core::arch::x86_64::*;

    /// The block's bits, 32 bytes a compare: `each` makes the compare of one
    /// 32-byte part, a byte of all ones where it is in the set.
    #[inline]
    #[target_feature(enable = "avx2")]
    fn bits<const L: usize>(block: &[u8; L], each: impl Fn(__m256i) -> __m256i) -> u64 {
        let mut mask = 0u64;
        for (i, part) in block.as_chunks::<32>().0.iter().enumerate() {
            // SAFETY: the load reads the 32 bytes of `part`, unaligned.
            let v = unsafe { _mm256_loadu_si256(part.as_ptr().cast::<__m256i>()) };
            mask |= u64::from(_mm256_movemask_epi8(each(v)) as u32) << (32 * i);
        }
        mask
    }

    #[inline]
    #[target_feature(enable = "avx2")]
    pub(super) fn high<const L: usize>(lanes: &[u8; L]) -> u64 {
        bits(lanes, |v| v)
    }

    #[inline]
    #[target_feature(enable = "avx2")]
    pub(super) fn any<const L: usize, const N: usize>(block: &[u8; L], few: [u8; N]) -> u64 {
        let splat = few.map(|m| _mm256_set1_epi8(m as i8));
        bits(block, |v| {
            let mut hits = _mm256_setzero_si256();
            for s in splat {
                hits = _mm256_or_si256(hits, _mm256_cmpeq_epi8(v, s));
            }
            hits
        })
    }

    #[inline]
    #[target_feature(enable = "avx2")]
    fn table(t: &[u8; 16]) -> __m256i {
        // SAFETY: the load reads the 16 bytes of `t`, unaligned.
        _mm256_broadcastsi128_si256(unsafe { _mm_loadu_si128(t.as_ptr().cast::<__m128i>()) })
    }

    /// [`super::few_or_tables`]: the bytes in the set, a byte of all ones
    /// each, chosen per part, then one mask.
    #[inline]
    #[target_feature(enable = "avx2")]
    pub(super) fn few_or_tables<const L: usize>(
        block: &[u8; L],
        few: [u8; 3],
        n: usize,
        tables: &Tables,
    ) -> u64 {
        let splat = few.map(|m| _mm256_set1_epi8(m as i8));
        let nibble = _mm256_set1_epi8(0x0f);
        bits(block, |v| match n {
            1 => _mm256_cmpeq_epi8(v, splat[0]),
            2 => _mm256_or_si256(
                _mm256_cmpeq_epi8(v, splat[0]),
                _mm256_cmpeq_epi8(v, splat[1]),
            ),
            3 => _mm256_or_si256(
                _mm256_or_si256(
                    _mm256_cmpeq_epi8(v, splat[0]),
                    _mm256_cmpeq_epi8(v, splat[1]),
                ),
                _mm256_cmpeq_epi8(v, splat[2]),
            ),
            _ => {
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
                // Outside the set where no table bit is left; inverted.
                _mm256_xor_si256(
                    _mm256_cmpeq_epi8(hits, _mm256_setzero_si256()),
                    _mm256_set1_epi8(-1),
                )
            }
        })
    }

    #[inline]
    #[target_feature(enable = "avx2")]
    pub(super) fn tables<const L: usize>(block: &[u8; L], tables: &Tables) -> u64 {
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
        // The compare finds the bytes outside the set; the bits past `L`
        // stay clear.
        !bits(block, lookup) & (u64::MAX >> (64 - L))
    }
}

#[cfg(all(target_arch = "x86_64", not(target_feature = "avx2")))]
mod sse2 {
    use core::arch::x86_64::*;

    /// The block's bits, 16 bytes a compare: `each` makes the compare of one
    /// 16-byte part, a byte of all ones where it is in the set.
    #[inline]
    #[target_feature(enable = "sse2")]
    fn bits<const L: usize>(block: &[u8; L], each: impl Fn(__m128i) -> __m128i) -> u64 {
        let mut mask = 0u64;
        for (i, part) in block.as_chunks::<16>().0.iter().enumerate() {
            // SAFETY: the load reads the 16 bytes of `part`, unaligned.
            let v = unsafe { _mm_loadu_si128(part.as_ptr().cast::<__m128i>()) };
            mask |= u64::from(_mm_movemask_epi8(each(v)) as u16) << (16 * i);
        }
        mask
    }

    #[inline]
    #[target_feature(enable = "sse2")]
    pub(super) fn any<const L: usize, const N: usize>(block: &[u8; L], few: [u8; N]) -> u64 {
        let splat = few.map(|m| _mm_set1_epi8(m as i8));
        bits(block, |v| {
            let mut hits = _mm_setzero_si128();
            for s in splat {
                hits = _mm_or_si128(hits, _mm_cmpeq_epi8(v, s));
            }
            hits
        })
    }

    #[cfg(target_feature = "ssse3")]
    #[inline]
    #[target_feature(enable = "ssse3")]
    pub(super) fn tables<const L: usize>(block: &[u8; L], tables: &super::Tables) -> u64 {
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
        !bits(block, lookup) & (u64::MAX >> (64 - L))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BLOCK, HALF, Tables, any_of, by_map, by_words, few_or_tables, high_bits, in_tables,
    };

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

    /// `high_bits` sets bit `i` exactly where byte `i` is at least 128, at
    /// each of its widths.
    #[test]
    fn high_bits_are_the_top_bits() {
        let mut seed = 5u64;
        for _ in 0..64 {
            let b = block(&mut seed);
            let top = |n: usize| (0..n).fold(0u64, |m, i| m | (u64::from(b[i] >> 7) << i));
            assert_eq!(high_bits::<64>(&b), top(64));
            assert_eq!(high_bits::<32>(b[..32].try_into().unwrap()), top(32));
            assert_eq!(high_bits::<16>(b[..16].try_into().unwrap()), top(16));
        }
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
                // Half a block finds what the block's first half does.
                let half = b.first_chunk::<HALF>().unwrap();
                assert_eq!(in_tables(half, &tables), want & 0xFFFF_FFFF, "{members:?}");
                assert_eq!(by_map(half, &tables), want & 0xFFFF_FFFF, "{members:?}");
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
                    let three = [few[0], few[1], few[2]];
                    assert_eq!(few_or_tables(&b, three, 3, &tables), want);
                    let t1 = Tables::new(map_of(&one));
                    assert_eq!(few_or_tables(&b, three, 1, &t1), any_of(&b, one));
                    assert_eq!(
                        few_or_tables(&b, [two[0], two[1], 0], 2, &tables),
                        any_of(&b, two)
                    );
                }
                // With no member compared directly, the tables answer.
                let want = in_tables(&b, &tables);
                assert_eq!(few_or_tables(&b, [0; 3], 0, &tables), want, "{members:?}");
                let half = b.first_chunk::<HALF>().unwrap();
                assert_eq!(few_or_tables(half, [0; 3], 0, &tables), want & 0xFFFF_FFFF);
            }
        }
        for few in [[0u8, 0x80, 0xFF], *b";\n\n", [7, 7, 7]] {
            for _ in 0..rounds {
                let b = block(&mut seed);
                let half = b.first_chunk::<HALF>().unwrap();
                let want = by_map(&b, &Tables::new(map_of(&few))) & 0xFFFF_FFFF;
                assert_eq!(any_of(half, few), want);
                assert_eq!(by_words(half, few), want);
                assert_eq!(in_tables(half, &Tables::new(map_of(&few))), want);
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
