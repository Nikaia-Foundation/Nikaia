//! **The box of a recursive type** ([ADR-327](../../../docs/specification/adr/adr-327.md)
//! D16, #2406): one pointer wide, built in the running task's block or outside
//! it as its site was placed (D9), and freed where its address says.
//!
//! A recursive enum or struct holds its children through a box, and a tree
//! holds one per node: a box that carried its place would double every child
//! pointer for a fact the address already holds. So [`Boxed`] is the pointer
//! alone, one type for both places: [`Boxed::new`] builds outside, with the
//! global allocator; [`Boxed::placed`] builds in the running task's block,
//! where one runs. Its free asks the address, as a list's does
//! ([`Place::deallocate`](crate::Place)): a cut of the running task's block,
//! or a segment of its chain (D15), goes back to it, every other pointer to
//! the global allocator.
//!
//! **A placed box never leaves its task** (D9): the compiler places a box
//! site in the block only where the value it is part of cannot outlive the
//! task, so the box is freed while its task is polled, its block in place.
//! A clone is built outside: where the clone goes is not the original's
//! site.

use crate::{Place, full};
use allocator_api2::alloc::{Allocator, Layout};
use std::marker::PhantomData;
use std::ptr::NonNull;

/// A value on its own memory, in the task's block or outside it, one
/// pointer wide.
pub struct Boxed<T> {
    /// The value, written at construction and dropped once, in [`Drop`] or
    /// [`Boxed::into_inner`].
    ptr: NonNull<T>,
    /// The box owns a `T`: for the drop check and the auto traits.
    owns: PhantomData<T>,
}

// SAFETY: a `Boxed<T>` owns its `T` alone, as a `Box<T>` does; a placed one
// crosses to no other task (D9), and an outside one is the global
// allocator's, which any thread frees.
unsafe impl<T: Send> Send for Boxed<T> {}
// SAFETY: a shared `Boxed<T>` hands out only `&T`.
unsafe impl<T: Sync> Sync for Boxed<T> {}

impl<T> Boxed<T> {
    /// **A box built outside every block** (D9): the value can outlive the
    /// task, or the compiler cannot tell.
    #[inline]
    pub fn new(value: T) -> Boxed<T> {
        Boxed::at(value, Place::Outside)
    }

    /// **A box built in the running task's block** (D16), outside where no
    /// task runs. A full block ends the task (D6).
    #[inline]
    #[track_caller]
    pub fn placed(value: T) -> Boxed<T> {
        Boxed::at(value, Place::Block)
    }

    /// A box built at `place`.
    #[inline]
    #[track_caller]
    pub fn at(value: T, place: Place) -> Boxed<T> {
        let layout = Layout::new::<T>();
        if layout.size() == 0 {
            // Nothing to allocate: a dangling, aligned pointer holds a
            // zero-sized value, as `Box` does.
            return Boxed {
                ptr: NonNull::dangling(),
                owns: PhantomData,
            };
        }
        let ptr = match place.allocate(layout) {
            Ok(memory) => memory.cast::<T>(),
            Err(_) if place == Place::Block => full(),
            Err(_) => std::alloc::handle_alloc_error(layout),
        };
        // SAFETY: fresh memory with `T`'s layout, written once.
        unsafe { ptr.as_ptr().write(value) };
        Boxed {
            ptr,
            owns: PhantomData,
        }
    }

    /// **The value, the box freed** (ADR-246 D2's `open` of a box taken by
    /// value).
    #[inline]
    pub fn into_inner(self) -> T {
        let boxed = std::mem::ManuallyDrop::new(self);
        // SAFETY: the value was written at construction, and the box is not
        // dropped (`ManuallyDrop`), so it is read once.
        let value = unsafe { boxed.ptr.as_ptr().read() };
        // SAFETY: the memory is this box's, and nothing reads it after.
        unsafe { free(boxed.ptr) };
        value
    }

    /// Whether the box lives in the running task's block.
    #[inline]
    pub fn in_block(&self) -> bool {
        Place::Block.holds(self.ptr.as_ptr().cast())
    }
}

/// **Give back a box's memory where its address says** (D16): the running
/// task's block, or the global allocator.
///
/// # Safety
///
/// `ptr` is a box's, allocated for a `T`, and its value is dropped or moved
/// out already; it is freed once.
#[inline]
unsafe fn free<T>(ptr: NonNull<T>) {
    let layout = Layout::new::<T>();
    if layout.size() != 0 {
        // SAFETY: `Place::Block` frees by the address: a cut of the running
        // block goes back to it, any other pointer to the global allocator,
        // which allocated it (the caller's contract).
        unsafe { Place::Block.deallocate(ptr.cast(), layout) };
    }
}

impl<T> Drop for Boxed<T> {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: the value was written at construction and is dropped once,
        // here; then its memory is freed once.
        unsafe {
            std::ptr::drop_in_place(self.ptr.as_ptr());
            free(self.ptr);
        }
    }
}

impl<T> std::ops::Deref for Boxed<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        // SAFETY: a live box holds a written value; the borrow is the box's.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> std::ops::DerefMut for Boxed<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as `deref`, and the box is borrowed mutably.
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> AsRef<T> for Boxed<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T> std::borrow::Borrow<T> for Boxed<T> {
    fn borrow(&self) -> &T {
        self
    }
}

/// A clone is built outside: where it goes is not the original's site (D9).
impl<T: Clone> Clone for Boxed<T> {
    fn clone(&self) -> Boxed<T> {
        Boxed::new(T::clone(self))
    }
}

impl<T: Default> Default for Boxed<T> {
    fn default() -> Boxed<T> {
        Boxed::new(T::default())
    }
}

impl<T> From<T> for Boxed<T> {
    fn from(value: T) -> Boxed<T> {
        Boxed::new(value)
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Boxed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Boxed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        T::fmt(self, f)
    }
}

impl<T: PartialEq> PartialEq for Boxed<T> {
    fn eq(&self, other: &Boxed<T>) -> bool {
        T::eq(self, other)
    }
}

impl<T: Eq> Eq for Boxed<T> {}

impl<T: PartialOrd> PartialOrd for Boxed<T> {
    fn partial_cmp(&self, other: &Boxed<T>) -> Option<std::cmp::Ordering> {
        T::partial_cmp(self, other)
    }
}

impl<T: Ord> Ord for Boxed<T> {
    fn cmp(&self, other: &Boxed<T>) -> std::cmp::Ordering {
        T::cmp(self, other)
    }
}

impl<T: std::hash::Hash> std::hash::Hash for Boxed<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        T::hash(self, state);
    }
}

/// A view's keep rebases the value into a new box, built outside, as
/// `Box`'s does.
#[cfg(feature = "tether")]
impl<'a, T: tether::Rebase<'a>> tether::Rebase<'a> for Boxed<T> {
    type At = Boxed<T::At>;
    fn rebase<const N: usize>(self, keeps: &tether::Keeps<'a, N>) -> Boxed<T::At> {
        Boxed::new(self.into_inner().rebase(keeps))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, enter};

    /// A tree as the emitter writes one (ADR-246): its children in boxes.
    #[derive(Debug, Clone, PartialEq)]
    enum Tree {
        Leaf,
        Node(Boxed<Tree>, Boxed<Tree>),
    }

    fn placed(depth: u32) -> Tree {
        if depth == 0 {
            return Tree::Leaf;
        }
        Tree::Node(
            Boxed::placed(placed(depth - 1)),
            Boxed::placed(placed(depth - 1)),
        )
    }

    fn count(tree: &Tree) -> u32 {
        match tree {
            Tree::Leaf => 1,
            Tree::Node(left, right) => 1 + count(left) + count(right),
        }
    }

    #[test]
    fn a_box_is_one_pointer_wide() {
        assert_eq!(
            std::mem::size_of::<Boxed<Tree>>(),
            std::mem::size_of::<usize>()
        );
        assert_eq!(
            std::mem::size_of::<Option<Boxed<Tree>>>(),
            std::mem::size_of::<usize>()
        );
    }

    #[test]
    fn without_a_task_a_placed_box_lives_outside() {
        let tree = placed(4);
        assert_eq!(count(&tree), 31);
        let Tree::Node(left, _) = &tree else { panic!() };
        assert!(!left.in_block());
    }

    #[test]
    fn a_placed_tree_lives_in_the_block_and_goes_back_to_it() {
        let block = Block::default();
        let _in = enter(&block);
        let before = crate::usage();
        {
            let tree = placed(6);
            assert_eq!(count(&tree), 127);
            let Tree::Node(left, right) = &tree else {
                panic!()
            };
            assert!(left.in_block() && right.in_block());
        }
        assert_eq!(crate::usage(), before);
    }

    #[test]
    fn an_outside_box_is_freed_outside_inside_a_task() {
        let outside = Boxed::new(Tree::Leaf);
        let block = Block::default();
        let _in = enter(&block);
        assert!(!outside.in_block());
        drop(outside);
        let mut tree = Tree::Node(Boxed::new(Tree::Leaf), Boxed::placed(Tree::Leaf));
        if let Tree::Node(left, _) = &mut tree {
            **left = placed(2);
        }
        assert_eq!(count(&tree), 9);
    }

    #[test]
    fn a_clone_is_built_outside_and_compares_equal() {
        let block = Block::default();
        let _in = enter(&block);
        let tree = placed(3);
        let copy = tree.clone();
        assert_eq!(tree, copy);
        let Tree::Node(left, _) = &copy else { panic!() };
        assert!(!left.in_block());
        assert_eq!(format!("{:?}", Boxed::new(5)), "5");
    }

    #[test]
    fn a_box_taken_apart_frees_its_memory_and_keeps_the_value() {
        let block = Block::default();
        let _in = enter(&block);
        let boxed = Boxed::placed(String::from("kept"));
        assert!(boxed.in_block());
        let text = boxed.into_inner();
        assert_eq!(text, "kept");
        let unit = Boxed::placed(());
        assert!(!unit.in_block());
        unit.into_inner();
    }

    #[test]
    fn a_full_block_ends_the_task_where_a_box_is_placed() {
        let block = Block::with_budget(crate::BLOCK);
        let _in = enter(&block);
        let mut kept: Vec<Boxed<[u8; 1024]>> = Vec::new();
        let ended = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for _ in 0..200 {
                kept.push(Boxed::placed([0; 1024]));
            }
        }));
        assert!(ended.is_err_and(|payload| payload.is::<crate::OverBudget>()));
        drop(kept);
    }
}
