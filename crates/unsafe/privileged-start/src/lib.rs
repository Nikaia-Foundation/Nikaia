//! **Whether this process was started with more rights than its caller**
//! ([ADR-397](../../../docs/specification/adr/adr-397.md) D3): a setuid or
//! setgid program, or one with file capabilities, run by a user who holds
//! fewer rights than it gets. Such a program inherits every environment
//! variable its caller chose, so a Nikaia program built with the environment
//! the operator's ends before `main` when this answers `true`.
//!
//! * Linux and Android: `getauxval(AT_SECURE)`, the flag the kernel puts in
//!   the auxiliary vector for exactly this start, the one the C library's own
//!   secure mode reads (`secure_getenv`, the loader ignoring `LD_PRELOAD`).
//! * macOS, FreeBSD, OpenBSD, NetBSD: `issetugid()`.
//! * Elsewhere (Windows among them) no such start exists: `false`.
//!
//! Both are declared here with `extern "C"`, so the crate depends on nothing.
//! Every `unsafe` in this crate, and the argument for it, is listed in
//! `README.md`.

#![deny(unsafe_op_in_unsafe_fn)]

/// Whether the process was started with more rights than its caller.
pub fn privileged() -> bool {
    imp::privileged()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod imp {
    use core::ffi::c_ulong;

    /// The auxiliary vector's entry the kernel sets to 1 for a privileged
    /// start (`<linux/auxvec.h>`, `AT_SECURE`).
    const AT_SECURE: c_ulong = 23;

    unsafe extern "C" {
        fn getauxval(kind: c_ulong) -> c_ulong;
    }

    pub(super) fn privileged() -> bool {
        // SAFETY: `getauxval` takes an integer and hands back an integer; it
        // reads the vector the kernel wrote before the process ran and that
        // nobody changes, and answers 0 for an entry it does not hold
        // (README, first row).
        unsafe { getauxval(AT_SECURE) != 0 }
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
mod imp {
    use core::ffi::c_int;

    unsafe extern "C" {
        fn issetugid() -> c_int;
    }

    pub(super) fn privileged() -> bool {
        // SAFETY: `issetugid` takes nothing and hands back an integer; it
        // reads a flag the kernel keeps for the process (README, second row).
        unsafe { issetugid() != 0 }
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
mod imp {
    pub(super) fn privileged() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    /// A test runs as its caller started it, with no more rights: the answer
    /// is `false`. (A privileged start cannot be made inside a test; the
    /// README says how it was checked by hand.) Miri has neither foreign
    /// function.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn a_plain_start_is_not_privileged() {
        assert!(!super::privileged());
    }
}
