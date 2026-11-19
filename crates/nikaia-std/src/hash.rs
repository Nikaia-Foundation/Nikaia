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
//! into one word by the byte hash of `rustc-hash` 2 (`hash_bytes`, written in
//! Nikaia in `runtime/hash.nika`), which
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

use crate::runtime::hash::{fx_add, fx_finish, hash_bytes, hash_long, hash_short};
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

/// A fast, non-cryptographic hash with a fixed seed: the shell Rust's
/// `Hasher` asks for, around the steps written in Nikaia
/// (`runtime/hash.nika`: `fx_add`, `fx_finish`, `hash_bytes`).
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
        self.hash = fx_add(self.hash, word);
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

    /// The rotation that brings the multiply's top bits down to the ones a
    /// table picks its bucket from (`fx_finish`).
    #[inline]
    fn finish(&self) -> u64 {
        fx_finish(self.hash)
    }
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
    fx_finish(fx_add(fx_add(0, word), 0xff))
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
/// out here rather than taken from `sha2`, for the reason `runtime/hash.nika`
/// copies rustc-hash: `std` is built into every program, and its dependency graph
/// stays small (ADR-002 D4).
pub fn sha256_hex(data: &[u8]) -> crate::block::Text {
    let mut state = SHA256_START;
    let (blocks, rest) = data.as_chunks::<64>();
    for block in blocks {
        sha256_block(&mut state, block);
    }
    // The padding: one 1 bit, zeros, then the message's length in bits as a
    // big-endian `u64`, so that the last block (or two) ends on 64 bytes.
    let mut tail = [0u8; 128];
    tail[..rest.len()].copy_from_slice(rest);
    tail[rest.len()] = 0x80;
    let end = if rest.len() < 56 { 64 } else { 128 };
    let bits = (data.len() as u64).wrapping_mul(8);
    tail[end - 8..end].copy_from_slice(&bits.to_be_bytes());
    for block in tail[..end].as_chunks::<64>().0 {
        sha256_block(&mut state, block);
    }
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = crate::block::Text::with_capacity(64);
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
fn sha256_block(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for (t, word) in block.as_chunks::<4>().0.iter().enumerate() {
        w[t] = u32::from_be_bytes(*word);
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
