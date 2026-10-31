//! The hash a map gets when nobody outside the program chose its keys.
//!
//! [ADR-010](../../../docs/specification/adr/adr-010.md) D5: provenance selects
//! a hash function and nothing else. Untrusted keys get the keyed, randomly
//! seeded hash that `std` gives every map by default; trusted keys get this one
//! - fast, fixed seed, not cryptographic and not trying to be.
//!
//! Two shapes of key reach it by two paths, and each has the function that was
//! measured best for it ([ADR-316](../../../docs/specification/adr/adr-316.md)
//! D5). A number, a `char`, a `bool` arrive through `write_u64` and its
//! siblings and are one Fx step each: the hash `rustc` used on its own tables
//! for years. Text arrives through `write`, at any length, and is first folded
//! into one word by the byte hash of `rustc-hash` 2 (`hash_bytes` below), which
//! reads a short key as two overlapping words instead of branching on its
//! length piece by piece. On 1BRC's names that halved the mispredictions a row
//! (3.42 to 1.62) and took 19 instructions off it; on `k-nucleotide`'s keys of
//! one length it cut 7 % of the instructions.
//!
//! It is here rather than behind a dependency for two reasons: it sits on the
//! hot path of every Nikaia program that aggregates anything, and a program
//! whose *hash function* comes from a crate it did not choose is a program
//! whose iteration order and worst case can change under it.
//!
//! What it is not: a defence against chosen keys. That is exactly why the
//! compiler picks between the two rather than making this the default, and why
//! a barrier in the provenance analysis widens to untrusted rather than to
//! here.

use std::hash::{BuildHasherDefault, Hasher};

/// **A map**: `std`'s own type over `hashbrown`'s table (`map.rs`,
/// ADR-296 D42), with the keyed hasher unless the type names another (#420).
/// `std`'s `HashMap` is the same table behind a wrapper whose `entry` reserves
/// a slot on every call, found or not; this one does not, and it compares and
/// hashes a text key a word at a time.
///
/// A map is made with `default()`: the type's default fills the hasher in
/// where the type is not written.
pub use crate::map::{Entry, HashMap};

/// A `HashMap` for keys the operator chose.
pub type TrustedMap<K, V> = HashMap<K, V, Fast>;

/// A `HashSet` for keys the operator chose.
pub type TrustedSet<K> = std::collections::HashSet<K, BuildHasherDefault<FxHasher>>;

/// **The fast hasher of a map position** (ADR-010 D9): what `TrustedMap` and
/// `TrustedSet` are written with, and the name Rust code gives a hidden type
/// parameter of a `pub` Nikaia item to vouch for its keys.
pub type Fast = BuildHasherDefault<FxHasher>;

/// **The keyed hasher of a map position** (D9): the randomly seeded one `std`
/// gives every map. It is the default of every hidden type parameter, so Rust
/// code that names a `pub` type holding maps gets the map that is safe for any
/// key.
pub type Keyed = std::hash::RandomState;

/// **What a hidden type parameter of a map position is bounded by** (D9): any
/// hasher a map can be made with. `Fast` and `Keyed` are the two the compiler
/// writes.
pub trait Seed: std::hash::BuildHasher + Default + Clone {}

impl<S: std::hash::BuildHasher + Default + Clone> Seed for S {}

/// The constant `rustc`'s hash multiplies by: the 64-bit odd number closest to
/// `2^64 / φ`, so that the multiply spreads a change in any bit across the
/// whole word.
const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

/// A fast, non-cryptographic hash with a fixed seed.
///
/// One multiply and one rotate per number written, and one more for text once
/// `hash_bytes` has made a word of it; no allocation, no state beyond the
/// accumulator. Every write folds into the same accumulator, so the order of
/// the parts of a compound key matters - which is what a hash of a tuple has to
/// promise.
#[derive(Default, Clone)]
pub struct FxHasher {
    hash: u64,
}

impl FxHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.add(hash_bytes(bytes));
    }

    #[inline]
    fn write_u8(&mut self, n: u8) {
        self.add(n as u64);
    }

    #[inline]
    fn write_u16(&mut self, n: u16) {
        self.add(n as u64);
    }

    #[inline]
    fn write_u32(&mut self, n: u32) {
        self.add(n as u64);
    }

    #[inline]
    fn write_u64(&mut self, n: u64) {
        self.add(n);
    }

    #[inline]
    fn write_usize(&mut self, n: usize) {
        self.add(n as u64);
    }

    /// The multiply leaves the most entropy in the top bits, but a table picks
    /// the bucket from the bottom ones: the rotation brings the top down, as
    /// `rustc-hash` 2 does, so keys on a power-of-two stride (`i * 1024`,
    /// `i << 32`) do not crowd into a few buckets.
    #[inline]
    fn finish(&self) -> u64 {
        self.hash.rotate_left(26)
    }
}

// `hash_bytes` and what it calls are `rustc-hash` 2.1.1's (`src/lib.rs`),
// MIT or Apache-2.0, by Orson Peters; the 64-bit half only, since `nikaia-std`
// targets 64-bit. Copied rather than depended on, for the reasons at the top.

// Digits of pi.
const SEED1: u64 = 0x243f_6a88_85a3_08d3;
const SEED2: u64 = 0x1319_8a2e_0370_7344;
const PREVENT_TRIVIAL_ZERO_COLLAPSE: u64 = 0xa409_3822_299f_31d0;

/// The full 64 x 64 -> 128 product, its halves xor-ed: the middle bits, which
/// move most with a small change in either input, end up everywhere.
#[inline]
fn multiply_mix(x: u64, y: u64) -> u64 {
    let full = (x as u128) * (y as u128);
    (full as u64) ^ ((full >> 64) as u64)
}

#[inline]
fn word(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes[..8].try_into().expect("eight"))
}

#[inline]
fn half(bytes: &[u8]) -> u64 {
    u32::from_le_bytes(bytes[..4].try_into().expect("four")) as u64
}

/// Any number of bytes as one word.
///
/// Up to 16 bytes, the key is read as its first and its last 8 (or 4) bytes,
/// which overlap where the key is shorter than both: one branch on the length's
/// class rather than one per piece of the tail. Longer keys are read 16 bytes a
/// step, and the last 16 overlap the step before. The length is xor-ed in at
/// the end, so two lengths of one overlapping pattern do not collide.
///
/// **Out of line on purpose** (#695): a map's `entry` with this inlined into
/// it is too big for LLVM to inline into a fold's loop, and whether it was
/// inlined depended on how the codegen units were cut - 1BRC ran 274
/// instructions a row at one unit and 231 at sixteen, where the hash happened
/// to stay a call. Kept a call, the table's `entry` is inlined at both: 244 and
/// 223, for the cost of one call a key.
#[inline(never)]
fn hash_bytes(bytes: &[u8]) -> u64 {
    if bytes.len() <= 16 {
        hash_short(bytes)
    } else {
        hash_long(bytes)
    }
}

/// [`hash_bytes`] of up to 16 bytes, inline: the first and the last word.
#[inline(always)]
fn hash_short(bytes: &[u8]) -> u64 {
    let len = bytes.len();
    let mut s0 = SEED1;
    let mut s1 = SEED2;
    if len >= 8 {
        s0 ^= word(bytes);
        s1 ^= word(&bytes[len - 8..]);
    } else if len >= 4 {
        s0 ^= half(bytes);
        s1 ^= half(&bytes[len - 4..]);
    } else if len > 0 {
        s0 ^= bytes[0] as u64;
        s1 ^= ((bytes[len - 1] as u64) << 8) | bytes[len / 2] as u64;
    }
    multiply_mix(s0, s1) ^ len as u64
}

/// **What [`FxHasher`] gives a text** (`str`'s `Hash`: the bytes, then the
/// byte `0xff`), worked out without a hasher: the map's hash of a text key
/// under the fast hasher (`map.rs`). A key of up to 16 bytes is hashed inline,
/// a longer one by the call.
#[inline(always)]
pub(crate) fn fx_text(bytes: &[u8]) -> u64 {
    let word = if bytes.len() <= 16 {
        hash_short(bytes)
    } else {
        hash_long(bytes)
    };
    let mut h = FxHasher::default();
    h.add(word);
    h.add(0xff);
    h.finish()
}

/// [`hash_bytes`] of more than 16 bytes.
#[inline(never)]
fn hash_long(bytes: &[u8]) -> u64 {
    let len = bytes.len();
    let mut s0 = SEED1;
    let mut s1 = SEED2;
    {
        let mut off = 0;
        while off < len - 16 {
            let x = word(&bytes[off..]);
            let y = word(&bytes[off + 8..]);
            // Two independent streams, s0 and s1, so the loop unrolls; the
            // constant keeps a run of zeros from collapsing the state.
            let t = multiply_mix(s0 ^ x, PREVENT_TRIVIAL_ZERO_COLLAPSE ^ y);
            s0 = s1;
            s1 = t;
            off += 16;
        }
        let suffix = &bytes[len - 16..];
        s0 ^= word(suffix);
        s1 ^= word(&suffix[8..]);
    }
    multiply_mix(s0, s1) ^ len as u64
}

/// **One CRC-32C step over a word** (ADR-371 D5): the CRC-32C of `word`'s
/// eight bytes, lowest first, continued from `crc`, without the inversions a
/// checksum adds before and after - the step SSE 4.2's `crc32` and ARMv8's
/// `crc32cx` take. One instruction where the build has SSE 4.2; a table
/// eight bytes at a time everywhere else, with the same value.
#[inline(always)]
pub fn crc32c(crc: u32, word: u64) -> u32 {
    #[cfg(all(target_arch = "x86_64", target_feature = "sse4.2"))]
    {
        crc32c::u64(crc, word)
    }
    #[cfg(not(all(target_arch = "x86_64", target_feature = "sse4.2")))]
    {
        crc32c_by_table(crc, word)
    }
}

/// The reflected Castagnoli polynomial.
const CASTAGNOLI: u32 = 0x82F6_3B78;

/// `CRC32C_TABLES[k][b]`: the CRC of byte `b` followed by `k` zero bytes, so
/// that eight bytes are eight lookups (slicing by eight).
const CRC32C_TABLES: [[u32; 256]; 8] = {
    let mut tables = [[0u32; 256]; 8];
    let mut b = 0;
    while b < 256 {
        let mut crc = b as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = (crc >> 1) ^ (CASTAGNOLI & (crc & 1).wrapping_neg());
            bit += 1;
        }
        tables[0][b] = crc;
        b += 1;
    }
    let mut k = 1;
    while k < 8 {
        let mut b = 0;
        while b < 256 {
            let before = tables[k - 1][b];
            tables[k][b] = (before >> 8) ^ tables[0][(before & 0xFF) as usize];
            b += 1;
        }
        k += 1;
    }
    tables
};

/// [`crc32c`] without the instruction.
#[cfg_attr(
    all(target_arch = "x86_64", target_feature = "sse4.2"),
    allow(dead_code)
)]
#[inline(always)]
fn crc32c_by_table(crc: u32, word: u64) -> u32 {
    let x = word ^ u64::from(crc);
    let t = &CRC32C_TABLES;
    let byte = |i: u32| ((x >> (8 * i)) & 0xFF) as usize;
    t[7][byte(0)]
        ^ t[6][byte(1)]
        ^ t[5][byte(2)]
        ^ t[4][byte(3)]
        ^ t[3][byte(4)]
        ^ t[2][byte(5)]
        ^ t[1][byte(6)]
        ^ t[0][byte(7)]
}

/// **The SHA-256 digest of `data` as 64 lowercase hex characters** (FIPS
/// 180-4), the same text `orchestrator::cache::sha256_hex` writes. Written
/// out here rather than taken from `sha2`, for the reason `hash_bytes` copies
/// rustc-hash: `std` is built into every program, and its dependency graph
/// stays small (ADR-002 D4).
pub fn sha256_hex(data: &[u8]) -> String {
    let mut state = SHA256_START;
    let mut blocks = data.chunks_exact(64);
    for block in &mut blocks {
        sha256_block(&mut state, block);
    }
    // The padding: one 1 bit, zeros, then the message's length in bits as a
    // big-endian `u64`, so that the last block (or two) ends on 64 bytes.
    let rest = blocks.remainder();
    let mut tail = [0u8; 128];
    tail[..rest.len()].copy_from_slice(rest);
    tail[rest.len()] = 0x80;
    let end = if rest.len() < 56 { 64 } else { 128 };
    let bits = (data.len() as u64).wrapping_mul(8);
    tail[end - 8..end].copy_from_slice(&bits.to_be_bytes());
    for block in tail[..end].chunks_exact(64) {
        sha256_block(&mut state, block);
    }
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for word in state {
        for b in word.to_be_bytes() {
            out.push(DIGITS[(b >> 4) as usize] as char);
            out.push(DIGITS[(b & 15) as usize] as char);
        }
    }
    out
}

/// SHA-256's initial hash value (FIPS 180-4 5.3.3).
const SHA256_START: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// SHA-256's 64 round constants (FIPS 180-4 4.2.2).
const SHA256_ROUNDS: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256's compression function over one 64-byte block (FIPS 180-4 6.2.2).
fn sha256_block(state: &mut [u32; 8], block: &[u8]) {
    let mut w = [0u32; 64];
    for (t, word) in block.chunks_exact(4).enumerate() {
        w[t] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
    }
    for t in 16..64 {
        let s0 = w[t - 15].rotate_right(7) ^ w[t - 15].rotate_right(18) ^ (w[t - 15] >> 3);
        let s1 = w[t - 2].rotate_right(17) ^ w[t - 2].rotate_right(19) ^ (w[t - 2] >> 10);
        w[t] = w[t - 16]
            .wrapping_add(s0)
            .wrapping_add(w[t - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for (k, wt) in SHA256_ROUNDS.iter().zip(w) {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(*k)
            .wrapping_add(wt);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (slot, v) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *slot = slot.wrapping_add(v);
    }
}

#[cfg(test)]
mod tests {
    /// The definition, a bit at a time.
    fn crc32c_slow(mut crc: u32, word: u64) -> u32 {
        for byte in word.to_le_bytes() {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (super::CASTAGNOLI & (crc & 1).wrapping_neg());
            }
        }
        crc
    }

    /// The table and the build's own path against the definition, and
    /// CRC-32C's check value: "123456789" is 0xE3069283.
    #[test]
    fn crc32c_is_the_castagnoli_crc() {
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        for _ in 0..10_000 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let (crc, word) = (seed as u32, seed.rotate_left(29));
            assert_eq!(super::crc32c_by_table(crc, word), crc32c_slow(crc, word));
            assert_eq!(super::crc32c(crc, word), crc32c_slow(crc, word));
        }
        let crc = super::crc32c(!0, u64::from_le_bytes(*b"12345678"));
        let crc = (crc >> 8) ^ super::CRC32C_TABLES[0][((crc ^ u32::from(b'9')) & 0xFF) as usize];
        assert_eq!(!crc, 0xE306_9283);
    }

    use super::*;
    use std::collections::HashMap;

    fn hash_of(bytes: &[u8]) -> u64 {
        let mut h = FxHasher::default();
        h.write(bytes);
        h.finish()
    }

    /// A hash function is only useful if the same key hashes the same way every
    /// time, in the same process and the next one. That is what "fixed seed"
    /// means and it is the whole difference from the map `std` gives by
    /// default.
    #[test]
    fn the_same_key_hashes_the_same_way() {
        assert_eq!(hash_of(b"Hamburg"), hash_of(b"Hamburg"));
        assert_ne!(hash_of(b"Hamburg"), hash_of(b"Bulawayo"));
    }

    /// Order matters inside a key: `ab` is not `ba`, and a compound key's parts
    /// do not commute.
    #[test]
    fn the_order_of_the_bytes_matters() {
        assert_ne!(hash_of(b"ab"), hash_of(b"ba"));

        let mut first = FxHasher::default();
        first.write_u32(1);
        first.write_u32(2);
        let mut second = FxHasher::default();
        second.write_u32(2);
        second.write_u32(1);
        assert_ne!(first.finish(), second.finish());
    }

    /// Every length is reached: a key one byte longer hashes differently,
    /// across the short reads (1-3, 4-7, 8-16 bytes) and the 16-byte steps.
    /// The empty key and the single zero byte differ too, since the length is
    /// mixed in. Not injectivity - a 64-bit non-cryptographic hash does not
    /// promise that, and a map compares keys in full (ADR-010 D5).
    #[test]
    fn every_length_is_covered() {
        let mut seen = std::collections::HashSet::new();
        for len in 0..80usize {
            let key: Vec<u8> = (0..len).map(|i| (i % 251 + 1) as u8).collect();
            assert!(seen.insert(hash_of(&key)), "collision at length {len}");
        }
        assert_ne!(hash_of(b""), hash_of(&[0]));
    }

    /// The overlapping reads see one pattern at two lengths, and still tell
    /// them apart: a run of one byte, every length up to 40.
    #[test]
    fn a_repeated_byte_at_every_length_differs() {
        let mut seen = std::collections::HashSet::new();
        for len in 0..40usize {
            assert!(seen.insert(hash_of(&vec![b'a'; len])), "length {len}");
        }
    }

    /// The copy answers as `rustc-hash` 2 does, read back out of its hasher:
    /// from a zero state, one `write` leaves `hash_bytes(b) * K` rotated left
    /// by 26, so undoing the rotation and the multiply gives `hash_bytes(b)`.
    #[test]
    fn hash_bytes_is_the_one_copied() {
        const K: u64 = 0xf135_7aea_2e62_a9c5;
        // K is odd, so it has an inverse mod 2^64: Newton's iteration doubles
        // the correct bits each step, from the 3 that `K` itself gets right.
        let mut inverse = K;
        for _ in 0..5 {
            inverse = inverse.wrapping_mul(2u64.wrapping_sub(K.wrapping_mul(inverse)));
        }
        assert_eq!(K.wrapping_mul(inverse), 1);
        for len in 0..80usize {
            let key: Vec<u8> = (0..len).map(|i| (i * 37 % 256) as u8).collect();
            let mut theirs = rustc_hash::FxHasher::default();
            theirs.write(&key);
            let back = theirs.finish().rotate_right(26).wrapping_mul(inverse);
            assert_eq!(back, hash_bytes(&key), "length {len}");
        }
    }

    /// Integer keys on a power-of-two stride spread over the buckets a
    /// 4096-bucket table picks from the low 12 bits: 2941 of them for
    /// `i * 1024` and 1985 for `i << 32` (random keys reach about 2589), where
    /// without the rotation in `finish` they share 4 and 1. Keyed again, they
    /// hash the same way.
    #[test]
    fn strided_integers_spread_over_the_low_bits() {
        let hash_u64 = |n: u64| {
            let mut h = FxHasher::default();
            h.write_u64(n);
            h.finish()
        };
        for (name, key) in [
            ("i * 1024", (|i: u64| i * 1024) as fn(u64) -> u64),
            ("i << 32", |i: u64| i << 32),
        ] {
            let buckets: std::collections::HashSet<u64> =
                (0..4096u64).map(|i| hash_u64(key(i)) & 0xfff).collect();
            assert!(
                buckets.len() >= 4096 / 4,
                "{name}: {} buckets",
                buckets.len()
            );
            for i in 0..4096u64 {
                assert_eq!(hash_u64(key(i)), hash_u64(key(i)), "{name}");
            }
        }
    }

    /// What it is for: a table that answers, over the keys a data format
    /// actually has.
    #[test]
    fn it_works_as_a_map_hasher() {
        let mut map: TrustedMap<&str, i32> = TrustedMap::default();
        for (i, name) in ["Hamburg", "Bulawayo", "Palembang", "St. John's", "東京"]
            .iter()
            .enumerate()
        {
            map.insert(name, i as i32);
        }
        assert_eq!(map.len(), 5);
        assert_eq!(map["東京"], 4);
        assert_eq!(map.get("nowhere"), None);

        // …and it agrees with the map it stands in for.
        let plain: HashMap<&str, i32> = map.iter().map(|(k, v)| (*k, *v)).collect();
        assert_eq!(plain.len(), map.len());
    }

    /// The keys of a real aggregation, spread over a table of the size one
    /// would have: no bucket may hold a tenth of them.
    #[test]
    fn it_spreads_the_keys_of_an_aggregation() {
        let keys: Vec<String> = (0..413).map(|i| format!("Station{i:03}")).collect();
        let buckets = 1024u64;
        let mut counts = vec![0usize; buckets as usize];
        for key in &keys {
            counts[(hash_of(key.as_bytes()) % buckets) as usize] += 1;
        }
        let worst = counts.iter().copied().max().expect("non-empty");
        assert!(worst < keys.len() / 10, "worst bucket held {worst}");
    }
}
