//! **`fs::read` hands its vector over** (ADR-373 D2): reading a file of a
//! megabyte allocates the megabyte once, not a second time for the `Bytes`.
//! A test binary of its own, because the counting allocator is the process's.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call is forwarded to `System` unchanged; the count is a side
// note.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        // SAFETY: forwarded as it came.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forwarded as it came.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATED.fetch_add(new_size, Ordering::SeqCst);
        // SAFETY: forwarded as it came.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

#[test]
fn a_read_allocates_the_file_once() {
    const SIZE: usize = 1 << 20;
    let dir = std::env::temp_dir().join(format!("nikaia-read-once-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch");
    std::fs::write(dir.join("big"), vec![b'x'; SIZE]).expect("write");
    let root = nikaia_std::fs::Root::Dir(dir.clone());
    let before = ALLOCATED.load(Ordering::SeqCst);
    let bytes = nikaia_std::rt::exec::block_on(nikaia_std::fs::read("big", &root)).expect("reads");
    let allocated = ALLOCATED.load(Ordering::SeqCst) - before;
    assert_eq!(bytes.len(), SIZE);
    assert!(
        allocated < SIZE + SIZE / 2,
        "{allocated} bytes allocated to read {SIZE}: the file was copied"
    );
    std::fs::remove_dir_all(&dir).ok();
}
