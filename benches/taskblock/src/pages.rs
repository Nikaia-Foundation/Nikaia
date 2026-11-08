//! **A block of pages, a page given back when nothing in it is held**
//! (ADR-327 D10): the 64 KiB block is 16 pages of 4 KiB. An allocation moves
//! a pointer through the current page, as the bump block does, and adds one
//! to the page's count of what it holds; a free takes one off. A page whose
//! count drops to nothing is free again: the current page starts over from
//! its first byte, any other goes back to the block's free pages. A larger
//! allocation takes a run of free pages. No free page left ends the task
//! (D6). Outside a task the system allocator answers.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// The block's size: what a task may hold.
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
        }
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
            full();
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
        let free = self.free.get();
        let want = if pages >= 32 {
            u32::MAX
        } else {
            (1u32 << pages) - 1
        };
        let Some(first) = (0..=PAGES.saturating_sub(pages)).find(|&p| (free >> p) & want == want)
        else {
            full();
        };
        self.free.set(free & !(want << first));
        self.held[first].set(1);
        unsafe { self.start.get().add(first * PAGE) }
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

    /// End the task: allocations go to the system again.
    pub fn end(&self) {
        self.start.set(std::ptr::null_mut());
    }
}

pub struct Pages;

unsafe impl GlobalAlloc for Pages {
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
            } else {
                unsafe { System.dealloc(ptr, layout) }
            }
        })
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        STATE.with(|state| {
            if !state.ours(ptr) {
                return unsafe { System.realloc(ptr, layout, size) };
            }
            // The last allocation cut grows where it is, inside its page.
            let at = ptr as usize - state.start.get() as usize;
            let page = state.current.get();
            if layout.size() <= PAGE
                && at == state.last.get()
                && at + size <= (page + 1) * PAGE
                && at / PAGE == page
            {
                state.offset.set(at + size - page * PAGE);
                return ptr;
            }
            let moved = state.take(size, layout.align());
            unsafe { std::ptr::copy_nonoverlapping(ptr, moved, layout.size().min(size)) };
            state.give(ptr, layout.size());
            moved
        })
    }
}
