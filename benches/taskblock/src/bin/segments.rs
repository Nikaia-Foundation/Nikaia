//! The pages as the current segment of a chain up to a budget
//! (`src/segments.rs`, ADR-327 D15): `pages`, with the next segment where the
//! current one is full.

#[path = "../segments.rs"]
mod pages;
#[path = "../workload.rs"]
mod workload;

use std::alloc::{GlobalAlloc, Layout, System};

#[global_allocator]
static HEAP: pages::Segments = pages::Segments;

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
