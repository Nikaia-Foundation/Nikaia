//! `std::mem` - the calling thread's memory, counted (ADR-400).
//!
//! Both functions read a per-thread counter of the program's allocator,
//! `task-block`'s `Counting` (ADR-327 D9, D10), which counts every request
//! and free of the thread that makes it. Neither hands out an address (D3).
//! They stay Rust: the counters live inside the allocator, which is
//! `task-block`'s (ADR-218), and these two lines only read them.

/// **How many times the calling thread has requested memory** since it
/// started (ADR-400 D1). Equal before and after an operation means the
/// operation requested nothing.
#[inline]
pub fn allocations() -> i64 {
    task_block::allocations()
}

/// **The calling thread's bytes in use** (ADR-400 D2): what it allocated
/// less what it freed. A free counts on the freeing thread, so it may be
/// negative.
#[inline]
pub fn usage() -> i64 {
    task_block::usage()
}
