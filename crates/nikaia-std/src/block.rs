//! **A task's memory block** ([ADR-327](../../../docs/specification/adr/adr-327.md)
//! D4, D5, D10, D12; #490 step 3).
//!
//! A task allocates from a block of its own, and the block is its budget: 16
//! pages of 4 KiB, each counting what it holds, so what the task frees it can
//! allocate again (D10) and its end hands the block back whole. The pages,
//! the handle and the lists and text whose growth the block may refuse are
//! `task-block`'s (`crates/unsafe/task-block`, ADR-218); [`crate::task`] puts
//! a task's block in place while the task is polled.
//!
//! **Where a value lives is its handle, [`Place`]**: the running task's block,
//! or outside, where `std`'s global allocator answers (D8, D9). Which
//! allocation sites are in the block is the emitter's decision (#490 step 5);
//! until it decides, every site is outside, and the emitter still writes
//! `Vec` and `String`.
//!
//! **A full block ends the task in [`full`]**, outside the allocator, which
//! only refuses (D6); so every growth of a [`List`] or a [`Text`] goes
//! through `try_reserve` ([`push`], [`reserve`], [`extend`]).

pub use task_block::{
    BLOCK, Block, Entered, List, Place, Text, enter, extend, full, list, push, reserve,
};

/// **A map whose table may be refused**: `hashbrown`'s, placed by its handle.
pub type Map<K, V, S> = hashbrown::HashMap<K, V, S, Place>;
