#![allow(dead_code, unused_imports)]

//! **A task's block, as `nikaia-std` has it** (ADR-327 D5, D12): every list,
//! text and table here is placed in the running task's block, and a full
//! block ends the task instead of aborting (`nikaia_std::block`). Where no
//! task runs (the 1BRC aggregation is one long `main`), the handle places
//! outside, with the global allocator, so that side counts the handle and the
//! fallible collections; the tasks' side counts the block.

pub use nikaia_std::block::{enter, extend, full as spent, list, push, Block, List, Place};

/// Where the collections here are placed: the running task's block.
pub const BUDGET: Place = Place::Block;

/// Text, as the bytes of a list that may refuse.
pub type Text = List<u8>;

/// `write!` into text that may refuse.
pub struct Into<'a>(pub &'a mut Text);

impl std::fmt::Write for Into<'_> {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        extend(self.0, s.as_bytes());
        Ok(())
    }
}

/// A new, empty text.
#[inline]
pub fn text() -> Text {
    list(BUDGET)
}

/// A map whose table comes from the block.
pub type Map<K, V> =
    nikaia_std::block::Map<K, V, std::hash::BuildHasherDefault<nikaia_std::hash::FxHasher>>;
