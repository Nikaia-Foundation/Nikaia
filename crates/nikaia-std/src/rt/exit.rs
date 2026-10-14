//! **What `main` still holds at its end: freed, or left to the operating
//! system** ([ADR-348](../../../../docs/specification/adr/adr-348.md)).
//!
//! By default nothing here runs: every value `main` holds dies and its memory
//! is freed, so a leak checker sees a clean exit. A program built with
//! `--optimization=skip-free-at-exit:on` declares an allocator that asks
//! [`frees`] before each free, and its `main` calls [`main_ends`] after its last
//! statement. From there until the drain begins ([`Started::finish`]) the
//! values still die in order - every `Drop` and every cleanup runs, so no
//! effect outside the process is skipped (D2) - but their memory is not handed
//! back: the process is about to end, and the operating system takes the whole
//! of it at once.
//!
//! [`Started::finish`]: super::Started::finish

use std::sync::atomic::{AtomicBool, Ordering};

/// Set between the end of `main`'s last statement and the drain.
static LEAVING: AtomicBool = AtomicBool::new(false);

/// **Whether a free gives the memory back**: what the allocator of a program
/// built with `skip-free-at-exit:on` asks before each free. The allocator is
/// the system's and is written into the program, because this crate has no
/// `unsafe` (ADR-218).
#[inline]
pub fn frees() -> bool {
    !LEAVING.load(Ordering::Relaxed)
}

/// **`main` reached the end of its last statement** (D1): what dies from here
/// on keeps its memory. Called only by a program built with
/// `skip-free-at-exit:on`; a `main` that leaves by `return` or by an error does
/// not call it and frees as any other.
pub fn main_ends() {
    LEAVING.store(true, Ordering::Relaxed);
}

/// The drain begins: what runs in it - unjoined tasks, parked cleanups - frees
/// its memory again, because it may run for as long as `cleanup-deadline`.
pub(super) fn drain_begins() {
    LEAVING.store(false, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The drain takes the flag down again.
    #[test]
    fn the_drain_frees_again() {
        LEAVING.store(true, Ordering::Relaxed);
        drain_begins();
        assert!(!LEAVING.load(Ordering::Relaxed));
    }
}
