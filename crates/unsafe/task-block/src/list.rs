//! **`List`'s ground** (ADR-327 D4, #490 step 1 of the switch): a list placed
//! by its handle, whose every growth goes through `try_reserve`, so a full
//! block ends the task in [`full`] instead of aborting the process.
//!
//! A newtype and not `allocator-api2`'s `Vec<T, Place>` itself: Rust's orphan
//! rule lets neither this crate nor `nikaia-std` give that `Vec` a
//! `FromIterator` (what `.collect()` needs), and its own `push`, `insert` and
//! `extend` abort on a refusal. What only reads is the slice's ([`Deref`]).
//!
//! **Toward Rust it crosses as it is** where it lives outside the running
//! task's block (D8): [`From<List<T>>`](List#impl-From<List<T>>-for-Vec<T>)
//! for `Vec<T>` hands over the same memory, as `From<Vec<T>>` takes it; only a
//! list in the block is copied out, which D9 lets the emitter never ask for.

use crate::{Place, full};
use allocator_api2::vec::Vec as PlacedVec;
use std::borrow::{Borrow, BorrowMut};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut, RangeBounds};

/// **A list whose growth may be refused** (D4), placed by its handle.
///
/// **It holds its `Vec` directly** (#490, supervisor 2026-11-09), so it is
/// as large as the `Vec`, 32 bytes: the nesting depth on the default stack.
/// `allocator-api2`'s `Vec` has no `#[may_dangle]` on stable Rust, so a list
/// of views keeps what it views borrowed until it is dropped; the emitter
/// drops such a list at its last use where what it views ends first
/// (`view_list_drops`).
pub struct List<T> {
    items: PlacedVec<T, Place>,
}

const _: () = assert!(size_of::<List<u8>>() == size_of::<PlacedVec<u8, Place>>());
const _: () = assert!(size_of::<List<u8>>() == 32);

/// **What a list's growth does to its `Vec`**, whoever holds it: a [`List`] or a
/// [`Text`](crate::Text).
pub(crate) trait Grow<T> {
    /// Room for `more` elements, or the task ends (D6).
    fn grow(&mut self, more: usize);
    /// Room for exactly `more` elements, or the task ends (D6).
    fn grow_exact(&mut self, more: usize);
    /// Push `item`, ending the task where the block refuses (D6).
    fn push_or_end(&mut self, item: T);
    /// The elements from `at` on, as a new `Vec` in the same place.
    fn split_off_placed(&mut self, at: usize) -> PlacedVec<T, Place>;
    /// Whether its memory is the global allocator's, or none at all.
    fn is_outside(&self) -> bool;
    /// Append a run of elements with one copy.
    fn extend_copied(&mut self, run: &[T])
    where
        T: Copy;
}

impl<T> Grow<T> for PlacedVec<T, Place> {
    #[inline]
    #[track_caller]
    fn grow(&mut self, more: usize) {
        if self.capacity() - self.len() < more && self.try_reserve(more).is_err() {
            full()
        }
    }

    #[inline]
    #[track_caller]
    fn grow_exact(&mut self, more: usize) {
        if self.capacity() - self.len() < more && self.try_reserve_exact(more).is_err() {
            full()
        }
    }

    #[inline]
    #[track_caller]
    fn push_or_end(&mut self, item: T) {
        if self.len() == self.capacity() && self.try_reserve(1).is_err() {
            full()
        }
        self.push(item);
    }

    #[inline]
    fn split_off_placed(&mut self, at: usize) -> PlacedVec<T, Place> {
        assert!(
            at <= self.len(),
            "`at` split index (is {at}) should be <= len"
        );
        let mut tail = PlacedVec::new_in(*self.allocator());
        tail.grow_exact(self.len() - at);
        tail.extend(self.drain(at..));
        tail
    }

    #[inline]
    fn is_outside(&self) -> bool {
        self.capacity() == 0
            || size_of::<T>() == 0
            || !self.allocator().holds(self.as_ptr().cast_mut().cast())
    }

    /// **One copy**, as `std`'s `Vec` does: `allocator-api2`'s
    /// `extend_from_slice` clones one element at a time on stable Rust,
    /// which doubled the tasks' count (`benches/fallible`).
    #[inline]
    #[track_caller]
    fn extend_copied(&mut self, run: &[T])
    where
        T: Copy,
    {
        self.grow(run.len());
        let len = self.len();
        // SAFETY: `grow` left room for `run.len()` more, and a `Copy`
        // element has nothing to drop; a `Vec` and a run it is extended from
        // are two values, so they do not overlap.
        unsafe {
            std::ptr::copy_nonoverlapping(run.as_ptr(), self.as_mut_ptr().add(len), run.len());
            self.set_len(len + run.len());
        }
    }
}

impl<T> List<T> {
    /// The list that holds `items`.
    #[inline(always)]
    pub(crate) const fn of(items: PlacedVec<T, Place>) -> List<T> {
        List { items }
    }

    /// Its `Vec`.
    #[inline(always)]
    fn items(&self) -> &PlacedVec<T, Place> {
        &self.items
    }

    /// Its `Vec`, to change.
    #[inline(always)]
    fn items_mut(&mut self) -> &mut PlacedVec<T, Place> {
        &mut self.items
    }

    /// Its `Vec`, taken out.
    #[inline(always)]
    pub(crate) fn into_items(self) -> PlacedVec<T, Place> {
        self.items
    }

    /// A new, empty list placed at `place`.
    #[inline]
    pub const fn new_in(place: Place) -> List<T> {
        List::of(PlacedVec::new_in(place))
    }

    /// A new, empty list outside every block (D9: where the compiler
    /// cannot tell, a value is built outside).
    #[inline]
    pub const fn new() -> List<T> {
        List::new_in(Place::Outside)
    }

    /// A new list with room for `room` elements, placed at `place`.
    #[inline]
    pub fn with_capacity_in(room: usize, place: Place) -> List<T> {
        let mut list = List::new_in(place);
        list.reserve_exact(room);
        list
    }

    /// A new list with room for `room` elements, outside every block.
    #[inline]
    pub fn with_capacity(room: usize) -> List<T> {
        List::with_capacity_in(room, Place::Outside)
    }

    /// Where the list lives.
    #[inline]
    pub fn place(&self) -> Place {
        *self.items().allocator()
    }

    /// How many elements it has room for.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.items().capacity()
    }

    /// Room for `more` elements, the list growing as `Vec` grows, or the
    /// task ends (D6).
    #[inline]
    #[track_caller]
    pub fn reserve(&mut self, more: usize) {
        self.items_mut().grow(more);
    }

    /// Room for exactly `more` elements, or the task ends (D6).
    #[inline]
    #[track_caller]
    pub fn reserve_exact(&mut self, more: usize) {
        self.items_mut().grow_exact(more);
    }

    /// Room for `more` elements, or the block's refusal.
    #[inline]
    pub fn try_reserve(
        &mut self,
        more: usize,
    ) -> Result<(), allocator_api2::collections::TryReserveError> {
        self.items_mut().try_reserve(more)
    }

    /// Push `item`, ending the task where the block refuses (D6).
    #[inline]
    #[track_caller]
    pub fn push(&mut self, item: T) {
        self.items_mut().push_or_end(item);
    }

    /// The last element, taken out.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        self.items_mut().pop()
    }

    /// Put `item` at `at`, moving the rest one on.
    #[inline]
    #[track_caller]
    pub fn insert(&mut self, at: usize, item: T) {
        self.reserve(1);
        self.items_mut().insert(at, item);
    }

    /// Take out the element at `at`, moving the rest one back.
    #[inline]
    pub fn remove(&mut self, at: usize) -> T {
        self.items_mut().remove(at)
    }

    /// Take out the element at `at`, the last taking its place.
    #[inline]
    pub fn swap_remove(&mut self, at: usize) -> T {
        self.items_mut().swap_remove(at)
    }

    /// Keep the first `len` elements.
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        self.items_mut().truncate(len);
    }

    /// Drop every element, keeping the room.
    #[inline]
    pub fn clear(&mut self) {
        self.items_mut().clear();
    }

    /// Keep the elements `keep` says yes to.
    #[inline]
    pub fn retain(&mut self, keep: impl FnMut(&T) -> bool) {
        self.items_mut().retain(keep);
    }

    /// Keep the elements `keep` says yes to, each lent to it to change.
    #[inline]
    pub fn retain_mut(&mut self, keep: impl FnMut(&mut T) -> bool) {
        self.items_mut().retain_mut(keep);
    }

    /// Drop each element `same` finds equal to the one before it.
    #[inline]
    pub fn dedup_by(&mut self, same: impl FnMut(&mut T, &mut T) -> bool) {
        self.items_mut().dedup_by(same);
    }

    /// Drop each element whose key equals the one before it.
    #[inline]
    pub fn dedup_by_key<K: PartialEq>(&mut self, key: impl FnMut(&mut T) -> K) {
        self.items_mut().dedup_by_key(key);
    }

    /// Take out the elements of `range`, as an iterator.
    #[inline]
    pub fn drain<R: RangeBounds<usize>>(
        &mut self,
        range: R,
    ) -> allocator_api2::vec::Drain<'_, T, Place> {
        self.items_mut().drain(range)
    }

    /// Move every element of `other` to the end.
    #[inline]
    pub fn append(&mut self, other: &mut List<T>) {
        self.reserve(other.len());
        self.items_mut().append(other.items_mut());
    }

    /// The elements from `at` on, as a new list in the same place.
    #[inline]
    pub fn split_off(&mut self, at: usize) -> List<T> {
        List::of(self.items_mut().split_off_placed(at))
    }

    /// Grow or shrink to `len`, new elements made by `make`.
    #[inline]
    pub fn resize_with(&mut self, len: usize, make: impl FnMut() -> T) {
        if len > self.len() {
            self.reserve(len - self.len());
        }
        self.items_mut().resize_with(len, make);
    }

    /// Give back the room it does not use.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.items_mut().shrink_to_fit();
    }

    /// Its elements.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self.items()
    }

    /// Its elements, to change.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.items_mut()
    }

    /// Its first element's address.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.items_mut().as_mut_ptr()
    }

    /// Set the length (`Vec::set_len`).
    ///
    /// # Safety
    ///
    /// As `Vec::set_len`: `len` is at most the capacity, and the first `len`
    /// elements are initialised.
    #[inline]
    pub unsafe fn set_len(&mut self, len: usize) {
        // SAFETY: the caller's.
        unsafe { self.items_mut().set_len(len) }
    }

    /// The elements of `run`, one clone each, after one reservation.
    #[inline]
    #[track_caller]
    pub fn extend_from_slice(&mut self, run: &[T])
    where
        T: Clone,
    {
        self.reserve(run.len());
        for item in run {
            self.items_mut().push(item.clone());
        }
    }

    /// Grow or shrink to `len`, new elements clones of `item`.
    #[inline]
    #[track_caller]
    pub fn resize(&mut self, len: usize, item: T)
    where
        T: Clone,
    {
        if len > self.len() {
            self.reserve(len - self.len());
        }
        self.items_mut().resize(len, item);
    }

    /// Drop each element equal to the one before it.
    #[inline]
    pub fn dedup(&mut self)
    where
        T: PartialEq,
    {
        self.items_mut().dedup();
    }

    /// A list built from `items`, placed at `place`.
    #[inline]
    pub fn from_iter_in(items: impl IntoIterator<Item = T>, place: Place) -> List<T> {
        let mut list = List::new_in(place);
        list.extend(items);
        list
    }

    /// The items listed, placed at `place`: an array, not an iterator, so
    /// that `rustc` carries the list's element type into each item (a
    /// closure becomes the list's `dyn Fn`), as `list_of` does outside.
    #[inline]
    pub fn from_array_in<const N: usize>(items: [T; N], place: Place) -> List<T> {
        let mut list = List::with_capacity_in(N, place);
        list.extend(items);
        list
    }

    /// A copy built at `place` (`Clone` builds outside, as the map's does).
    #[inline]
    pub fn clone_in(&self, place: Place) -> List<T>
    where
        T: Clone,
    {
        let mut copy = List::with_capacity_in(self.len(), place);
        copy.extend_from_slice(self);
        copy
    }

    /// Whether its memory is the global allocator's: outside the running
    /// task's block (where a block list built where no task runs lives
    /// too), or no memory at all.
    #[inline]
    pub fn is_outside(&self) -> bool {
        self.items().is_outside()
    }
}

impl<T: Copy> List<T> {
    /// **Append a run of elements with one copy**, as `std`'s `Vec` does:
    /// `allocator-api2`'s `extend_from_slice` clones one element at a time on
    /// stable Rust, which doubled the tasks' count (`benches/fallible`).
    #[inline]
    #[track_caller]
    pub fn extend_copied(&mut self, run: &[T]) {
        self.items_mut().extend_copied(run);
    }
}

impl<T> Default for List<T> {
    #[inline]
    fn default() -> List<T> {
        List::new()
    }
}

impl<T> Deref for List<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        self.items()
    }
}

impl<T> DerefMut for List<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        self.items_mut()
    }
}

impl<T> Borrow<[T]> for List<T> {
    #[inline]
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T> BorrowMut<[T]> for List<T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T> AsRef<[T]> for List<T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T> AsMut<[T]> for List<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

/// **A clone is built outside** (D9), as the map's is: where it goes, the
/// compiler has not followed. [`List::clone_in`] places it.
impl<T: Clone> Clone for List<T> {
    #[inline]
    fn clone(&self) -> List<T> {
        self.clone_in(Place::Outside)
    }
}

impl<T: Hash> Hash for List<T> {
    /// As the slice's, so a list hashes as `Vec` does and a map keyed by
    /// lists keeps its order.
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl<T: PartialEq<U>, U> PartialEq<List<U>> for List<T> {
    #[inline]
    fn eq(&self, other: &List<U>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: PartialEq<U>, U> PartialEq<[U]> for List<T> {
    #[inline]
    fn eq(&self, other: &[U]) -> bool {
        self.as_slice() == other
    }
}

impl<T: PartialEq<U>, U> PartialEq<&[U]> for List<T> {
    #[inline]
    fn eq(&self, other: &&[U]) -> bool {
        self.as_slice() == *other
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for List<T> {
    #[inline]
    fn eq(&self, other: &[U; N]) -> bool {
        self.as_slice() == other
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<&[U; N]> for List<T> {
    #[inline]
    fn eq(&self, other: &&[U; N]) -> bool {
        self.as_slice() == *other
    }
}

impl<T: PartialEq<U>, U> PartialEq<Vec<U>> for List<T> {
    #[inline]
    fn eq(&self, other: &Vec<U>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

/// A slice compared with a list, element by element, as with a `Vec`.
impl<T: PartialEq<U>, U> PartialEq<List<U>> for [T] {
    #[inline]
    fn eq(&self, other: &List<U>) -> bool {
        *self == **other
    }
}

impl<T: Eq> Eq for List<T> {}

impl<T: PartialOrd> PartialOrd for List<T> {
    #[inline]
    fn partial_cmp(&self, other: &List<T>) -> Option<std::cmp::Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}

impl<T: Ord> Ord for List<T> {
    #[inline]
    fn cmp(&self, other: &List<T>) -> std::cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl<T: fmt::Debug> fmt::Debug for List<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_slice(), f)
    }
}

/// **`.collect()` builds outside** (D9): where the list goes, the compiler
/// has not followed; [`List::from_iter_in`] places it.
impl<T> FromIterator<T> for List<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> List<T> {
        List::from_iter_in(items, Place::Outside)
    }
}

/// Each element through the list's fallible growth: room for the
/// iterator's lower bound first, as `Vec` does.
impl<T> Extend<T> for List<T> {
    #[inline]
    #[track_caller]
    fn extend<I: IntoIterator<Item = T>>(&mut self, items: I) {
        let items = items.into_iter();
        self.reserve(items.size_hint().0);
        for item in items {
            self.push(item);
        }
    }
}

impl<'a, T: Copy + 'a> Extend<&'a T> for List<T> {
    #[inline]
    #[track_caller]
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, items: I) {
        self.extend(items.into_iter().copied());
    }
}

/// What a grammar's repetition collects into where the grammar says
/// `collect List;` (winnow-grammar 0.2, #490): a new list outside every
/// block (D9), each element through the list's fallible growth.
#[cfg(feature = "collect")]
impl<T> winnow_grammar::Collect<T> for List<T> {
    #[inline(always)]
    fn empty() -> List<T> {
        List::new()
    }
    #[inline(always)]
    fn push(&mut self, item: T) {
        List::push(self, item)
    }
    #[inline(always)]
    fn len(&self) -> usize {
        self.items().len()
    }
}

/// **A parallel walk collects into a list** as into a `Vec` (#490): the
/// `Vec` rayon builds, taken over as it is (D8), outside every block (D9).
/// **A list of views is carried into a keep's handles** element by element,
/// as `Vec`'s is (#490: a task holding a list of a struct of views).
#[cfg(feature = "tether")]
impl<'a, T: tether::Rebase<'a>> tether::Rebase<'a> for List<T> {
    type At = List<T::At>;
    fn rebase<const N: usize>(self, keeps: &tether::Keeps<'a, N>) -> List<T::At> {
        self.into_iter().map(|value| value.rebase(keeps)).collect()
    }
}

#[cfg(feature = "rayon")]
impl<T: Send> rayon::iter::FromParallelIterator<T> for List<T> {
    #[inline]
    fn from_par_iter<I: rayon::iter::IntoParallelIterator<Item = T>>(items: I) -> List<T> {
        List::from(rayon::iter::ParallelIterator::collect::<Vec<T>>(
            items.into_par_iter(),
        ))
    }
}

impl<T> IntoIterator for List<T> {
    type Item = T;
    type IntoIter = allocator_api2::vec::IntoIter<T, Place>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.into_items().into_iter()
    }
}

impl<'a, T> IntoIterator for &'a List<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.items().iter()
    }
}

impl<'a, T> IntoIterator for &'a mut List<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.items_mut().iter_mut()
    }
}

impl<T: Clone> From<&[T]> for List<T> {
    #[inline]
    fn from(run: &[T]) -> List<T> {
        List::with_capacity(run.len()).with_clones_of(run)
    }
}

impl<T, const N: usize> From<[T; N]> for List<T> {
    #[inline]
    fn from(items: [T; N]) -> List<T> {
        List::from_array(items)
    }
}

impl<T> List<T> {
    /// A list of `items`, outside every block: what `vec![a, b]` is on the
    /// block types. An inherent function and not `From::from`, so that the
    /// expected element type reaches the items (a `Kept` closure is coerced
    /// to `Kept<dyn Fn>` there, as in `vec!`).
    #[inline]
    pub fn from_array<const N: usize>(items: [T; N]) -> List<T> {
        List::from_iter_in(items, Place::Outside)
    }
}

impl<T: Clone> List<T> {
    /// `n` clones of `item`, outside every block (`vec![item; n]`).
    #[inline]
    pub fn from_elem(item: T, n: usize) -> List<T> {
        let mut list = List::with_capacity(n);
        list.resize(n, item);
        list
    }

    /// `n` clones of `item`, placed at `place`: one allocation of `n`.
    #[inline]
    #[track_caller]
    pub fn from_elem_in(item: T, n: usize, place: Place) -> List<T> {
        let mut list = List::with_capacity_in(n, place);
        list.resize(n, item);
        list
    }

    /// This list with clones of `run` after its elements.
    #[inline]
    fn with_clones_of(mut self, run: &[T]) -> List<T> {
        self.extend_from_slice(run);
        self
    }
}

/// **A Rust `Vec` is taken over as it is** (D8), outside every block: the
/// same memory, no copy.
impl<T> From<Vec<T>> for List<T> {
    #[inline]
    fn from(vec: Vec<T>) -> List<T> {
        let mut vec = ManuallyDrop::new(vec);
        let (ptr, len, room) = (vec.as_mut_ptr(), vec.len(), vec.capacity());
        // SAFETY: `std`'s `Vec` allocated `ptr` with its global allocator
        // for `room` elements (or not at all, for no room or a size of
        // nothing, which `allocator-api2`'s `Vec` treats alike);
        // `Place::Outside` frees with that same allocator, and the `Vec` is
        // never dropped.
        List::of(unsafe { PlacedVec::from_raw_parts_in(ptr, len, room, Place::Outside) })
    }
}

/// **Toward Rust a list crosses as it is** (D8) where its memory is the
/// global allocator's; one in the running task's block is moved out into a
/// new `Vec`, which D9 lets the emitter never ask for.
impl<T> From<List<T>> for Vec<T> {
    #[inline]
    fn from(list: List<T>) -> Vec<T> {
        let outside = list.is_outside();
        let (ptr, len, room, place) = list.into_items().into_raw_parts_with_alloc();
        if outside {
            // SAFETY: the memory is the global allocator's, for `room`
            // elements (or none), which `std`'s `Vec` frees with the same
            // allocator.
            return unsafe { Vec::from_raw_parts(ptr, len, room) };
        }
        let mut vec = Vec::with_capacity(len);
        // SAFETY: `vec` has room for `len`; the elements are moved, not
        // copied: the block's list is put back with no length, so it frees
        // only its memory.
        unsafe {
            std::ptr::copy_nonoverlapping(ptr, vec.as_mut_ptr(), len);
            vec.set_len(len);
            drop(PlacedVec::from_raw_parts_in(ptr, 0, room, place));
        }
        vec
    }
}

/// **A list of exactly `N` is an array**, as a `Vec` of `N` is (#490: a C
/// library's fixed array field read back from a list); a list of another
/// length is handed back as the error.
impl<T, const N: usize> TryFrom<List<T>> for [T; N] {
    type Error = List<T>;
    fn try_from(list: List<T>) -> Result<[T; N], List<T>> {
        <[T; N]>::try_from(Vec::from(list)).map_err(List::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, enter};
    use std::hash::BuildHasher;

    #[test]
    fn collect_builds_outside_and_a_clone_too() {
        let block = Block::default();
        let _in = enter(&block);
        let xs: List<u32> = (0..100).collect();
        assert_eq!(xs.place(), Place::Outside);
        let mut placed = List::from_iter_in(0..100u32, Place::Block);
        assert!(!placed.is_outside());
        let copy = placed.clone();
        assert!(copy.is_outside() && copy == xs && placed == xs);
        placed.retain(|x| x % 2 == 0);
        placed.insert(0, 7);
        assert_eq!(placed[..3], [7, 0, 2]);
        let tail = placed.split_off(49);
        assert_eq!((placed.len(), tail.as_slice()), (49, &[96, 98][..]));
        assert!(!tail.is_outside());
    }

    #[test]
    fn a_list_hashes_compares_and_prints_as_a_vec() {
        let vec = vec![3_i64, 1, 2];
        let list = List::from(vec.clone());
        let hasher = std::collections::hash_map::RandomState::new();
        assert_eq!(hasher.hash_one(&vec), hasher.hash_one(&list));
        assert_eq!(format!("{list:?}"), format!("{vec:?}"));
        let four = List::from(vec![4]);
        assert!(list == vec && list == [3, 1, 2] && list < four);
        let mut sorted = list.clone();
        sorted.sort();
        assert_eq!(sorted, [1, 2, 3]);
        assert_eq!((&list).into_iter().sum::<i64>(), 6);
        assert_eq!(list.into_iter().rev().collect::<List<_>>(), [2, 1, 3]);
    }

    #[test]
    fn toward_rust_a_list_outside_crosses_as_it_is() {
        let vec: Vec<String> = vec!["a".into(), "b".into()];
        let at = vec.as_ptr();
        let list = List::from(vec);
        let back: Vec<String> = list.into();
        assert_eq!(back.as_ptr(), at);
        assert_eq!(back, ["a", "b"]);
        // Built in the block where no task runs, it lives outside.
        let mut outside = List::new_in(Place::Block);
        outside.push(1_u8);
        assert!(outside.is_outside());
        let at = outside.as_ptr();
        let crossed = Vec::from(outside);
        assert_eq!(crossed.as_ptr(), at);
    }

    #[test]
    fn toward_rust_a_list_in_the_block_is_moved_out() {
        let block = Block::default();
        let _in = enter(&block);
        let mut list = List::new_in(Place::Block);
        list.extend(["x".to_string(), "y".to_string()]);
        assert!(!list.is_outside());
        let vec: Vec<String> = list.into();
        assert!(!block.ours(vec.as_ptr().cast_mut().cast()));
        assert_eq!(vec, ["x", "y"]);
        let empty: Vec<u8> = List::new_in(Place::Block).into();
        assert!(empty.is_empty());
    }
}
