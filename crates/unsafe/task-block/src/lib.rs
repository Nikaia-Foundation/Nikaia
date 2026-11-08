//! **A task's memory block** ([ADR-327](../../../docs/specification/adr/adr-327.md)
//! D4, D5, D10, D12; #490 step 3), and the lists and text that may be refused
//! by it. Every `unsafe`, and the argument for it, is listed in `README.md`
//! (ADR-218); the caller is `nikaia-std` (`src/block.rs`, `src/task.rs`).
//!
//! A task allocates from a block of its own, and the block is its budget. The
//! block is 16 pages of 4 KiB, each counting what it holds (D12): an
//! allocation moves a pointer through the current page and adds one to the
//! page's count, a free takes one off, and a page whose count drops to nothing
//! starts over (the current one) or is free again (any other). More than a
//! page takes a run of free pages. So a task can allocate again what it freed
//! (D10), and its end hands the block back whole, whatever it still held.
//!
//! **Where a value lives is its handle, [`Place`]**: the task's block, or
//! outside it, where `std`'s global allocator answers (D8, D9). Where a value
//! is *freed* is decided by its address: a pointer inside the running task's
//! block goes back to it, any other to the global allocator. So a list placed
//! in the block but built where no task runs (`main`, a thread of a Rust
//! library) lives outside, and is freed there.
//!
//! **A block value never leaves its task** (D9): the emitter places in the
//! block only what cannot outlive the task, and a value is freed while the
//! task that holds it is polled. Which allocation sites are in the block is
//! the emitter's decision (#490 step 5); until it decides, every site is
//! outside.
//!
//! **A full block refuses, it does not panic**: [`Place::allocate`] hands back
//! an `AllocError`, a collection's `try_reserve` hands it on, and the
//! collection's call ends the task in [`full`], outside the allocator (D6).
//! That is why every growth of [`List`] and [`Text`] goes through
//! `try_reserve`: a plain `push` would abort the process on a refusal.
//!
//! The pages are the measured prototype's (`benches/taskblock/src/pages.rs`,
//! 0.0.1366: -38.5 % instructions against `malloc` on short tasks, -38.7 % on
//! one long task).

#![deny(unsafe_op_in_unsafe_fn)]

use allocator_api2::alloc::{AllocError, Allocator, Global, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

/// The block's size: what a task may hold. Fixed until the strategy sizes it
/// (D7, #490 step 8).
pub const BLOCK: usize = 64 * 1024;

/// A page's size.
const PAGE: usize = 4 * 1024;

/// Pages in a block.
const PAGES: usize = BLOCK / PAGE;

/// Every page free.
const ALL: u32 = if PAGES == 32 {
    u32::MAX
} else {
    (1 << PAGES) - 1
};

/// The pages a run of `pages` takes, as bits from the first.
#[inline]
fn run_bits(pages: usize) -> u32 {
    if pages >= 32 {
        u32::MAX
    } else {
        (1u32 << pages) - 1
    }
}

/// **A task's block**: its memory, taken when it first allocates in it, and
/// what each page holds. A task owns one and puts it in place while it is
/// polled ([`Entered`]).
pub struct Block {
    /// Where the block's memory starts, null until the task first allocates
    /// in it.
    start: Cell<*mut u8>,
    /// One bit per page that is free.
    free: Cell<u32>,
    /// The page allocations are cut from.
    current: Cell<usize>,
    /// How far into it they have come.
    offset: Cell<usize>,
    /// Where in the block the last cut starts, which may grow where it is.
    last: Cell<usize>,
    /// What each page holds: allocations started in it; a run's pages beyond
    /// its first are counted in its first.
    held: [Cell<u32>; PAGES],
}

// SAFETY: the block's memory is the task's alone, and the task, which may move
// between threads between polls, is the only one that reaches it: through
// `CURRENT`, only while it is polled.
unsafe impl Send for Block {}

impl Default for Block {
    fn default() -> Block {
        Block {
            start: Cell::new(std::ptr::null_mut()),
            free: Cell::new(0),
            current: Cell::new(0),
            offset: Cell::new(0),
            last: Cell::new(usize::MAX),
            held: [const { Cell::new(0) }; PAGES],
        }
    }
}

thread_local! {
    /// The block of the task this thread is polling, null where none is.
    static CURRENT: Cell<*const Block> = const { Cell::new(std::ptr::null()) };
    /// **Blocks of tasks that ended**, kept for the next task on this thread:
    /// a short task then costs no call to the system for its memory.
    static SPARE: Spares = const { Spares(Cell::new([std::ptr::null_mut(); SPARES])) };
}

/// How many ended tasks' blocks a thread keeps.
const SPARES: usize = 4;

/// A thread's spare blocks, handed back to the system with the thread.
struct Spares(Cell<[*mut u8; SPARES]>);

impl Drop for Spares {
    fn drop(&mut self) {
        for memory in self.0.get() {
            if !memory.is_null() {
                // SAFETY: a spare was allocated with `MEMORY`, and nothing
                // holds it.
                unsafe { std::alloc::dealloc(memory, MEMORY) };
            }
        }
    }
}

/// The layout of a block's memory.
const MEMORY: std::alloc::Layout = match std::alloc::Layout::from_size_align(BLOCK, PAGE) {
    Ok(layout) => layout,
    Err(_) => panic!("a block's layout"),
};

/// **A full block ends the task** (D6), here and not in the allocator, which
/// only refuses. Cold: a task that fits its block never comes here.
#[cold]
#[inline(never)]
pub fn full() -> ! {
    panic!("the task is over its memory budget")
}

impl Block {
    /// `size` bytes aligned to `align` from the current page, a fresh one
    /// where it is too short, or a run of pages for more than a page; `None`
    /// where the block has no room.
    #[inline(always)]
    fn take(&self, size: usize, align: usize) -> Option<*mut u8> {
        let start = self.start.get();
        if start.is_null() {
            return self.first(size, align);
        }
        let page = self.current.get();
        let base = page * PAGE;
        let at = (start.addr() + base + self.offset.get()).next_multiple_of(align) - start.addr();
        if at + size <= base + PAGE {
            self.offset.set(at + size - base);
            self.last.set(at);
            self.held[page].set(self.held[page].get() + 1);
            // SAFETY: `at + size` is inside the block.
            return Some(unsafe { start.add(at) });
        }
        self.take_slow(size, align)
    }

    /// The task's first allocation in its block: the memory is taken now,
    /// from a spare block or the system.
    #[cold]
    fn first(&self, size: usize, align: usize) -> Option<*mut u8> {
        let spare = SPARE.try_with(|spare| {
            let mut all = spare.0.get();
            let found = all
                .iter_mut()
                .find(|p| !p.is_null())
                .map(|p| std::mem::replace(p, std::ptr::null_mut()));
            spare.0.set(all);
            found
        });
        let spare = spare.ok().flatten();
        let memory = match spare {
            Some(memory) => memory,
            // SAFETY: `MEMORY` has a size.
            None => unsafe { std::alloc::alloc(MEMORY) },
        };
        if memory.is_null() {
            return None;
        }
        self.start.set(memory);
        self.free.set(ALL & !1);
        self.current.set(0);
        self.offset.set(0);
        self.last.set(usize::MAX);
        for page in &self.held {
            page.set(0);
        }
        self.take(size, align)
    }

    #[cold]
    fn take_slow(&self, size: usize, align: usize) -> Option<*mut u8> {
        let start = self.start.get();
        if size > PAGE {
            return self.run(size.div_ceil(PAGE));
        }
        // The current page is too full. Where it holds nothing (an alignment
        // did not fit), it is free again; else its count frees it later.
        let old = self.current.get();
        if self.held[old].get() == 0 {
            self.free.set(self.free.get() | (1 << old));
        }
        let free = self.free.get();
        if free == 0 {
            // Nothing to cut from: the old page, which holds something, stays
            // current, so its last free starts it over.
            return None;
        }
        let page = free.trailing_zeros() as usize;
        self.free.set(free & !(1 << page));
        self.current.set(page);
        let base = page * PAGE;
        let at = (start.addr() + base).next_multiple_of(align) - start.addr();
        self.offset.set(at + size - base);
        self.last.set(at);
        self.held[page].set(1);
        // SAFETY: `align <= PAGE`, so the cut is inside the page.
        Some(unsafe { start.add(at) })
    }

    /// A run of `pages` free pages, held as one allocation of its first.
    #[cold]
    fn run(&self, pages: usize) -> Option<*mut u8> {
        if pages > PAGES {
            return None;
        }
        let free = self.free.get();
        let want = run_bits(pages);
        let first = (0..=PAGES - pages).find(|&p| (free >> p) & want == want)?;
        self.free.set(free & !(want << first));
        self.held[first].set(1);
        // SAFETY: the run is inside the block.
        Some(unsafe { self.start.get().add(first * PAGE) })
    }

    /// Give back an allocation of `size` bytes at `ptr`.
    #[inline(always)]
    fn give(&self, ptr: *mut u8, size: usize) {
        let at = ptr.addr() - self.start.get().addr();
        let page = at / PAGE;
        let now = self.held[page].get() - 1;
        self.held[page].set(now);
        if now == 0 {
            self.emptied(page, size);
        }
    }

    /// A page that holds nothing any more: the current one starts over, any
    /// other is free again, a run with all its pages.
    #[inline(always)]
    fn emptied(&self, page: usize, size: usize) {
        if size > PAGE {
            let want = run_bits(size.div_ceil(PAGE));
            self.free.set(self.free.get() | (want << page));
        } else if page == self.current.get() {
            self.offset.set(0);
            self.last.set(usize::MAX);
        } else {
            self.free.set(self.free.get() | (1 << page));
        }
    }

    /// Whether `ptr` is in this block.
    #[inline(always)]
    fn ours(&self, ptr: *mut u8) -> bool {
        let start = self.start.get();
        !start.is_null() && ptr >= start && ptr.addr() < start.addr() + BLOCK
    }

    /// Grow the allocation at `ptr` from `old` to `new` bytes: in place where
    /// it is the last cut and its page has room, else moved, with one copy.
    #[inline]
    fn grow(&self, ptr: *mut u8, old: usize, new: usize, align: usize) -> Option<*mut u8> {
        let at = ptr.addr() - self.start.get().addr();
        let page = self.current.get();
        if old <= PAGE
            && at == self.last.get()
            && at / PAGE == page
            && at + new <= (page + 1) * PAGE
        {
            self.offset.set(at + new - page * PAGE);
            return Some(ptr);
        }
        let moved = self.take(new, align)?;
        // SAFETY: both are live allocations of at least `old` bytes, and two
        // live allocations do not overlap.
        unsafe { std::ptr::copy_nonoverlapping(ptr, moved, old) };
        self.give(ptr, old);
        Some(moved)
    }
}

/// **A task's block hands its memory back whole** when the task is over, to
/// the thread's spares or the system.
impl Drop for Block {
    fn drop(&mut self) {
        let memory = self.start.get();
        if memory.is_null() {
            return;
        }
        let kept = SPARE.try_with(|spare| {
            let mut all = spare.0.get();
            let slot = all.iter_mut().find(|p| p.is_null());
            let kept = slot.map(|p| *p = memory).is_some();
            spare.0.set(all);
            kept
        });
        if kept != Ok(true) {
            // SAFETY: `memory` was allocated with `MEMORY`.
            unsafe { std::alloc::dealloc(memory, MEMORY) };
        }
    }
}

/// **A task's block in place** while the task is polled, or its body
/// dropped: what was in place before comes back when this is dropped, so a
/// task polled inside another's poll leaves the other's block as it was.
pub struct Entered {
    previous: *const Block,
}

/// Put `block` in place for this thread until the guard is dropped.
#[inline]
pub fn enter(block: &Block) -> Entered {
    Entered {
        previous: CURRENT.with(|current| current.replace(block)),
    }
}

impl Drop for Entered {
    #[inline]
    fn drop(&mut self) {
        CURRENT.with(|current| current.set(self.previous));
    }
}

/// The running task's block, if a task is polled on this thread.
#[inline(always)]
fn current<'a>() -> Option<&'a Block> {
    // SAFETY: a block is in place only while its `Entered` lives, which the
    // task that owns the block holds.
    CURRENT.with(|current| unsafe { current.get().as_ref() })
}

/// **Where a value lives** (D8, D9): the running task's block, or outside
/// it, with `std`'s global allocator. One byte beside a collection's
/// pointer, length and capacity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Place {
    /// Outside every block: what can outlive the task, and whatever the
    /// emitter cannot yet place (#490 step 5).
    #[default]
    Outside,
    /// The running task's block; outside where no task runs.
    Block,
}

/// A block's answer as an allocator's.
#[inline(always)]
fn answered(ptr: Option<*mut u8>, size: usize) -> Result<NonNull<[u8]>, AllocError> {
    match ptr {
        Some(ptr) => Ok(NonNull::slice_from_raw_parts(
            // SAFETY: a block's cut is never null.
            unsafe { NonNull::new_unchecked(ptr) },
            size,
        )),
        None => Err(AllocError),
    }
}

impl Place {
    /// The running task's block where this handle places there and one runs.
    #[inline(always)]
    fn block<'a>(self) -> Option<&'a Block> {
        match self {
            Place::Block => current(),
            Place::Outside => None,
        }
    }

    /// The running task's block where `ptr` is in it.
    #[inline(always)]
    fn holding<'a>(self, ptr: NonNull<u8>) -> Option<&'a Block> {
        self.block().filter(|block| block.ours(ptr.as_ptr()))
    }
}

// SAFETY: a block allocation stays valid until it is given back or its task
// ends, and no block allocation outlives its task (D9, the module's
// invariant); everything else is the global allocator's.
unsafe impl Allocator for Place {
    #[inline]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        match self.block() {
            Some(block) if layout.size() != 0 && layout.align() <= PAGE => {
                answered(block.take(layout.size(), layout.align()), layout.size())
            }
            _ => Global.allocate(layout),
        }
    }

    #[inline]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        match self.holding(ptr) {
            Some(block) => block.give(ptr.as_ptr(), layout.size()),
            // SAFETY: not a block's, so the global allocator's.
            None => unsafe { Global.deallocate(ptr, layout) },
        }
    }

    #[inline]
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old: Layout,
        new: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let Some(block) = self.holding(ptr) else {
            // SAFETY: not a block's, so the global allocator's, and the
            // caller keeps `grow`'s contract.
            return unsafe { Global.grow(ptr, old, new) };
        };
        if new.align() <= PAGE {
            return answered(
                block.grow(ptr.as_ptr(), old.size(), new.size(), new.align()),
                new.size(),
            );
        }
        // An alignment past a page lives outside, as it does when allocated.
        let moved = Global.allocate(new)?;
        // SAFETY: two live allocations of at least `old` bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(ptr.as_ptr(), moved.as_ptr().cast::<u8>(), old.size())
        };
        block.give(ptr.as_ptr(), old.size());
        Ok(moved)
    }
}

/// **A list whose growth may be refused** (D4): `List`'s ground, placed by
/// its handle.
pub type List<T> = allocator_api2::vec::Vec<T, Place>;

/// A new, empty list placed at `place`.
#[inline]
pub fn list<T>(place: Place) -> List<T> {
    List::new_in(place)
}

/// Room for `more` elements, or the task ends (D6).
#[inline]
pub fn reserve<T>(list: &mut List<T>, more: usize) {
    if list.capacity() - list.len() < more && list.try_reserve(more).is_err() {
        full()
    }
}

/// Push `item`, ending the task where the block refuses (D6).
#[inline]
pub fn push<T>(list: &mut List<T>, item: T) {
    if list.len() == list.capacity() && list.try_reserve(1).is_err() {
        full()
    }
    list.push(item);
}

/// **Append a run of elements with one copy**, as `std`'s `Vec` does:
/// `allocator-api2`'s `extend_from_slice` clones one element at a time on
/// stable Rust, which doubled the tasks' count (`benches/fallible`).
#[inline]
pub fn extend<T: Copy>(list: &mut List<T>, run: &[T]) {
    reserve(list, run.len());
    let len = list.len();
    // SAFETY: `reserve` left room for `run.len()` more, and a `Copy` element
    // has nothing to drop; a list and a run it is extended from are two
    // values, so they do not overlap.
    unsafe {
        std::ptr::copy_nonoverlapping(run.as_ptr(), list.as_mut_ptr().add(len), run.len());
        list.set_len(len + run.len());
    }
}

/// **Text whose growth may be refused**: valid UTF-8 in a [`List`] of bytes.
/// Every byte in it came from a `str`, so it is handed out as one without
/// being checked again (#490 step 3), as `String` does.
pub struct Text {
    bytes: List<u8>,
}

impl Text {
    /// A new, empty text placed at `place`.
    #[inline]
    pub fn new_in(place: Place) -> Text {
        Text {
            bytes: List::new_in(place),
        }
    }

    /// Append `s`, with one copy.
    #[inline]
    pub fn push_str(&mut self, s: &str) {
        extend(&mut self.bytes, s.as_bytes());
    }

    /// Append one character.
    #[inline]
    pub fn push(&mut self, c: char) {
        self.push_str(c.encode_utf8(&mut [0; 4]));
    }

    /// The text, unchecked: it only ever grew by whole `str`s.
    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: every append is a whole `str`.
        unsafe { std::str::from_utf8_unchecked(&self.bytes) }
    }

    /// Its bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// ASCII letters made lower case where they are, which keeps the UTF-8
    /// valid.
    #[inline]
    pub fn make_ascii_lowercase(&mut self) {
        self.bytes.make_ascii_lowercase();
    }
}

impl std::ops::Deref for Text {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Write for Text {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    /// Whether `ptr` is in the block in place.
    fn in_block<T>(ptr: *const T) -> bool {
        current().is_some_and(|block| block.ours(ptr.cast_mut().cast()))
    }

    #[test]
    fn without_a_task_a_block_list_lives_outside() {
        let mut xs: List<i64> = list(Place::Block);
        for i in 0..100 {
            push(&mut xs, i);
        }
        assert_eq!(xs.iter().sum::<i64>(), 4950);
    }

    #[test]
    fn a_task_allocates_in_its_block_and_outside_where_told() {
        let block = Block::default();
        let _in = enter(&block);
        let mut inside: List<u64> = list(Place::Block);
        let mut outside: List<u64> = list(Place::Outside);
        push(&mut inside, 1);
        push(&mut outside, 2);
        assert!(in_block(inside.as_ptr()));
        assert!(!in_block(outside.as_ptr()));
    }

    #[test]
    fn memory_freed_is_allocated_again() {
        // D10: a megabyte, a kilobyte at a time, under a 64 KiB block.
        let block = Block::default();
        let _in = enter(&block);
        let mut kept: List<List<u8>> = list(Place::Block);
        for turn in 0..1000_usize {
            let mut chunk: List<u8> = list(Place::Block);
            extend(&mut chunk, &[turn as u8; 1024]);
            assert!(in_block(chunk.as_ptr()));
            if turn % 100 == 0 {
                push(&mut kept, chunk);
            }
        }
        assert_eq!(kept.len(), 10);
        assert!(kept.iter().all(|chunk| chunk.len() == 1024));
    }

    #[test]
    fn a_run_of_pages_is_free_again_when_given_back() {
        let block = Block::default();
        let _in = enter(&block);
        for _ in 0..50 {
            let mut big: List<u8> = list(Place::Block);
            reserve(&mut big, 20 * 1024);
            let mut other: List<u8> = list(Place::Block);
            reserve(&mut other, 20 * 1024);
            assert!(in_block(big.as_ptr()) && in_block(other.as_ptr()));
        }
    }

    #[test]
    fn a_full_block_refuses_and_the_list_call_ends_the_task() {
        let block = Block::default();
        let _in = enter(&block);
        let mut held: List<List<u8>> = list(Place::Block);
        reserve(&mut held, 64);
        let refused = (0..64).any(|_| {
            let mut chunk: List<u8> = list(Place::Block);
            let refused = chunk.try_reserve(4000).is_err();
            held.push(chunk);
            refused
        });
        assert!(refused);
        let mut more: List<u8> = list(Place::Block);
        let ended = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            reserve(&mut more, 4000);
        }));
        let payload = ended.expect_err("a full block ends the task");
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"the task is over its memory budget")
        );
        // What it frees, it can allocate again.
        held.clear();
        reserve(&mut more, 4000);
        assert!(in_block(more.as_ptr()));
    }

    #[test]
    fn the_last_cut_grows_where_it_is() {
        let block = Block::default();
        let _in = enter(&block);
        let mut xs: List<u8> = list(Place::Block);
        reserve(&mut xs, 16);
        let at = xs.as_ptr();
        extend(&mut xs, &[7; 16]);
        reserve(&mut xs, 100);
        assert_eq!(xs.as_ptr(), at);
        assert_eq!(&xs[..], &[7; 16]);
    }

    #[test]
    fn a_list_moves_with_its_elements_when_it_cannot_grow_in_place() {
        let block = Block::default();
        let _in = enter(&block);
        let mut xs: List<u32> = list(Place::Block);
        let mut ys: List<u32> = list(Place::Block);
        for i in 0..2000 {
            push(&mut xs, i);
            push(&mut ys, i * 2);
        }
        assert!(xs.iter().copied().eq(0..2000));
        assert!(ys.iter().copied().eq((0..2000).map(|i| i * 2)));
    }

    #[test]
    fn a_task_polled_inside_another_leaves_its_block_in_place() {
        let outer = Block::default();
        let _outer = enter(&outer);
        let mut a: List<u8> = list(Place::Block);
        push(&mut a, 1);
        {
            let inner = Block::default();
            let _inner = enter(&inner);
            let mut b: List<u8> = list(Place::Block);
            push(&mut b, 2);
            assert!(!outer.ours(b.as_mut_ptr()));
        }
        push(&mut a, 3);
        assert!(outer.ours(a.as_mut_ptr()));
    }

    #[test]
    fn an_ended_tasks_block_serves_the_next_task() {
        let first = {
            let block = Block::default();
            let _in = enter(&block);
            let mut xs: List<u8> = list(Place::Block);
            push(&mut xs, 1);
            drop(xs);
            block.start.get()
        };
        let block = Block::default();
        let _in = enter(&block);
        let mut xs: List<u8> = list(Place::Block);
        push(&mut xs, 1);
        assert_eq!(block.start.get(), first);
    }

    #[test]
    fn an_alignment_past_a_page_lives_outside() {
        #[repr(align(8192))]
        struct Wide(#[allow(dead_code)] u8);
        let block = Block::default();
        let _in = enter(&block);
        let mut xs: List<Wide> = list(Place::Block);
        push(&mut xs, Wide(1));
        assert!(!in_block(xs.as_ptr()));
    }

    #[test]
    fn text_grows_by_whole_strs_and_is_read_unchecked() {
        let block = Block::default();
        let _in = enter(&block);
        let mut text = Text::new_in(Place::Block);
        text.push_str("Grüße, ");
        text.push('ü');
        write!(text, "{}", 42).unwrap();
        text.make_ascii_lowercase();
        assert_eq!(text.as_str(), "grüße, ü42");
        assert_eq!(text.len(), text.as_bytes().len());
        assert_eq!(format!("{text}|{text:?}"), "grüße, ü42|\"grüße, ü42\"");
    }
}
