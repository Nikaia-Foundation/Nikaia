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
//! until it decides, every site is outside.
//!
//! **[`List`] and [`Text`] are the ground of Nikaia's `List` and `String`**
//! (D4): a list or text placed by its handle, `FromIterator` for `.collect()`
//! (a newtype, since the orphan rule allows none on `allocator-api2`'s `Vec`),
//! and the allocating text methods built where the handle says, with no
//! `String` between ([`to_uppercase`], [`to_lowercase`], [`replace`],
//! [`replacen`], [`repeat`], [`join`], [`format`]). Toward Rust they convert
//! with `From`/`Into`, the same memory where it is outside the running
//! task's block (D8). The emitter still writes `Vec` and `String` until it
//! switches to them (#490).
//!
//! **A full block ends the task in [`full`]**, outside the allocator, which
//! only refuses (D6); so every growth of a [`List`] or a [`Text`] goes
//! through `try_reserve` ([`push`], [`reserve`], [`extend`]).

pub use task_block::text::{format, join, repeat, replace, replacen, to_lowercase, to_uppercase};
pub use task_block::{
    BLOCK, Block, Entered, List, Place, Text, enter, extend, full, list, push, reserve,
};

/// **A map whose table may be refused**: `std`'s own map
/// ([`crate::map::HashMap`]), whose table is placed by its handle and grows
/// in a block only through `try_reserve`.
pub type Map<K, V, S> = crate::map::HashMap<K, V, S>;

/// **The block types under the names a program's Rust writes** (#490,
/// staged): `Vec` is [`List`], `String` is [`Text`], `vec![…]` builds a
/// [`List`] and `format!(…)` a [`Text`], so a program lowers to the block
/// types where its file imports these names (`lowers_to_block_types` in
/// `tools/emit_needs.nika`, the one place that decides it).
pub mod names {
    pub use super::List as Vec;
    pub use super::Text as String;
    pub use crate::__nikaia_list as vec;
    pub use crate::__nikaia_text as format;
}

/// `vec![…]` for a [`List`]: empty, `n` clones of one item, or the items
/// listed, all outside every block (D9) until the emitter places sites.
#[macro_export]
#[doc(hidden)]
macro_rules! __nikaia_list {
    () => {
        $crate::block::List::new()
    };
    ($item:expr; $n:expr) => {
        $crate::block::List::from_elem($item, $n)
    };
    ($($item:expr),+ $(,)?) => {
        $crate::block::List::from([$($item),+])
    };
}

/// `format!(…)` for a [`Text`], built with no `String` between ([`format`]),
/// outside every block (D9) until the emitter places sites.
#[macro_export]
#[doc(hidden)]
macro_rules! __nikaia_text {
    ($($part:tt)*) => {
        $crate::block::format($crate::block::Place::Outside, ::std::format_args!($($part)*))
    };
}
