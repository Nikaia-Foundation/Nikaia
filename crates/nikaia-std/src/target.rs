//! **What the CPU level a program is built for has**
//! ([ADR-371](../../../docs/specification/adr/adr-371.md) D6).
//!
//! `target::has(target::Instruction::Crc32c)` answers for the level the build names
//! (`--cpu`), not for the machine the program runs on. A program compiled at
//! that level reads it off its own compilation (`cfg!(target_feature)`); a
//! build-time run is compiled for the machine that builds, so the compiler
//! passes the level in [`LEVEL_VAR`] and the answer is read from that.

/// The variable the compiler hands a build-time run: the build's CPU level
/// (`x86-64-v3`, `native`), empty or absent for the baseline.
pub const LEVEL_VAR: &str = "NIKAIA_COMPTIME_CPU";

/// An instruction a CPU level may have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Instruction {
    /// CRC-32C of a word: SSE 4.2 on x86-64, `crc` on AArch64.
    Crc32c,
    /// x86-64's BMI2 (`pext`, `pdep`, shifts without flags).
    Bmi2,
    /// x86-64's 256-bit integer vectors.
    Avx2,
}

/// **Whether the CPU level the program is built for has `i`**.
#[inline]
pub fn has(i: Instruction) -> bool {
    if crate::build_time::running() {
        return at_level(&std::env::var(LEVEL_VAR).unwrap_or_default(), i);
    }
    compiled_with(i)
}

/// What the level this code was compiled at has.
#[inline]
fn compiled_with(i: Instruction) -> bool {
    match i {
        Instruction::Crc32c => {
            cfg!(all(target_arch = "x86_64", target_feature = "sse4.2"))
                || cfg!(all(target_arch = "aarch64", target_feature = "crc"))
        }
        Instruction::Bmi2 => cfg!(all(target_arch = "x86_64", target_feature = "bmi2")),
        Instruction::Avx2 => cfg!(all(target_arch = "x86_64", target_feature = "avx2")),
    }
}

/// What the named level has; `native` is this machine's.
fn at_level(level: &str, i: Instruction) -> bool {
    match level {
        "native" => on_this_machine(i),
        "x86-64-v2" => i == Instruction::Crc32c,
        "x86-64-v3" | "x86-64-v4" => true,
        _ => false,
    }
}

/// What the machine this process runs on has.
#[allow(unreachable_code)]
fn on_this_machine(i: Instruction) -> bool {
    #[cfg(target_arch = "x86_64")]
    return match i {
        Instruction::Crc32c => std::arch::is_x86_feature_detected!("sse4.2"),
        Instruction::Bmi2 => std::arch::is_x86_feature_detected!("bmi2"),
        Instruction::Avx2 => std::arch::is_x86_feature_detected!("avx2"),
    };
    #[cfg(target_arch = "aarch64")]
    return match i {
        Instruction::Crc32c => std::arch::is_aarch64_feature_detected!("crc"),
        Instruction::Bmi2 | Instruction::Avx2 => false,
    };
    let _ = i;
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        assert!(!at_level("", Instruction::Crc32c));
        assert!(!at_level("x86-64", Instruction::Crc32c));
        assert!(at_level("x86-64-v2", Instruction::Crc32c));
        assert!(!at_level("x86-64-v2", Instruction::Avx2));
        assert!(at_level("x86-64-v3", Instruction::Bmi2));
        assert!(at_level("x86-64-v4", Instruction::Avx2));
    }
}
