//! Pages with a count of what each holds inside a task's block
//! (`src/pages.rs`, ADR-327 D10): a bump pointer through a page, the page
//! started over or given back when its count drops to nothing.

#[path = "../pages.rs"]
mod pages;
#[path = "../workload.rs"]
mod workload;

use std::alloc::{GlobalAlloc, Layout, System};

#[global_allocator]
static HEAP: pages::Pages = pages::Pages;

/// The one block, reused by every task on this thread.
struct PerTask {
    block: *mut u8,
}

impl workload::Tasks for PerTask {
    fn begin(&self) {
        pages::STATE.with(|state| state.begin(self.block));
    }
    fn end(&self) {
        pages::STATE.with(|state| state.end());
    }
}

fn main() {
    let layout = Layout::from_size_align(pages::BLOCK, 16).expect("a block's layout");
    let block = unsafe { System.alloc(layout) };
    assert!(!block.is_null());
    workload::run(workload::tasks(), &PerTask { block });
    unsafe { System.dealloc(block, layout) };
}
