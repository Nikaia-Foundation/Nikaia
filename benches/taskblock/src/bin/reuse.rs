//! Size classes with free lists inside a task's block (`src/classes.rs`,
//! ADR-327 D10), taking from a free list first and cutting from the
//! untouched rest only where the list is empty.

#[path = "../classes.rs"]
mod classes;
#[path = "../workload.rs"]
mod workload;

use std::alloc::{GlobalAlloc, Layout, System};

#[global_allocator]
static HEAP: classes::Classes<false> = classes::Classes;

/// The one block, reused by every task on this thread.
struct PerTask {
    block: *mut u8,
}

impl workload::Tasks for PerTask {
    fn begin(&self) {
        classes::STATE.with(|state| state.begin(self.block));
    }
    fn end(&self) {
        classes::STATE.with(|state| state.end());
    }
}

fn main() {
    let layout = Layout::from_size_align(classes::BLOCK, 16).expect("a block's layout");
    let block = unsafe { System.alloc(layout) };
    assert!(!block.is_null());
    workload::run(workload::tasks(), &PerTask { block });
    unsafe { System.dealloc(block, layout) };
}
