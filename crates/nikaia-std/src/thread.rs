//! `std::thread` - the name of the calling thread
//! ([ADR-401](../../../docs/specification/adr/adr-401.md)).
//!
//! `thread::Id` is Nikaia, `runtime/thread.nika`. What stays Rust is what
//! Nikaia has no word for (#557): the process-wide counter and the per-thread
//! slot, a `const` thread-local without a destructor, so a thread that never
//! asks costs nothing (D2).

use std::cell::Cell;
use std::sync::atomic::{AtomicI64, Ordering};

/// The name of a thread: compared and hashed, nothing else (ADR-401 D1).
pub use crate::runtime::thread::Id;

/// The next number to hand out; never handed out twice (ADR-401 D2).
static NEXT: AtomicI64 = AtomicI64::new(1);

std::thread_local! {
    /// The calling thread's number, 0 until it first asks.
    static MINE: Cell<i64> = const { Cell::new(0) };
}

/// **The calling thread's id** (ADR-401 D1): taken from the counter on the
/// thread's first call and kept for its life (D2). It names the thread, not a
/// task, which may be resumed on another thread (D3).
pub fn id() -> Id {
    let number = MINE.with(|mine| {
        let mut number = mine.get();
        if number == 0 {
            number = NEXT.fetch_add(1, Ordering::Relaxed);
            mine.set(number);
        }
        number
    });
    crate::runtime::thread::thread_numbered(number)
}
