//! **A word with only its lowest bits kept, as one `bzhi`** where the build's
//! CPU level has BMI2 (`--cpu=x86-64-v3`;
//! [ADR-371](../../../docs/specification/adr/adr-371.md) D2: an instruction
//! of one architecture is a safe function of the backend).
//!
//! The caller is `nikaia-std`'s `num::LowBits` (`x.low_bits(n)`, Part I),
//! which calls [`u64`] and [`u32`] where `cfg` says the build enables BMI2
//! and keeps its own portable form everywhere else. `bzhi` reads the low
//! eight bits of its index and keeps every bit where the index is at least
//! the width, so an index of type `u8` is total: the caller saturates its
//! count to `0..=255` first.
//!
//! The `unsafe` is the call of a `#[target_feature]` function: `core::arch`
//! asks for it from a function that does not list the feature, even where
//! the build enables it for every function. Every `unsafe` here, and the
//! argument for it, is listed in `README.md`.

#![deny(unsafe_op_in_unsafe_fn)]
#![no_std]

#[cfg(all(target_arch = "x86_64", any(test, target_feature = "bmi2")))]
use core::arch::x86_64::{_bzhi_u32, _bzhi_u64};

/// `x` with only its `index` lowest bits kept; all of `x` where `index` is
/// 64 or more.
#[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))]
#[inline(always)]
pub fn u64(x: u64, index: u8) -> u64 {
    // SAFETY: the `cfg` above says the build enables BMI2 for every function.
    unsafe { bzhi64(x, index) }
}

/// `x` with only its `index` lowest bits kept; all of `x` where `index` is
/// 32 or more.
#[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))]
#[inline(always)]
pub fn u32(x: u32, index: u8) -> u32 {
    // SAFETY: as in `u64`.
    unsafe { bzhi32(x, index) }
}

/// The instruction itself, so that a test can reach it where only the host
/// (detected at run time) has BMI2, not the build.
#[cfg(all(target_arch = "x86_64", any(test, target_feature = "bmi2")))]
#[target_feature(enable = "bmi2")]
#[inline]
fn bzhi64(x: u64, index: u8) -> u64 {
    _bzhi_u64(x, core::primitive::u32::from(index))
}

#[cfg(all(target_arch = "x86_64", any(test, target_feature = "bmi2")))]
#[target_feature(enable = "bmi2")]
#[inline]
fn bzhi32(x: u32, index: u8) -> u32 {
    _bzhi_u32(x, core::primitive::u32::from(index))
}

#[cfg(all(test, target_arch = "x86_64"))]
mod tests {
    extern crate std;

    /// The definition: the `n` lowest bits, none at `n <= 0`, all at the
    /// width and past it.
    fn keep(x: u64, n: i64, width: i64) -> u64 {
        if n <= 0 {
            0
        } else if n >= width {
            x
        } else {
            x & ((1u64 << n) - 1)
        }
    }

    /// Every index against the definition, where the host has BMI2 (Miri
    /// and older hosts skip it; the build's own path is checked below).
    #[test]
    fn bzhi_keeps_the_lowest_bits() {
        if !std::is_x86_feature_detected!("bmi2") {
            return;
        }
        let a: u64 = 0x0123_4567_89ab_cdef;
        for index in 0..=255u8 {
            let n = i64::from(index);
            // SAFETY: the host has BMI2, checked above.
            let (wide, narrow) =
                unsafe { (super::bzhi64(a, index), super::bzhi32(a as u32, index)) };
            assert_eq!(wide, keep(a, n, 64), "u64 {index}");
            assert_eq!(
                u64::from(narrow),
                keep(u64::from(a as u32), n, 32),
                "u32 {index}"
            );
        }
    }

    /// The safe functions, where the build enables BMI2
    /// (`RUSTFLAGS="-C target-cpu=x86-64-v3" cargo test`).
    #[cfg(target_feature = "bmi2")]
    #[test]
    fn the_build_path_keeps_the_lowest_bits() {
        let a: u64 = 0x0123_4567_89ab_cdef;
        for index in 0..=255u8 {
            let n = i64::from(index);
            assert_eq!(super::u64(a, index), keep(a, n, 64));
            assert_eq!(
                u64::from(super::u32(a as u32, index)),
                keep(u64::from(a as u32), n, 32)
            );
        }
    }
}
