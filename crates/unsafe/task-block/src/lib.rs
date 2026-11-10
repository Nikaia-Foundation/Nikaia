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
//! **A block is a chain of segments up to the task's budget** (D15): a full
//! segment is followed by the next while the segments held stay within the
//! budget the strategy gave the task when it started, a segment emptied whole
//! is given back, and more than a segment takes a run of whole segments,
//! counted as many. The hot path inside the current segment is the one of a
//! single block; the chain is reached only where it is full, and a free only
//! where its pointer is not in the current segment.
//!
//! The pages are the measured prototype's (`benches/taskblock/src/pages.rs`,
//! 0.0.1366: -38.5 % instructions against `malloc` on short tasks, -38.7 % on
//! one long task).

#![deny(unsafe_op_in_unsafe_fn)]

use allocator_api2::alloc::{AllocError, Allocator, Global, Layout};
use std::alloc::{GlobalAlloc, System};
use std::cell::{Cell, UnsafeCell};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A segment's size (D12, D15): a block is a chain of them up to the task's
/// budget.
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
    /// The segments beyond the current one, and the budget (D15). Touched only
    /// where the current segment is full, or a free is not in it.
    chain: UnsafeCell<Chain>,
    /// **What the task holds outside its block** (ADR-327 D9, D10), in bytes:
    /// what [`Counting`] counted while the block was in place, less what was
    /// handed over. A free counts against whoever runs, so it may go below
    /// zero where a task frees what another built.
    outside: Cell<isize>,
}

/// **A block's other segments** (D15): reached only on the cold paths.
struct Chain {
    /// How many segments the block may hold, the current one included.
    budget: usize,
    /// Segments held beyond the current one: the parked ones and the runs.
    extra: usize,
    /// Segments that were current once and hold something still.
    parked: Vec<Parked>,
    /// Runs of whole segments, one allocation each: where it starts and how
    /// many segments it is.
    runs: Vec<(*mut u8, usize)>,
}

/// **A segment that is not the current one**: the fields of [`Block`]'s
/// current segment, kept while another is cut from.
struct Parked {
    start: *mut u8,
    free: u32,
    current: usize,
    offset: usize,
    last: usize,
    held: [u32; PAGES],
}

impl Parked {
    /// Whether `ptr` is in this segment.
    fn ours(&self, ptr: *mut u8) -> bool {
        ptr >= self.start && ptr.addr() < self.start.addr() + BLOCK
    }

    /// Room for `pages`: a free run, the current page counted where it
    /// holds nothing.
    fn has_room(&self, pages: usize) -> bool {
        let idle = if self.held[self.current] == 0 {
            1 << self.current
        } else {
            0
        };
        fit(self.free | idle, pages).is_some()
    }

    /// Give back an allocation of `size` bytes at `ptr`, as [`Block::give`]
    /// does in the current segment. Whether the segment holds nothing now.
    fn give(&mut self, ptr: *mut u8, size: usize) -> bool {
        let page = (ptr.addr() - self.start.addr()) / PAGE;
        self.held[page] -= 1;
        if self.held[page] == 0 {
            if size > PAGE {
                self.free |= run_bits(size.div_ceil(PAGE)) << page;
            } else if page == self.current {
                self.offset = 0;
                self.last = usize::MAX;
            } else {
                self.free |= 1 << page;
            }
        }
        self.free | (1 << self.current) == ALL && self.held[self.current] == 0
    }
}

/// Where a run of `pages` free pages starts in `free`.
fn fit(free: u32, pages: usize) -> Option<usize> {
    if pages > PAGES {
        return None;
    }
    let want = run_bits(pages);
    (0..=PAGES - pages).find(|&p| (free >> p) & want == want)
}

/// **Segments taken from the system and not given back**, by every block and
/// every thread's spares: what the tasks hold, for `task::Load` (D15). Counted
/// only where a segment comes from the system or goes back to it, so a task
/// served from a spare does not touch it.
static SEGMENTS: AtomicUsize = AtomicUsize::new(0);

/// **The bytes the tasks' blocks hold** (D15), spares included.
pub fn held() -> usize {
    SEGMENTS.load(Ordering::Relaxed) * BLOCK
}

/// A segment from the thread's spares or the system; null where the system
/// refuses.
fn segment() -> *mut u8 {
    let spare = SPARE.try_with(|spare| {
        let mut all = spare.0.get();
        let found = all
            .iter_mut()
            .find(|p| !p.is_null())
            .map(|p| std::mem::replace(p, std::ptr::null_mut()));
        spare.0.set(all);
        found
    });
    match spare.ok().flatten() {
        Some(memory) => memory,
        None => {
            // SAFETY: `MEMORY` has a size.
            let memory = unsafe { System.alloc(MEMORY) };
            if !memory.is_null() {
                SEGMENTS.fetch_add(1, Ordering::Relaxed);
            }
            memory
        }
    }
}

/// Hand a segment back, to the thread's spares or the system.
fn hand_back(memory: *mut u8) {
    let kept = SPARE.try_with(|spare| {
        let mut all = spare.0.get();
        let slot = all.iter_mut().find(|p| p.is_null());
        let kept = slot.map(|p| *p = memory).is_some();
        spare.0.set(all);
        kept
    });
    if kept != Ok(true) {
        // SAFETY: `memory` was allocated with `MEMORY`.
        unsafe { System.dealloc(memory, MEMORY) };
        SEGMENTS.fetch_sub(1, Ordering::Relaxed);
    }
}

/// The layout of a run of `segments` whole segments.
fn run_layout(segments: usize) -> std::alloc::Layout {
    match std::alloc::Layout::from_size_align(segments * BLOCK, PAGE) {
        Ok(layout) => layout,
        Err(_) => MEMORY,
    }
}

// SAFETY: the block's memory is the task's alone, and the task, which may move
// between threads between polls, is the only one that reaches it: through
// `CURRENT`, only while it is polled.
unsafe impl Send for Block {}

impl Default for Block {
    /// A block of one segment: a shared value's, or a task's without a
    /// strategy's budget.
    fn default() -> Block {
        Block::with_budget(BLOCK)
    }
}

impl Block {
    /// **A block that may hold `bytes`** (D15): as many whole segments, one at
    /// least.
    pub fn with_budget(bytes: usize) -> Block {
        Block {
            start: Cell::new(std::ptr::null_mut()),
            free: Cell::new(0),
            current: Cell::new(0),
            offset: Cell::new(0),
            last: Cell::new(usize::MAX),
            held: [const { Cell::new(0) }; PAGES],
            chain: UnsafeCell::new(Chain {
                budget: (bytes / BLOCK).max(1),
                extra: 0,
                parked: Vec::new(),
                runs: Vec::new(),
            }),
            outside: Cell::new(0),
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
                unsafe { System.dealloc(memory, MEMORY) };
                SEGMENTS.fetch_sub(1, Ordering::Relaxed);
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
/// only refuses. Cold: a task that fits its block never comes here. It unwinds
/// as a panic does, so the cleanups run, but past the panic hook: the task did
/// not crash, it ran out of its budget, and whoever waits for it is told so
/// with `task::OverBudget` (ADR-327 D13, ADR-async-5 D19).
#[cold]
#[inline(never)]
pub fn full() -> ! {
    std::panic::resume_unwind(Box::new(OverBudget))
}

/// **What a full block unwinds with** (ADR-327 D13): the task's edge tells it
/// from a panic by this payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverBudget;

impl std::fmt::Display for OverBudget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the task is over its memory budget")
    }
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
        let memory = segment();
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
            // current, so its last free starts it over; the next segment
            // serves, where the budget has room.
            return self.next(1, size, align);
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
            return self.whole(pages.div_ceil(PAGES));
        }
        // The current page counts where it holds nothing, or a run of every
        // page would never fit a fresh segment, whose first page is current.
        let current = self.current.get();
        let idle = self.held[current].get() == 0;
        let mut free = self.free.get();
        if idle {
            free |= 1 << current;
        }
        let Some(first) = fit(free, pages) else {
            return self.next(pages, pages * PAGE, PAGE);
        };
        let taken = run_bits(pages) << first;
        let mut left = free & !taken;
        if idle && taken & (1 << current) != 0 {
            // The run took the current page: its first page is current now,
            // full, so the next cut takes a free page.
            self.current.set(first);
            self.offset.set(PAGE);
            self.last.set(usize::MAX);
        } else if idle {
            left &= !(1 << current);
        }
        self.free.set(left);
        self.held[first].set(1);
        // SAFETY: the run is inside the block.
        Some(unsafe { self.start.get().add(first * PAGE) })
    }

    /// **The next segment** (D15), for a cut of `size` bytes that needs
    /// `pages` pages and the current segment has no room for: a parked one
    /// with room, else a new one where the budget allows it; it becomes the
    /// current one and the current one is parked.
    #[cold]
    #[inline(never)]
    fn next(&self, pages: usize, size: usize, align: usize) -> Option<*mut u8> {
        {
            // SAFETY: the chain is reached only through its block, by the one
            // thread that polls the task, and no other borrow of it lives.
            let chain = unsafe { &mut *self.chain.get() };
            let index = match chain.parked.iter().position(|p| p.has_room(pages)) {
                Some(index) => index,
                None => {
                    if (2 + chain.extra) * BLOCK + self.held_outside() > chain.budget * BLOCK {
                        return None;
                    }
                    let memory = segment();
                    if memory.is_null() {
                        return None;
                    }
                    chain.extra += 1;
                    chain.parked.push(Parked {
                        start: memory,
                        free: ALL & !1,
                        current: 0,
                        offset: 0,
                        last: usize::MAX,
                        held: [0; PAGES],
                    });
                    chain.parked.len() - 1
                }
            };
            self.swap(&mut chain.parked[index]);
        }
        self.take(size, align)
    }

    /// What the task holds outside the block, none where it freed more than
    /// it built.
    #[inline]
    fn held_outside(&self) -> usize {
        usize::try_from(self.outside.get()).unwrap_or(0)
    }

    /// **Whether the block holds what it holds within its budget** (D9): its
    /// segments and what it holds outside.
    fn within(&self) -> bool {
        // SAFETY: as in `next`; only read.
        let chain = unsafe { &*self.chain.get() };
        let segments = if self.start.get().is_null() {
            0
        } else {
            1 + chain.extra
        };
        segments * BLOCK + self.held_outside() <= chain.budget * BLOCK
    }

    /// Exchange the current segment with a parked one.
    fn swap(&self, parked: &mut Parked) {
        parked.start = self.start.replace(parked.start);
        parked.free = self.free.replace(parked.free);
        parked.current = self.current.replace(parked.current);
        parked.offset = self.offset.replace(parked.offset);
        parked.last = self.last.replace(parked.last);
        for (mine, theirs) in self.held.iter().zip(parked.held.iter_mut()) {
            *theirs = mine.replace(*theirs);
        }
    }

    /// **A run of `segments` whole segments** (D15), for more than a segment:
    /// one allocation, counted as that many against the budget.
    #[cold]
    #[inline(never)]
    fn whole(&self, segments: usize) -> Option<*mut u8> {
        // SAFETY: as in `next`.
        let chain = unsafe { &mut *self.chain.get() };
        if (1 + chain.extra + segments) * BLOCK + self.held_outside() > chain.budget * BLOCK {
            return None;
        }
        // SAFETY: the layout has a size.
        let memory = unsafe { System.alloc(run_layout(segments)) };
        if memory.is_null() {
            return None;
        }
        SEGMENTS.fetch_add(segments, Ordering::Relaxed);
        chain.extra += segments;
        chain.runs.push((memory, segments));
        Some(memory)
    }

    /// Whether `ptr` is in a segment of the chain beyond the current one.
    #[inline(always)]
    fn chained(&self, ptr: *mut u8) -> bool {
        // SAFETY: as in `next`; only read.
        let extra = unsafe { (*self.chain.get()).extra };
        extra != 0 && self.in_chain(ptr)
    }

    #[cold]
    #[inline(never)]
    fn in_chain(&self, ptr: *mut u8) -> bool {
        // SAFETY: as in `next`; only read.
        let chain = unsafe { &*self.chain.get() };
        chain.parked.iter().any(|p| p.ours(ptr))
            || chain
                .runs
                .iter()
                .any(|&(start, n)| ptr >= start && ptr.addr() < start.addr() + n * BLOCK)
    }

    /// Give back an allocation in a segment beyond the current one: a run
    /// goes back to the system, a parked segment emptied whole to the spares.
    #[cold]
    #[inline(never)]
    fn give_chained(&self, ptr: *mut u8, size: usize) {
        // SAFETY: as in `next`.
        let chain = unsafe { &mut *self.chain.get() };
        if let Some(at) = chain.runs.iter().position(|&(start, _)| start == ptr) {
            let (start, segments) = chain.runs.swap_remove(at);
            // SAFETY: the run was allocated with this layout.
            unsafe { System.dealloc(start, run_layout(segments)) };
            SEGMENTS.fetch_sub(segments, Ordering::Relaxed);
            chain.extra -= segments;
            return;
        }
        let Some(at) = chain.parked.iter().position(|p| p.ours(ptr)) else {
            return;
        };
        if chain.parked[at].give(ptr, size) {
            let emptied = chain.parked.swap_remove(at);
            chain.extra -= 1;
            hand_back(emptied.start);
        }
    }

    /// Give back an allocation of `size` bytes at `ptr`, wherever in the
    /// chain it is.
    #[inline(always)]
    fn give_any(&self, ptr: *mut u8, size: usize) {
        if self.ours(ptr) {
            self.give(ptr, size);
        } else {
            self.give_chained(ptr, size);
        }
    }

    /// Give back an allocation of `size` bytes at `ptr`, in the current
    /// segment.
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

    /// Whether `ptr` is in the current segment.
    #[inline(always)]
    fn ours(&self, ptr: *mut u8) -> bool {
        let start = self.start.get();
        !start.is_null() && ptr >= start && ptr.addr() < start.addr() + BLOCK
    }

    /// Grow the allocation at `ptr` from `old` to `new` bytes: in place where
    /// it is the last cut and its page has room, else moved, with one copy.
    #[inline]
    fn grow(&self, ptr: *mut u8, old: usize, new: usize, align: usize) -> Option<*mut u8> {
        let at = ptr.addr().wrapping_sub(self.start.get().addr());
        let page = self.current.get();
        if self.ours(ptr)
            && old <= PAGE
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
        // The cut may have made another segment current.
        self.give_any(ptr, old);
        Some(moved)
    }
}

/// **A task's block hands its memory back whole** when the task is over, to
/// the thread's spares or the system, every segment of the chain.
impl Drop for Block {
    fn drop(&mut self) {
        let memory = self.start.get();
        if memory.is_null() {
            return;
        }
        hand_back(memory);
        let chain = self.chain.get_mut();
        if chain.extra != 0 {
            for parked in chain.parked.drain(..) {
                hand_back(parked.start);
            }
            for (start, segments) in chain.runs.drain(..) {
                // SAFETY: the run was allocated with this layout.
                unsafe { System.dealloc(start, run_layout(segments)) };
                SEGMENTS.fetch_sub(segments, Ordering::Relaxed);
            }
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

/// **The running task's block takes a budget of `bytes`** (D15), before its
/// body runs: a detached task's strategy decides once its task is in place.
pub fn budget_running(bytes: usize) {
    if let Some(block) = current() {
        // SAFETY: the chain is reached only through its block, by the thread
        // that polls the task, and no other borrow of it lives here.
        unsafe { (*block.chain.get()).budget = (bytes / BLOCK).max(1) };
    }
}

/// **Count `bytes` held outside against the running block** (D9, D10):
/// what [`Counting`] calls on every allocation and free; nothing where no
/// block is in place.
#[inline(always)]
pub fn count(bytes: isize) {
    if let Some(block) = current() {
        block.outside.set(block.outside.get().wrapping_add(bytes));
    }
}

/// **A value of `size` bytes leaves the running task** (D9): sent, set
/// into a shared value, or handed to a Rust owner.
#[inline]
pub fn hand_over(size: usize) {
    count(-isize::try_from(size).unwrap_or(isize::MAX));
}

/// **A value of `size` bytes reaches the running task or shared value**
/// (D9): counted, and whether its block still holds what it holds within its
/// budget. Where it does not, the caller panics (D6): the value is the
/// receiver's now, and its free counts there.
#[inline]
pub fn take_over(size: usize) -> bool {
    count(isize::try_from(size).unwrap_or(isize::MAX));
    current().is_none_or(Block::within)
}

/// **What the running task still holds outside** (D9) as it ends, taken
/// off its block: its value's deep size, for whoever joins it.
pub fn leftover() -> usize {
    match current() {
        Some(block) => usize::try_from(block.outside.replace(0)).unwrap_or(0),
        None => 0,
    }
}

/// **The program's allocator** (ADR-327 D9, D10): the system's, and every
/// allocation and free counted against the block in place, so a budget
/// counts what a task holds outside its block too. A free counts against
/// whoever runs (D9: frees go to the running account).
pub struct Counting;

// SAFETY: every call is the system allocator's with the caller's arguments;
// the count touches only a `Cell` of the block in place, never memory.
unsafe impl GlobalAlloc for Counting {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller keeps `alloc`'s contract.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            count(layout.size().cast_signed());
        }
        ptr
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as in `alloc`.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            count(layout.size().cast_signed());
        }
        ptr
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller keeps `dealloc`'s contract.
        unsafe { System.dealloc(ptr, layout) };
        count(-layout.size().cast_signed());
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: the caller keeps `realloc`'s contract.
        let moved = unsafe { System.realloc(ptr, layout, size) };
        if !moved.is_null() {
            count(size.cast_signed().wrapping_sub(layout.size().cast_signed()));
        }
        moved
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

    /// Whether `ptr` is in the running task's block, where this handle
    /// places.
    #[inline(always)]
    pub(crate) fn holds(self, ptr: *mut u8) -> bool {
        self.block()
            .is_some_and(|block| block.ours(ptr) || block.chained(ptr))
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
        let at = ptr.as_ptr();
        match self.block() {
            Some(block) if block.ours(at) => block.give(at, layout.size()),
            Some(block) if block.chained(at) => block.give_chained(at, layout.size()),
            // SAFETY: not a block's, so the global allocator's.
            _ => unsafe { Global.deallocate(ptr, layout) },
        }
    }

    #[inline]
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old: Layout,
        new: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let at = ptr.as_ptr();
        let Some(block) = self
            .block()
            .filter(|block| block.ours(at) || block.chained(at))
        else {
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
        block.give_any(ptr.as_ptr(), old.size());
        Ok(moved)
    }
}

/// **A value with a block of its own** (D5, D6; #1536): what a shared value
/// is below its lock. What is placed in the block while the value is
/// [`inside`](Housed::inside) - a door, its construction, its rebuild - is
/// the value's, not the task's that opened the door, and the value is
/// dropped with its block in place, so what it still holds there goes back
/// to the block before the block goes.
///
/// **A value that leaves the door is not in the block** (D9, supervisor
/// 2026-11-10 on #1536): `Clone` builds outside, and the compiler places in
/// the block only what the door keeps to itself, so nothing the door hands
/// out points into a block that is not in place where it is freed.
pub struct Housed<T> {
    /// The value; dropped by hand, inside its block.
    value: std::mem::ManuallyDrop<T>,
    /// Its block, taken when something is first placed in it.
    block: Block,
}

impl<T> Housed<T> {
    /// A value built already, outside the block.
    pub fn new(value: T) -> Housed<T> {
        Housed {
            value: std::mem::ManuallyDrop::new(value),
            block: Block::default(),
        }
    }

    /// A value built inside its own block (D5: the construction is the
    /// value's).
    #[track_caller]
    pub fn built(build: impl FnOnce() -> T) -> Housed<T> {
        let block = Block::default();
        let value = {
            let _in = enter(&block);
            build()
        };
        Housed {
            value: std::mem::ManuallyDrop::new(value),
            block,
        }
    }

    /// **The value with its block in place**, for a door: what `f` places
    /// in the block is the value's, and a full block ends `f` in [`full`],
    /// which its lock reads as a poison (D6).
    #[inline]
    #[track_caller]
    pub fn inside<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        let _in = enter(&self.block);
        f(&mut self.value)
    }

    /// The value, read without its block in place: for a door that only
    /// reads and builds nothing.
    #[inline]
    pub fn get(&self) -> &T {
        &self.value
    }
}

impl<T> Drop for Housed<T> {
    fn drop(&mut self) {
        let _in = enter(&self.block);
        // SAFETY: the value is dropped once, here, and never read again: the
        // `Housed` is being dropped.
        unsafe { std::mem::ManuallyDrop::drop(&mut self.value) };
    }
}

impl<T: Default> Default for Housed<T> {
    fn default() -> Housed<T> {
        Housed::new(T::default())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Housed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

mod list;
pub mod text;

pub use list::List;
pub use text::Text;

/// A new, empty list placed at `place`.
#[inline]
pub fn list<T>(place: Place) -> List<T> {
    List::new_in(place)
}

/// Room for `more` elements, or the task ends (D6).
#[inline]
#[track_caller]
pub fn reserve<T>(list: &mut List<T>, more: usize) {
    list.reserve(more);
}

/// Push `item`, ending the task where the block refuses (D6).
#[inline]
#[track_caller]
pub fn push<T>(list: &mut List<T>, item: T) {
    list.push(item);
}

/// **Append a run of elements with one copy** ([`List::extend_copied`]).
#[inline]
#[track_caller]
pub fn extend<T: Copy>(list: &mut List<T>, run: &[T]) {
    list.extend_copied(run);
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
        assert_eq!(payload.downcast_ref::<OverBudget>(), Some(&OverBudget));
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
    fn a_block_follows_a_full_segment_with_the_next_within_its_budget() {
        // D15: 30 chunks of 4000 bytes, two segments, under a budget
        // of four, and a run of two segments; then all of it given back.
        let block = Block::with_budget(4 * BLOCK);
        let _in = enter(&block);
        let mut held: List<List<u8>> = list(Place::Outside);
        for turn in 0..30_u8 {
            let mut chunk: List<u8> = list(Place::Block);
            extend(&mut chunk, &[turn; 4000]);
            assert!(Place::Block.holds(chunk.as_mut_ptr()));
            push(&mut held, chunk);
        }
        assert!(
            held.iter()
                .enumerate()
                .all(|(i, c)| c[..] == [i as u8; 4000])
        );
        let mut large: List<u8> = list(Place::Block);
        reserve(&mut large, 100 * 1024);
        assert!(Place::Block.holds(large.as_mut_ptr()));
        let ended = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut more: List<u8> = list(Place::Block);
            reserve(&mut more, 100 * 1024);
        }));
        assert!(ended.is_err(), "past the budget the task ends");
        drop(large);
        held.clear();
        // SAFETY: only read, nothing else borrows the chain.
        assert_eq!(unsafe { (*block.chain.get()).extra }, 0);
    }

    #[test]
    fn a_run_of_every_page_takes_a_whole_segment() {
        // D15: a list of exactly 64 KiB fits a segment whose current page
        // holds nothing, in the first segment and in a next one; a small cut
        // after it takes a page of its own, not one of the run's.
        let block = Block::with_budget(4 * BLOCK);
        let _in = enter(&block);
        let mut first: List<u8> = list(Place::Block);
        reserve(&mut first, BLOCK);
        let mut small: List<u8> = list(Place::Block);
        extend(&mut small, &[7; 100]);
        let mut second: List<u8> = list(Place::Block);
        reserve(&mut second, BLOCK);
        extend(&mut first, &[1; BLOCK]);
        extend(&mut second, &[2; BLOCK]);
        assert!(Place::Block.holds(first.as_mut_ptr()) && Place::Block.holds(second.as_mut_ptr()));
        assert!(small[..] == [7; 100] && first.iter().all(|&b| b == 1));
        drop(first);
        drop(second);
        let mut again: List<u8> = list(Place::Block);
        reserve(&mut again, BLOCK);
        extend(&mut again, &[3; BLOCK]);
        assert!(small[..] == [7; 100] && again.iter().all(|&b| b == 3));
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
