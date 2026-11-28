//! The `yes` executor: a task may run on a thread of its own
//! ([ADR-055](../../../../docs/specification/adr/adr-055.md) §6 step 1's second
//! half, [ADR-037](../../../../docs/specification/adr/adr-037.md) D2).
//!
//! **A second executor and not a use of the pool that was already there.**
//! `rayon`'s pool is a work-stealing pool for **closures**: `join` and
//! `par_iter` hand it a `FnOnce`, it runs it to the end, and there is no point
//! at which a closure gives its thread up. A task can pause, so what a pool
//! would have to hold for one is a future to be polled again later, which is
//! not what a closure pool is for. So this is a queue of futures over the same
//! worker count (`user-pool`), and `rayon` keeps the work it is good at:
//! `fs::map`'s UTF-8 check and the parallel piece driver.
//!
//! **What is different from [`super::exec`]'s one thread** is all one
//! difference — a task may be polled on a thread that did not start it — and
//! three things follow from it:
//!
//! * a task's future must be `Send` (§2 D6), which the compiler refuses before
//!   this can;
//! * a waker has to reach the queue from any thread, so a task is an
//!   `Arc<Cell>` a waker pushes back rather than a flag one loop reads;
//! * and **exactly one thread at a time may wait in the I/O**, because the ring
//!   is behind a lock and a thread inside `io_uring_enter` is woken by the
//!   kernel and by nothing else. That thread is the **pilot**; every other
//!   worker waits on this pool's own bell.

use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Waker};

/// A task the pool owns. `Send`, which is the whole of what `yes` asks of a
/// task that `no` does not.
type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

/// **A pinned task before its first poll** (#2830): what builds its future,
/// on the worker that will poll it to its end. The future need not be `Send`,
/// since it never leaves that worker; this closure is what crosses.
type Make = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()>>> + Send>;

/// A pinned task's future, which never leaves its worker.
type Local = Pin<Box<dyn Future<Output = ()>>>;

/// No worker: a pinned task not yet claimed, and the pilot's seat empty.
const NOWHERE: usize = usize::MAX;

std::thread_local! {
    /// Which worker of the pool this thread is, [`NOWHERE`] off the pool.
    static WORKER: std::cell::Cell<usize> = const { std::cell::Cell::new(NOWHERE) };
    /// **The pinned tasks this worker polls** (#2830), by cell id: kept
    /// here, on the thread, because a future that is not `Send` may not be
    /// in the cell another thread can reach.
    static PINNED: std::cell::RefCell<BTreeMap<u64, Local>> = const { std::cell::RefCell::new(BTreeMap::new()) };
}

/// Where a task is, as one atomic.
///
/// The states exist to answer one question: **what happens to a wake that
/// arrives while the task is being polled?** Without [`RUNNING_WOKEN`] it would
/// be dropped — the waker would see `RUNNING` and decline to queue, correctly,
/// since the future is out of the cell and a second worker must not poll it —
/// and then nothing would put it back. So a wake during a poll is *recorded*,
/// and the worker that finishes the poll re-queues instead of going idle.
const IDLE: u8 = 0;
const QUEUED: u8 = 1;
const RUNNING: u8 = 2;
const RUNNING_WOKEN: u8 = 3;

/// One task: its future, where it is, and the pool to push it back onto.
///
/// The future lives in a `Mutex<Option<…>>` rather than beside the queue,
/// because a waker is a thing anybody may hold — an I/O reply, another task's
/// `Slot`, a thread this pool never heard of — and all it has to be able to do
/// is say *"this one is ready again"* without owning what it names.
struct Cell {
    id: u64,
    state: AtomicU8,
    future: Mutex<Option<Task>>,
    pool: std::sync::Weak<Pool>,
    /// **A task that runs on one worker to its end** (#2830, ADR-async-v6
    /// D20 at `yes`): its future is not `Send`, so `future` stays empty, the
    /// first worker to take it builds the future from `make` and keeps it in
    /// [`PINNED`], and every later wake queues it on that worker alone.
    pinned: bool,
    make: Mutex<Option<Make>>,
    /// The worker a pinned task is on, [`NOWHERE`] before its first poll.
    home: AtomicUsize,
}

impl Cell {
    /// Say this task is ready, from any thread.
    fn wake(self: &Arc<Self>) {
        self.wake_from(true);
    }

    /// The same; `woken` is a wake somebody's waker gave rather than a
    /// rearm's, and only such a wake reaches a pilot in the kernel's wait
    /// (see [`Pool::push_from`]).
    fn wake_from(self: &Arc<Self>, woken: bool) {
        loop {
            match self.state.load(Ordering::Acquire) {
                IDLE => {
                    if self
                        .state
                        .compare_exchange(IDLE, QUEUED, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        if let Some(pool) = self.pool.upgrade() {
                            pool.push_from(self.clone(), woken);
                        }
                        return;
                    }
                }
                RUNNING => {
                    if self
                        .state
                        .compare_exchange(
                            RUNNING,
                            RUNNING_WOKEN,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                    {
                        return;
                    }
                }
                // Already queued, or already recorded as woken while running.
                // Either way somebody is going to poll it.
                _ => return,
            }
        }
    }
}

/// A `Waker` over a [`Cell`]: `std`'s own, through [`std::task::Wake`], so
/// the count on the `Arc` is `std`'s to keep.
fn waker_for(cell: Arc<Cell>) -> Waker {
    Waker::from(cell)
}

impl std::task::Wake for Cell {
    fn wake(self: Arc<Self>) {
        Cell::wake(&self);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        Cell::wake(self);
    }
}

/// How long a worker that is not piloting waits on the bell before looking
/// again.
///
/// **Not a poll interval, and nothing depends on it for correctness.**
/// Everything that makes a task ready pushes it and rings this bell, so a
/// waiting worker is woken with no delay. The bound is there because the
/// *pilot* is inside the kernel's wait, where this bell cannot reach it: if the
/// pilot is the thread that will notice the completion, the others should look
/// again rather than sleep on a bell nobody is going to ring.
const SLICE: std::time::Duration = std::time::Duration::from_millis(20);

/// The pool: every live task, the ready ones, and the threads that drain them.
pub struct Pool {
    /// **Every task started and not finished.** The pool owns them, the way
    /// `exec`'s queue owns its own: a task that returned `Pending` is held by
    /// its wakers alone otherwise, and an I/O future stores no waker (D3's
    /// design, `rt::io::Reading`) — so without this a task waiting on a read
    /// would be dropped rather than polled again.
    live: Mutex<BTreeMap<u64, Arc<Cell>>>,
    /// The ones that say they are ready now.
    ready: Mutex<VecDeque<Arc<Cell>>>,
    /// **Each worker's own ready pinned tasks** (#2830), by worker number.
    homes: Vec<Mutex<VecDeque<Arc<Cell>>>>,
    /// The worker in the pilot's seat, [`NOWHERE`] when nobody is.
    pilot_at: AtomicUsize,
    /// Rung on every push, when a task finishes, and when the pilot comes back.
    bell: Condvar,
    next: AtomicU64,
    /// Held by whichever worker is waiting in the I/O. Exactly one: a second
    /// thread would block on the ring's lock rather than take work.
    piloting: AtomicBool,
    closing: AtomicBool,
    /// How many workers actually started — counted rather than asked for, so
    /// `--runtime` reports what this process got.
    threads: std::sync::atomic::AtomicUsize,
    /// The earliest time a task on this pool asked to be polled again
    /// (`rt::timer::wake_at`, a `sleep`). The timer cell is per thread and a
    /// task may be polled on any worker, so each poll hands its time over here.
    timer: Mutex<Option<std::time::Instant>>,
    /// How often an idle turn woke anybody: the test that an idle pool stays
    /// quiet (#693) counts it. Counted in every build (#2316): the increment
    /// sits beside a condvar broadcast on a path that has just left the ring.
    rang: std::sync::atomic::AtomicI64,
    /// **The completion count as of the last rearm** (#2729). Any thread
    /// that holds the ring reaps every completion on it - a task's own poll
    /// or submit as much as the pilot's park - and moves the count
    /// (`Runtime::with_held`). Only the pool can turn that into a poll of
    /// the task whose answer it was, since an I/O future keeps no waker. So
    /// an idle turn that finds the count past this rearms, whoever moved it.
    seen: std::sync::atomic::AtomicI64,
}

impl Pool {
    /// Start `threads` workers. `0` is `user-pool`'s documented default — as
    /// many as the machine has — and not a count somebody typed.
    pub fn start(threads: usize) -> Arc<Pool> {
        let threads = match threads {
            0 => std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            n => n,
        };
        let pool = Arc::new(Pool {
            live: Mutex::new(BTreeMap::new()),
            ready: Mutex::new(VecDeque::new()),
            homes: (0..threads).map(|_| Mutex::new(VecDeque::new())).collect(),
            pilot_at: AtomicUsize::new(NOWHERE),
            bell: Condvar::new(),
            next: AtomicU64::new(0),
            piloting: AtomicBool::new(false),
            closing: AtomicBool::new(false),
            threads: std::sync::atomic::AtomicUsize::new(0),
            timer: Mutex::new(None),
            rang: std::sync::atomic::AtomicI64::new(0),
            seen: std::sync::atomic::AtomicI64::new(crate::rt::io::generation()),
        });
        for n in 0..threads {
            let mine = pool.clone();
            // **Detached, and no handle is kept.** A worker leaves when
            // `close` is called, and ADR-297 D6's drain is about the *tasks*
            // rather than about the threads — `block_on` has already waited for
            // the work by the time anything closes this.
            let started = std::thread::Builder::new()
                .name(format!("nikaia-task-{n}"))
                .spawn(move || mine.work(n));
            if started.is_err() {
                // A machine that will not give us a thread is a machine this
                // program runs on with fewer of them. Refusing to start would
                // turn a resource limit into a failure to run at all.
                break;
            }
            pool.threads.fetch_add(1, Ordering::AcqRel);
        }
        pool
    }

    /// Start a task. It runs whether or not anybody joins it (ADR-055 D5).
    pub fn start_task(self: &Arc<Pool>, future: impl Future<Output = ()> + Send + 'static) {
        let id = self.next.fetch_add(1, Ordering::AcqRel);
        let cell = Arc::new(Cell {
            id,
            state: AtomicU8::new(QUEUED),
            future: Mutex::new(Some(Box::pin(future))),
            pool: Arc::downgrade(self),
            pinned: false,
            make: Mutex::new(None),
            home: AtomicUsize::new(NOWHERE),
        });
        self.live
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, cell.clone());
        self.push(cell);
    }

    /// **Start a task that runs on one worker to its end** (#2830): `make`
    /// builds its future on the first worker free to take it, and that worker
    /// alone polls it from then on. A group's runtime is a one-thread thing by
    /// design (ADR-async-v6 D20, ADR-328 D10), so a future that holds one is
    /// not `Send`; this is how such a task runs at `yes` without making it so.
    pub fn start_on_pool_pinned<F: Future<Output = ()> + 'static>(
        self: &Arc<Pool>,
        make: impl FnOnce() -> F + Send + 'static,
    ) {
        let id = self.next.fetch_add(1, Ordering::AcqRel);
        let make: Make = Box::new(move || Box::pin(make()) as Local);
        let cell = Arc::new(Cell {
            id,
            state: AtomicU8::new(QUEUED),
            future: Mutex::new(None),
            pool: Arc::downgrade(self),
            pinned: true,
            make: Mutex::new(Some(make)),
            home: AtomicUsize::new(NOWHERE),
        });
        self.live
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, cell.clone());
        self.push(cell);
    }

    /// How many worker threads this pool actually got, which is what
    /// `--runtime` reports: a machine that would not give us one is a machine
    /// this program runs on with fewer of them.
    pub fn threads(&self) -> usize {
        self.threads.load(Ordering::Acquire)
    }

    /// How many tasks are started and unfinished — what the drain at the end of
    /// `main` asks about (ADR-297 D6).
    pub fn live(&self) -> usize {
        self.live.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// How often an idle turn has woken the workers since the pool started:
    /// what `runtime/run_config.nika`'s idle-pool test reads (#693, #2316).
    pub(crate) fn rang(&self) -> i64 {
        self.rang.load(Ordering::Relaxed)
    }

    /// Stop the workers.
    ///
    /// Whether anything is left to run is decided **before** this, by
    /// `exec::block_on`'s drain; by the time the runtime closes, the answer has
    /// been acted on and said out loud.
    pub fn close(&self) {
        self.closing.store(true, Ordering::Release);
        self.bell.notify_all();
    }

    fn push(&self, cell: Arc<Cell>) {
        self.push_from(cell, false);
    }

    /// Queue a ready task: an unpinned one, or a pinned one before its first
    /// poll, where any worker takes it; a pinned one on its worker's own queue.
    ///
    /// **A pinned task whose worker is the pilot** is a wake the bell cannot
    /// carry, since the pilot is in the kernel's wait; so a wake from a waker
    /// on another thread rings the ring. A rearm does not: every rearm would
    /// otherwise ring the ring whose completion is the next rearm, and two
    /// workers would wake each other for ever (#693).
    fn push_from(&self, cell: Arc<Cell>, woken: bool) {
        let home = if cell.pinned {
            cell.home.load(Ordering::Acquire)
        } else {
            NOWHERE
        };
        if home == NOWHERE {
            self.ready
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push_back(cell);
            self.bell.notify_one();
            return;
        }
        if let Some(queue) = self.homes.get(home) {
            queue
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push_back(cell);
        }
        // Every waiter, since the one this is for is one of them; under the
        // shared lock, which `wait` holds while it looks at its own queue.
        drop(self.ready.lock().unwrap_or_else(|e| e.into_inner()));
        self.bell.notify_all();
        // **Seen by the pilot or seeing it** (with the fence in `idle`): the
        // pilot takes its seat, then looks at its own queue; this pushes,
        // then looks at the seat.
        std::sync::atomic::fence(Ordering::SeqCst);
        if woken && self.pilot_at.load(Ordering::SeqCst) == home && WORKER.with(|w| w.get()) != home
        {
            crate::rt::ring_the_bell();
        }
    }

    /// Whether this worker's own pinned queue holds a ready task.
    fn mine_ready(&self) -> bool {
        let me = WORKER.with(|w| w.get());
        self.homes
            .get(me)
            .is_some_and(|q| !q.lock().unwrap_or_else(|e| e.into_inner()).is_empty())
    }

    /// One worker's whole life.
    fn work(self: Arc<Pool>, me: usize) {
        WORKER.with(|w| w.set(me));
        loop {
            // **Its own pinned tasks first**: nobody else may poll them.
            let mine = self
                .homes
                .get(me)
                .and_then(|q| q.lock().unwrap_or_else(|e| e.into_inner()).pop_front());
            let next = mine.or_else(|| {
                self.ready
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .pop_front()
            });
            if let Some(cell) = next {
                self.run(cell);
                continue;
            }
            if self.closing.load(Ordering::Acquire) {
                return;
            }
            self.idle();
        }
    }

    /// Poll one pinned task once, on its worker, which this is (#2830).
    fn run_pinned(&self, cell: Arc<Cell>) {
        cell.state.store(RUNNING, Ordering::Release);
        let me = WORKER.with(|w| w.get());
        let taken = if cell.home.load(Ordering::Acquire) == NOWHERE {
            // **Its first poll: this worker is its worker from now on.** The
            // `make` is taken once, so a second pop of the same cell finds
            // nothing.
            let make = cell.make.lock().unwrap_or_else(|e| e.into_inner()).take();
            make.map(|make| {
                cell.home.store(me, Ordering::Release);
                make()
            })
        } else {
            PINNED.with(|pinned| pinned.borrow_mut().remove(&cell.id))
        };
        let Some(mut future) = taken else {
            return;
        };
        let waker = waker_for(cell.clone());
        let mut context = Context::from_waker(&waker);
        if future.as_mut().poll(&mut context).is_ready() {
            let _ = crate::rt::timer::taken();
            self.live
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&cell.id);
            // Dropped here, on its worker, and before the joiner is told.
            drop(future);
            self.bell.notify_all();
            crate::rt::ring_the_bell();
            return;
        }
        PINNED.with(|pinned| pinned.borrow_mut().insert(cell.id, future));
        self.settle(cell);
    }

    /// After a poll that left the task pending: hand its time to the pool,
    /// and put it back if a wake arrived while it ran.
    fn settle(&self, cell: Arc<Cell>) {
        if let Some(when) = crate::rt::timer::taken() {
            let mut timer = self.timer.lock().unwrap_or_else(|e| e.into_inner());
            if timer.is_none_or(|already| when < already) {
                *timer = Some(when);
            }
        }
        if cell
            .state
            .compare_exchange(RUNNING, IDLE, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            cell.state.store(QUEUED, Ordering::Release);
            self.push(cell);
        }
    }

    /// Poll one task once.
    fn run(&self, cell: Arc<Cell>) {
        if cell.pinned {
            self.run_pinned(cell);
            return;
        }
        cell.state.store(RUNNING, Ordering::Release);
        // **Taken out of the cell for the poll**, which is what makes it
        // impossible for two workers to poll one future: the cell is empty
        // while it runs, so a second pop of the same `Arc` finds nothing and
        // drops it. The state machine is what stops a wake being lost with it.
        let taken = cell.future.lock().unwrap_or_else(|e| e.into_inner()).take();
        let Some(mut future) = taken else {
            return;
        };
        let waker = waker_for(cell.clone());
        let mut context = Context::from_waker(&waker);
        if future.as_mut().poll(&mut context).is_ready() {
            // A time asked for by a future that has now finished is no wake.
            let _ = crate::rt::timer::taken();
            self.live
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&cell.id);
            // **A task finishing is a cross-thread wake.** Whoever joined it is
            // on another thread — at `yes` that is usually `main` — and the
            // runtime's bell is what `main` waits on there.
            self.bell.notify_all();
            crate::rt::ring_the_bell();
            return;
        }
        *cell.future.lock().unwrap_or_else(|e| e.into_inner()) = Some(future);
        // Back to idle, unless a wake arrived while it was running — in which
        // case this worker is the one that has to put it back, because the
        // waker saw `RUNNING` and declined to.
        self.settle(cell);
    }

    /// Nothing was ready. Wait for something, in the one way that is allowed.
    ///
    /// **Only something that happened wakes anybody** (#693). A park that timed
    /// out, or that found nothing outstanding, rearms nothing and rings
    /// nothing: an idle pool that woke itself on every turn kept every thread
    /// of it, and `main`, busy for nothing. What does wake it is a completion
    /// (the park answers `true`), a task's time arriving, or the pool closing.
    fn idle(self: &Arc<Pool>) {
        // **A time that has come is a wake like any other**, and any worker
        // may notice it: nothing posts it, so it is looked at on every turn.
        if self.timer_due() {
            self.rearm();
            return;
        }
        // **A completion somebody else reaped is a wake too** (#2729). A
        // task's poll on another worker took the ring, reaped this task's
        // answer with its own and moved the count, so the count had moved
        // before the pilot below read its generation: its park saw no move,
        // nothing rearmed, and the task whose answer it was waited for ever
        // (eight pool tasks writing and reading their own files, about one
        // run in 25 under load).
        if self.moved_since_rearm() {
            self.rearm();
            return;
        }
        let slice = self.slice();
        let piloting = self
            .piloting
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();
        if piloting {
            self.pilot_at
                .store(WORKER.with(|w| w.get()), Ordering::SeqCst);
            std::sync::atomic::fence(Ordering::SeqCst);
        }
        if !piloting {
            // A wait that times out returns to the loop and tries the pilot's
            // seat again, which is how a non-pilot takes over within a slice.
            self.wait(slice);
            return;
        }

        // **The generation is read before the queue is checked**, which is the
        // ordering `exec::block_on` keeps for the same reason: a completion
        // arriving between the two has already moved the count, so the park
        // returns at once rather than waiting for a wake that has happened.
        // **And it is the count as of the last rearm**, not the count now
        // (#2729): a move between the check above and this read was a
        // completion another thread reaped, and a park from the count now
        // would not hear it.
        let generation = self.seen.load(Ordering::Acquire);
        let empty = self
            .ready
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
            && !self.mine_ready();
        let closing = self.closing.load(Ordering::Acquire);
        let mut moved = false;
        let mut left = None;
        if empty && !closing {
            let parked = std::time::Instant::now();
            moved = crate::rt::io::park_for(generation, Some(slice));
            if moved {
                // **Everybody gets another turn.** Which task was waiting for
                // this completion is not something the pool knows: an I/O
                // future stores no waker, because the executor is the only
                // thing that parks. So the answer to "who should be polled
                // now" is everyone.
                let _ = self.moved_since_rearm();
                self.rearm();
                // **No bell of its own.** Whoever is on the main thread, which
                // at `yes` waits on the runtime's bell, has heard already: an
                // answer this park reaped moved the count under the ring's lock
                // (`Runtime::with_ring`), and a count that moved is somebody
                // else's ring. Ringing again here rang the ring's `eventfd`,
                // whose completion the next park reaped as one more move, so a
                // pilot with any I/O outstanding woke itself for ever (#693).
            } else {
                // `false` before the slice is up means nothing was outstanding,
                // so the park did not wait at all: the rest of the slice is
                // waited out below, off the pilot's seat.
                left = slice.checked_sub(parked.elapsed());
            }
        }
        self.pilot_at.store(NOWHERE, Ordering::SeqCst);
        self.piloting.store(false, Ordering::Release);
        if moved || closing {
            self.rang.fetch_add(1, Ordering::Relaxed);
            self.bell.notify_all();
        } else if let Some(left) = left {
            // **Off the seat, so the next worker to go idle takes it** — the one
            // whose task has just started the I/O this pilot found missing, which
            // then parks in it at once rather than a slice later.
            self.wait(left);
        }
    }

    /// Wait on the pool's own bell for at most `limit`, unless there is
    /// already something to do.
    fn wait(&self, limit: std::time::Duration) {
        // **Its own queue is looked at under the shared lock**, which a pinned
        // push takes before it rings: so the push is seen here, or its bell
        // finds this thread waiting.
        let ready = self.ready.lock().unwrap_or_else(|e| e.into_inner());
        if !ready.is_empty() || self.closing.load(Ordering::Acquire) || self.mine_ready() {
            return;
        }
        let _ = self
            .bell
            .wait_timeout(ready, limit)
            .unwrap_or_else(|e| e.into_inner());
    }

    /// How long an idle turn may wait: a slice, or less if a task's time comes
    /// sooner.
    fn slice(&self) -> std::time::Duration {
        match *self.timer.lock().unwrap_or_else(|e| e.into_inner()) {
            Some(when) => when
                .saturating_duration_since(std::time::Instant::now())
                .min(SLICE),
            None => SLICE,
        }
    }

    /// Whether a task's time has come, clearing it if so: the task that is
    /// still sleeping after its poll leaves its time again.
    fn timer_due(&self) -> bool {
        let mut timer = self.timer.lock().unwrap_or_else(|e| e.into_inner());
        match *timer {
            Some(when) if when <= std::time::Instant::now() => {
                *timer = None;
                true
            }
            _ => false,
        }
    }

    /// Whether the completion count moved since the last rearm, taking the
    /// count as seen if so. Taken **before** the rearm it calls for: a
    /// completion reaped during that rearm's polls moves the count past it
    /// again, and the next idle turn rearms once more.
    fn moved_since_rearm(&self) -> bool {
        let now = crate::rt::io::generation();
        self.seen.fetch_max(now, Ordering::AcqRel) < now
    }

    /// Put every idle task back on the queue.
    ///
    /// `wake` decides what that means per task: one already queued stays where
    /// it is, and one being polled is marked instead, so this never queues the
    /// same future twice.
    fn rearm(&self) {
        let live: Vec<Arc<Cell>> = self
            .live
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect();
        for cell in live {
            cell.wake_from(false);
        }
    }
}
