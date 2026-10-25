//! **A `Bytes` handed to a parameter that keeps it** (ADR-373 D1, D4): a
//! `ref Bytes` the callee keeps is a `&Bytes`, and the lowering writes
//! `&*nikaia_std::bytes::kept(arg)` at each call. A `Bytes` and a mapping
//! reach it without a copy of their bytes; a run of bytes is copied once. A
//! test binary of its own, because the counting allocator is the process's.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use nikaia_std::bytes::{Bytes, kept};

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

const SIZE: usize = 1 << 20;

/// What a keeping callee does with its `&Bytes`: stores a clone.
fn store(b: &Bytes) -> Bytes {
    b.clone()
}

#[test]
fn a_bytes_and_a_mapping_are_kept_without_a_copy() {
    let dir = std::env::temp_dir().join(format!("nikaia-kept-bytes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch");
    std::fs::write(dir.join("big"), vec![b'x'; SIZE]).expect("write");
    let root = nikaia_std::fs::Root::Dir(dir.clone());
    let own = Bytes::from(vec![b'y'; SIZE]);
    let mapped = nikaia_std::rt::exec::block_on(nikaia_std::fs::map("big", &root)).expect("maps");

    let before = ALLOCATED.load(Ordering::SeqCst);
    let from_own = store(&kept(&own));
    let from_mapped = store(&kept(&mapped));
    let allocated = ALLOCATED.load(Ordering::SeqCst) - before;
    assert!(
        allocated < SIZE / 2,
        "{allocated} bytes allocated to keep two runs of {SIZE}: one was copied"
    );
    assert_eq!(from_own.as_ptr(), own.as_ptr());
    assert_eq!(from_mapped.as_ptr(), mapped.deref().as_ptr());
    assert_eq!((from_own.len(), from_mapped.len()), (SIZE, SIZE));

    // A run of bytes with no owner - a text's, a cut - is copied into one.
    let text = "z".repeat(SIZE);
    let from_text = store(&kept(&text.as_bytes()));
    assert_ne!(from_text.as_ptr(), text.as_ptr());
    assert_eq!(from_text.len(), SIZE);
    drop(mapped);
    std::fs::remove_dir_all(&dir).ok();
}

/// What a keeping method does (#842): `Holder::put` stores a clone in `self`,
/// as the lowering of `fn put(ref mut self, b: ref Bytes)` writes it.
struct Holder {
    body: Bytes,
}

impl Holder {
    fn put(&mut self, b: &Bytes) {
        self.body = b.clone();
    }
}

#[test]
fn a_method_keeps_a_bytes_and_a_mapping_without_a_copy() {
    let dir = std::env::temp_dir().join(format!("nikaia-kept-method-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch");
    std::fs::write(dir.join("big"), vec![b'x'; SIZE]).expect("write");
    let root = nikaia_std::fs::Root::Dir(dir.clone());
    let own = Bytes::from(vec![b'y'; SIZE]);
    let mapped = nikaia_std::rt::exec::block_on(nikaia_std::fs::map("big", &root)).expect("maps");
    let mut h = Holder {
        body: Bytes::from(Vec::new()),
    };
    h.put(&kept(&own));
    assert_eq!(h.body.as_ptr(), own.as_ptr());
    h.put(&kept(&mapped));
    assert_eq!(h.body.as_ptr(), mapped.deref().as_ptr());
    let text = "z".repeat(SIZE);
    h.put(&kept(&text.as_bytes()));
    assert_ne!(h.body.as_ptr(), text.as_ptr());
    assert_eq!(h.body.len(), SIZE);
    drop(mapped);
    std::fs::remove_dir_all(&dir).ok();
}
