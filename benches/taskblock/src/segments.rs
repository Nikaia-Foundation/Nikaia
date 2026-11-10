//! **A block as a chain of segments up to a budget** (ADR-327 D15): the
//! pages of `pages.rs` as the current segment, and where it has no room the
//! next segment, a parked one with room or a new one while the budget allows
//! it; more than a segment takes a run of whole segments, and a segment
//! emptied whole goes back. The hot paths are `pages.rs`'s; the chain is
//! reached only where the current segment is full, or a free is not in it.
//! It mirrors `crates/unsafe/task-block`, where `pages.rs` mirrored the
//! single block.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::{Cell, UnsafeCell};

/// A segment's size.
pub const BLOCK: usize = 64 * 1024;

/// A page's size.
const PAGE: usize = 4 * 1024;

/// Pages in a block.
const PAGES: usize = BLOCK / PAGE;

/// A task's block, kept per thread.
pub struct State {
    /// Where the block starts, null outside a task.
    start: Cell<*mut u8>,
    /// One bit per page that is free.
    free: Cell<u32>,
    /// The page allocations are cut from.
    current: Cell<usize>,
    /// How far into it they have come.
    offset: Cell<usize>,
    /// Where the last allocation cut starts in the block, which may grow
    /// where it is.
    last: Cell<usize>,
    /// What each page holds: allocations started in it, and a run's pages
    /// beyond its first are counted in its first.
    held: [Cell<u32>; PAGES],
    /// Segments held beyond the current one: a free not in the current
    /// segment looks into the chain only where this is not nothing.
    extra: Cell<usize>,
}

/// A block's other segments, in a thread-local of their own, so `STATE`
/// keeps no destructor (which would cost every access a check).
pub struct Chain {
    budget: usize,
    parked: Vec<Parked>,
    runs: Vec<(*mut u8, usize)>,
}

/// A segment that is not the current one.
struct Parked {
    start: *mut u8,
    free: u32,
    current: usize,
    offset: usize,
    last: usize,
    held: [u32; PAGES],
}

/// The budget of every task here, in segments.
const BUDGET: usize = 16;

fn fit(free: u32, pages: usize) -> Option<usize> {
    if pages > PAGES {
        return None;
    }
    let want = bits(pages);
    (0..=PAGES - pages).find(|&p| (free >> p) & want == want)
}

fn bits(pages: usize) -> u32 {
    if pages >= 32 {
        u32::MAX
    } else {
        (1u32 << pages) - 1
    }
}

const SEGMENT: Layout = match Layout::from_size_align(BLOCK, PAGE) {
    Ok(layout) => layout,
    Err(_) => panic!("a segment's layout"),
};

fn run_layout(segments: usize) -> Layout {
    Layout::from_size_align(segments * BLOCK, PAGE).expect("a run's layout")
}

thread_local! {
    pub static STATE: State = const {
        State {
            start: Cell::new(std::ptr::null_mut()),
            free: Cell::new(0),
            current: Cell::new(0),
            offset: Cell::new(0),
            last: Cell::new(usize::MAX),
            held: [const { Cell::new(0) }; PAGES],
            extra: Cell::new(0),
        }
    };
    static CHAIN: UnsafeCell<Chain> = const {
        UnsafeCell::new(Chain { budget: BUDGET, parked: Vec::new(), runs: Vec::new() })
    };
}

/// Every page free.
const ALL: u32 = if PAGES == 32 {
    u32::MAX
} else {
    (1 << PAGES) - 1
};

#[cold]
#[inline(never)]
fn full() -> ! {
    panic!("the task's block is full");
}

impl State {
    /// `size` bytes aligned to `align` from the current page, a fresh one
    /// where it is too short, or a run of pages for more than a page.
    #[inline(always)]
    fn take(&self, size: usize, align: usize) -> *mut u8 {
        let start = self.start.get();
        let page = self.current.get();
        let base = page * PAGE;
        let at =
            (start as usize + base + self.offset.get()).next_multiple_of(align) - start as usize;
        if at + size <= base + PAGE {
            self.offset.set(at + size - base);
            self.last.set(at);
            self.held[page].set(self.held[page].get() + 1);
            return unsafe { start.add(at) };
        }
        self.take_slow(size, align)
    }

    #[cold]
    fn take_slow(&self, size: usize, align: usize) -> *mut u8 {
        let start = self.start.get();
        if size > PAGE || align > PAGE {
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
            return self.next(1, size, align);
        }
        let page = free.trailing_zeros() as usize;
        self.free.set(free & !(1 << page));
        self.current.set(page);
        self.offset.set(0);
        let base = page * PAGE;
        let at = (start as usize + base).next_multiple_of(align) - start as usize;
        self.offset.set(at + size - base);
        self.last.set(at);
        self.held[page].set(1);
        unsafe { start.add(at) }
    }

    /// A run of `pages` free pages, held as one allocation of its first.
    #[cold]
    fn run(&self, pages: usize) -> *mut u8 {
        if pages > PAGES {
            return self.whole(pages.div_ceil(PAGES));
        }
        let current = self.current.get();
        let idle = self.held[current].get() == 0;
        let mut free = self.free.get();
        if idle {
            free |= 1 << current;
        }
        let Some(first) = fit(free, pages) else {
            return self.next(pages, pages * PAGE, PAGE);
        };
        let taken = bits(pages) << first;
        let mut left = free & !taken;
        if idle && taken & (1 << current) != 0 {
            self.current.set(first);
            self.offset.set(PAGE);
            self.last.set(usize::MAX);
        } else if idle {
            left &= !(1 << current);
        }
        self.free.set(left);
        self.held[first].set(1);
        unsafe { self.start.get().add(first * PAGE) }
    }

    /// The next segment: a parked one with room, else a new one within the
    /// budget.
    #[cold]
    #[inline(never)]
    fn next(&self, pages: usize, size: usize, align: usize) -> *mut u8 {
        {
            let chain = unsafe { &mut *CHAIN.with(|c| c.get()) };
            let found = chain.parked.iter().position(|p| {
                let idle = if p.held[p.current] == 0 {
                    1 << p.current
                } else {
                    0
                };
                fit(p.free | idle, pages).is_some()
            });
            let index = match found {
                Some(index) => index,
                None => {
                    if 1 + self.extra.get() >= chain.budget {
                        full();
                    }
                    let memory = unsafe { System.alloc(SEGMENT) };
                    if memory.is_null() {
                        full();
                    }
                    self.extra.set(self.extra.get() + 1);
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
            let parked = &mut chain.parked[index];
            parked.start = self.start.replace(parked.start);
            parked.free = self.free.replace(parked.free);
            parked.current = self.current.replace(parked.current);
            parked.offset = self.offset.replace(parked.offset);
            parked.last = self.last.replace(parked.last);
            for (mine, theirs) in self.held.iter().zip(parked.held.iter_mut()) {
                *theirs = mine.replace(*theirs);
            }
        }
        self.take(size, align)
    }

    /// A run of whole segments.
    #[cold]
    #[inline(never)]
    fn whole(&self, segments: usize) -> *mut u8 {
        let chain = unsafe { &mut *CHAIN.with(|c| c.get()) };
        if 1 + self.extra.get() + segments > chain.budget {
            full();
        }
        let memory = unsafe { System.alloc(run_layout(segments)) };
        if memory.is_null() {
            full();
        }
        self.extra.set(self.extra.get() + segments);
        chain.runs.push((memory, segments));
        memory
    }

    /// Whether `ptr` is in a segment beyond the current one.
    #[inline(always)]
    fn chained(&self, ptr: *mut u8) -> bool {
        self.extra.get() != 0 && self.in_chain(ptr)
    }

    #[cold]
    #[inline(never)]
    fn in_chain(&self, ptr: *mut u8) -> bool {
        let chain = unsafe { &*CHAIN.with(|c| c.get()) };
        chain
            .parked
            .iter()
            .any(|p| ptr >= p.start && (ptr as usize) < p.start as usize + BLOCK)
            || chain
                .runs
                .iter()
                .any(|&(s, n)| ptr >= s && (ptr as usize) < s as usize + n * BLOCK)
    }

    /// Give back an allocation beyond the current segment.
    #[cold]
    #[inline(never)]
    fn give_chained(&self, ptr: *mut u8, size: usize) {
        let chain = unsafe { &mut *CHAIN.with(|c| c.get()) };
        if let Some(at) = chain.runs.iter().position(|&(s, _)| s == ptr) {
            let (s, n) = chain.runs.swap_remove(at);
            unsafe { System.dealloc(s, run_layout(n)) };
            self.extra.set(self.extra.get() - n);
            return;
        }
        let Some(at) = chain
            .parked
            .iter()
            .position(|p| ptr >= p.start && (ptr as usize) < p.start as usize + BLOCK)
        else {
            return;
        };
        let p = &mut chain.parked[at];
        let page = (ptr as usize - p.start as usize) / PAGE;
        p.held[page] -= 1;
        if p.held[page] == 0 {
            if size > PAGE {
                p.free |= bits(size.div_ceil(PAGE)) << page;
            } else if page == p.current {
                p.offset = 0;
                p.last = usize::MAX;
            } else {
                p.free |= 1 << page;
            }
        }
        if p.free | (1 << p.current) == ALL && p.held[p.current] == 0 {
            let gone = chain.parked.swap_remove(at);
            self.extra.set(self.extra.get() - 1);
            unsafe { System.dealloc(gone.start, SEGMENT) };
        }
    }

    /// Give back an allocation of `size` bytes at `ptr`.
    #[inline(always)]
    fn give(&self, ptr: *mut u8, size: usize) {
        let at = ptr as usize - self.start.get() as usize;
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
            let pages = size.div_ceil(PAGE);
            let want = if pages >= 32 {
                u32::MAX
            } else {
                (1u32 << pages) - 1
            };
            self.free.set(self.free.get() | (want << page));
        } else if page == self.current.get() {
            self.offset.set(0);
            self.last.set(usize::MAX);
        } else {
            self.free.set(self.free.get() | (1 << page));
        }
    }

    /// Whether `ptr` is in the running task's block.
    #[inline(always)]
    fn ours(&self, ptr: *mut u8) -> bool {
        let start = self.start.get();
        !start.is_null() && ptr >= start && (ptr as usize) < start as usize + BLOCK
    }

    /// Start a task on `block`: every page free but the first, which it
    /// starts cutting from.
    pub fn begin(&self, block: *mut u8) {
        self.start.set(block);
        self.free.set(ALL & !1);
        self.current.set(0);
        self.offset.set(0);
        self.last.set(usize::MAX);
        self.held[0].set(0);
    }

    /// End the task: allocations go to the system again, and the other
    /// segments with them.
    pub fn end(&self) {
        self.start.set(std::ptr::null_mut());
        if self.extra.get() != 0 {
            let chain = unsafe { &mut *CHAIN.with(|c| c.get()) };
            for p in chain.parked.drain(..) {
                unsafe { System.dealloc(p.start, SEGMENT) };
            }
            for (s, n) in chain.runs.drain(..) {
                unsafe { System.dealloc(s, run_layout(n)) };
            }
            self.extra.set(0);
        }
    }
}

pub struct Segments;

unsafe impl GlobalAlloc for Segments {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        STATE.with(|state| {
            if state.start.get().is_null() {
                unsafe { System.alloc(layout) }
            } else {
                state.take(layout.size(), layout.align())
            }
        })
    }
    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        STATE.with(|state| {
            if state.ours(ptr) {
                state.give(ptr, layout.size());
            } else if state.chained(ptr) {
                state.give_chained(ptr, layout.size());
            } else {
                unsafe { System.dealloc(ptr, layout) }
            }
        })
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        STATE.with(|state| {
            let here = state.ours(ptr);
            if !here && !state.chained(ptr) {
                return unsafe { System.realloc(ptr, layout, size) };
            }
            // The last allocation cut grows where it is, inside its page.
            let at = ptr as usize - state.start.get() as usize;
            let page = state.current.get();
            if here
                && layout.size() <= PAGE
                && at == state.last.get()
                && at + size <= (page + 1) * PAGE
                && at / PAGE == page
            {
                state.offset.set(at + size - page * PAGE);
                return ptr;
            }
            let moved = state.take(size, layout.align());
            unsafe { std::ptr::copy_nonoverlapping(ptr, moved, layout.size().min(size)) };
            if state.ours(ptr) {
                state.give(ptr, layout.size());
            } else {
                state.give_chained(ptr, layout.size());
            }
            moved
        })
    }
}
