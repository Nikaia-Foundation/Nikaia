//! **A block that gives freed memory back to its task** (ADR-327 D10): the
//! block is cut into size classes (powers of two, 16 bytes to the whole
//! block), and a freed piece goes onto its class's free list inside the
//! block, where the next allocation of that class takes it. A full block ends
//! the task (D6). Outside a task the system allocator answers.
//!
//! `BUMP_FIRST` chooses where an allocation looks first:
//! - `false` (`reuse`): the free list, then the untouched rest of the block;
//! - `true` (`bumpreuse`): the untouched rest, as the bump block does, and
//!   the free lists only once the block is full. Frees are kept either way,
//!   so the switch loses nothing.
//!
//! Where a class's list is empty and the untouched rest too short, a free
//! piece of a larger class is split. Pieces are not merged again.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// The block's size: what a task may hold.
pub const BLOCK: usize = 64 * 1024;

/// The smallest class, which is also the alignment every piece has.
const SMALLEST: usize = 16;

/// Classes 16, 32, ... `BLOCK`.
const CLASSES: usize = (BLOCK / SMALLEST).trailing_zeros() as usize + 1;

/// A task's block, kept per thread.
pub struct State {
    /// Where the block starts, null outside a task.
    start: Cell<*mut u8>,
    /// How much of the block was ever handed out.
    taken: Cell<usize>,
    /// Where the last piece cut from the untouched rest starts, which may
    /// grow where it is.
    last: Cell<usize>,
    /// The head of each class's free list; a free piece's first word points
    /// to the next.
    free: [Cell<*mut u8>; CLASSES],
}

thread_local! {
    pub static STATE: State = const {
        State {
            start: Cell::new(std::ptr::null_mut()),
            taken: Cell::new(0),
            last: Cell::new(usize::MAX),
            free: [const { Cell::new(std::ptr::null_mut()) }; CLASSES],
        }
    };
}

/// The class of `size` bytes, and the class's size.
#[inline(always)]
fn class(size: usize) -> (usize, usize) {
    let size = size.max(SMALLEST);
    let index =
        (usize::BITS - (size - 1).leading_zeros()) as usize - SMALLEST.trailing_zeros() as usize;
    (index, SMALLEST << index)
}

#[cold]
#[inline(never)]
fn full() -> ! {
    panic!("the task's block is full");
}

impl State {
    /// Cut `size` bytes aligned to `align` from the untouched rest, or null
    /// where it is too short.
    #[inline(always)]
    fn cut(&self, size: usize, align: usize) -> *mut u8 {
        let start = self.start.get();
        let taken = self.taken.get();
        let at = (start as usize + taken).next_multiple_of(align) - start as usize;
        if at + size > BLOCK {
            return std::ptr::null_mut();
        }
        self.taken.set(at + size);
        self.last.set(at);
        unsafe { start.add(at) }
    }

    /// Take the head of class `index`'s free list, or null.
    #[inline(always)]
    fn pop(&self, index: usize) -> *mut u8 {
        let head = self.free[index].get();
        if !head.is_null() {
            self.free[index].set(unsafe { *(head as *mut *mut u8) });
        }
        head
    }

    /// Split a free piece of a larger class for class `index`: one half is
    /// split again until it has the class's size, each other half goes onto
    /// its own class's list. Null where no larger piece is free.
    #[cold]
    fn split(&self, index: usize) -> *mut u8 {
        let Some(larger) = (index + 1..CLASSES).find(|&j| !self.free[j].get().is_null()) else {
            return std::ptr::null_mut();
        };
        let piece = self.pop(larger);
        for j in (index..larger).rev() {
            let half = unsafe { piece.add(SMALLEST << j) };
            unsafe { *(half as *mut *mut u8) = self.free[j].get() };
            self.free[j].set(half);
        }
        piece
    }

    /// Put a piece of class `index` onto its free list; the last piece cut
    /// gives its bytes back to the untouched rest instead.
    #[inline(always)]
    fn push(&self, ptr: *mut u8, index: usize, size: usize) {
        let at = ptr as usize - self.start.get() as usize;
        if at == self.last.get() && at + size == self.taken.get() {
            self.taken.set(at);
            self.last.set(usize::MAX);
            return;
        }
        unsafe { *(ptr as *mut *mut u8) = self.free[index].get() };
        self.free[index].set(ptr);
    }

    /// A piece of `layout`'s class from the running task's block.
    #[inline(always)]
    fn take<const BUMP_FIRST: bool>(&self, layout: Layout) -> *mut u8 {
        let (index, size) = class(layout.size());
        let align = layout.align().max(SMALLEST);
        // A piece on a free list is aligned to `SMALLEST` only.
        let listed = align == SMALLEST;
        let ptr = if BUMP_FIRST {
            let cut = self.cut(size, align);
            if cut.is_null() && listed {
                self.pop(index)
            } else {
                cut
            }
        } else {
            let popped = if listed {
                self.pop(index)
            } else {
                std::ptr::null_mut()
            };
            if popped.is_null() {
                self.cut(size, align)
            } else {
                popped
            }
        };
        if !ptr.is_null() {
            return ptr;
        }
        let split = if listed {
            self.split(index)
        } else {
            std::ptr::null_mut()
        };
        if split.is_null() {
            full();
        }
        split
    }

    /// Whether `ptr` is in the running task's block.
    #[inline(always)]
    fn ours(&self, ptr: *mut u8) -> bool {
        let start = self.start.get();
        !start.is_null() && ptr >= start && (ptr as usize) < start as usize + BLOCK
    }

    /// Start a task on `block`: everything in it is free.
    pub fn begin(&self, block: *mut u8) {
        self.start.set(block);
        self.taken.set(0);
        self.last.set(usize::MAX);
        for head in &self.free {
            head.set(std::ptr::null_mut());
        }
    }

    /// End the task: allocations go to the system again.
    pub fn end(&self) {
        self.start.set(std::ptr::null_mut());
    }
}

pub struct Classes<const BUMP_FIRST: bool>;

unsafe impl<const BUMP_FIRST: bool> GlobalAlloc for Classes<BUMP_FIRST> {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        STATE.with(|state| {
            if state.start.get().is_null() {
                unsafe { System.alloc(layout) }
            } else {
                state.take::<BUMP_FIRST>(layout)
            }
        })
    }
    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        STATE.with(|state| {
            if state.ours(ptr) {
                let (index, size) = class(layout.size());
                state.push(ptr, index, size);
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
            let (index, had) = class(layout.size());
            let (_, wants) = class(size);
            // The same class: the piece already has room. A smaller class
            // moves too, so that a piece's class is always its layout's and
            // `dealloc` puts it back on the right list.
            if wants == had {
                return ptr;
            }
            // The last piece cut grows where it is, while the untouched rest
            // has room; else it moves, perhaps onto a free piece.
            let at = ptr as usize - state.start.get() as usize;
            if wants > had
                && at == state.last.get()
                && at + had == state.taken.get()
                && at + wants <= BLOCK
            {
                state.taken.set(at + wants);
                return ptr;
            }
            let layout_now = unsafe { Layout::from_size_align_unchecked(size, layout.align()) };
            let moved = state.take::<BUMP_FIRST>(layout_now);
            unsafe { std::ptr::copy_nonoverlapping(ptr, moved, layout.size().min(size)) };
            state.push(ptr, index, had);
            moved
        })
    }
}
