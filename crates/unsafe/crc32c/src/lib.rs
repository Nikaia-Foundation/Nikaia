//! **One CRC-32C step over a word, as one `crc32`** where the build's CPU
//! level has SSE 4.2 (`--cpu=x86-64-v3`;
//! [ADR-371](../../../docs/specification/adr/adr-371.md) D2: an instruction
//! of one architecture is a safe function of the backend).
//!
//! The caller is `nikaia-std`'s `hash::crc32c`, which calls [`u64`] where
//! `cfg` says the build enables SSE 4.2 and keeps its own portable form
//! everywhere else; the two give the same value.
//!
//! The `unsafe` is the call of a `#[target_feature]` function: `core::arch`
//! asks for it from a function that does not list the feature, even where
//! the build enables it for every function. Every `unsafe` here, and the
//! argument for it, is listed in `README.md`.

#![deny(unsafe_op_in_unsafe_fn)]
#![no_std]

#[cfg(all(target_arch = "x86_64", any(test, target_feature = "sse4.2")))]
use core::arch::x86_64::_mm_crc32_u64;

/// The CRC-32C of `word`'s eight bytes, lowest first, continued from `crc`,
/// without the inversions a checksum adds before and after.
#[cfg(all(target_arch = "x86_64", target_feature = "sse4.2"))]
#[inline(always)]
pub fn u64(crc: u32, word: u64) -> u32 {
    // SAFETY: the `cfg` above says the build enables SSE 4.2 for every
    // function.
    unsafe { step(crc, word) }
}

/// The instruction itself, so that a test can reach it where only the host
/// (detected at run time) has SSE 4.2, not the build.
#[cfg(all(target_arch = "x86_64", any(test, target_feature = "sse4.2")))]
#[target_feature(enable = "sse4.2")]
#[inline]
fn step(crc: u32, word: u64) -> u32 {
    // The instruction's result has its high half zero.
    _mm_crc32_u64(core::primitive::u64::from(crc), word) as u32
}

#[cfg(all(test, target_arch = "x86_64"))]
mod tests {
    extern crate std;

    /// The definition, a bit at a time: the reflected Castagnoli polynomial.
    fn slow(mut crc: u32, word: u64) -> u32 {
        for byte in word.to_le_bytes() {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0x82F6_3B78 & (crc & 1).wrapping_neg());
            }
        }
        crc
    }

    fn words() -> impl Iterator<Item = (u32, u64)> {
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        (0..10_000).map(move |_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed as u32, seed.rotate_left(29))
        })
    }

    /// The instruction against the definition, where the host has SSE 4.2
    /// (Miri and older hosts skip it; the build's own path is checked below).
    #[test]
    fn the_instruction_is_the_definition() {
        if !std::is_x86_feature_detected!("sse4.2") {
            return;
        }
        // CRC-32C's check value: "123456789" with both inversions.
        let nine = u64::from_le_bytes(*b"12345678");
        // SAFETY: the host has SSE 4.2, checked above.
        let eight = unsafe { super::step(!0, nine) };
        assert_eq!(slow(!0, nine), eight);
        for (crc, word) in words() {
            // SAFETY: as above.
            assert_eq!(unsafe { super::step(crc, word) }, slow(crc, word));
        }
    }

    /// The safe function, where the build enables SSE 4.2
    /// (`RUSTFLAGS="-C target-cpu=x86-64-v3" cargo test`).
    #[cfg(target_feature = "sse4.2")]
    #[test]
    fn the_build_path_is_the_definition() {
        for (crc, word) in words() {
            assert_eq!(super::u64(crc, word), slow(crc, word));
        }
    }
}
