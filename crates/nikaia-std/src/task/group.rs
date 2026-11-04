//! **The group runtime** ([ADR-async-5](../../../../docs/specification/adr/adr-async-5.md)
//! D1, D3, D9, D11-D14, D20, D21).
//!
//! Every task is started in a group and ends before the group does (D1). A
//! group counts its tasks in start order, which is the written order, hands
//! each task's end to its receiver (D3), and keeps every failure nobody read
//! until its own end, where the first in start order is thrown and the others
//! join it, in that order (D21). `task::scope` and its three siblings in
//! `task.rs` are groups of this kind; the forms of async-5 (`async { }`,
//! `async(handle)`, `async(deferred)`) are lowered onto the same runtime by
//! #988, through the entry points below.
//!
//! **The receiver of a task's end** is its [`ChildHandle`] where it has one,
//! and the group otherwise: a task started by `spawn` on a scope, or one whose
//! handle was dropped neither joined nor cancelled (D11), hands its failure to
//! the group as an unread one.

use super::{Crashed, Outcome, caught, locked};

/// **How a task of a group ended** (D3): with a value, a failure, a crash, a
/// cancellation its handle asked for (D14), or a full memory block.
#[derive(Debug)]
pub enum Ended<T, E> {
    /// The task's value.
    Value(T),
    /// The task's body failed with this error.
    Failed(E),
    /// The task panicked; the group throws [`Crashed`] at its end as well
    /// (ADR-328 D8).
    Crashed(Crashed),
    /// The task was cancelled through its handle, or never ran because a task
    /// before it crashed. Not a failure (D22): it runs no fallback.
    Cancelled,
    /// **Reserved** for a full memory block, `task::OverBudget` (D19). Nothing
    /// produces it yet: per-task blocks are not built.
    OverBudget,
}

/// **What a group has seen of its tasks**: how many started, whether one
/// crashed, and the failures nobody read, each with its place in start order.
pub(super) struct Group<E> {
    pub(super) started: usize,
    pub(super) crashed: bool,
    pub(super) failed: Vec<(usize, E)>,
}

impl<E> Group<E> {
    pub(super) fn new() -> Self {
        Group {
            started: 0,
            crashed: false,
            failed: Vec::new(),
        }
    }

    /// The next task's place, or `None` where a crash cancelled it.
    pub(super) fn start(&mut self) -> Option<usize> {
        if self.crashed {
            return None;
        }
        self.started += 1;
        Some(self.started - 1)
    }

    /// What another group's record of one task adds to this one.
    pub(super) fn absorb(&mut self, mut own: Group<E>) {
        self.crashed |= own.crashed;
        self.failed.append(&mut own.failed);
    }
}

impl<E: From<Crashed>> Group<E> {
    pub(super) fn crash(&mut self, at: usize, crash: Crashed) {
        self.crashed = true;
        self.failed.push((at, E::from(crash)));
    }
}

/// **How a group ends** (D21): `value` where nothing failed; else the first
/// failure in start order, every later one joined to it in that order.
pub(super) fn ended<T, E: crate::error::Joined>(
    value: T,
    mut failed: Vec<(usize, E)>,
) -> Result<T, E> {
    failed.sort_by_key(|(at, _)| *at);
    let mut each = failed.into_iter().map(|(_, error)| error);
    match each.next() {
        None => Ok(value),
        Some(mut first) => {
            for later in each {
                first.joined_by(later);
            }
            Err(first)
        }
    }
}

/// **A group left by a failure** (D21): the failure, with the ones nobody read
/// joined to it in start order.
pub(super) fn left<E: crate::error::Joined>(mut first: E, mut failed: Vec<(usize, E)>) -> E {
    failed.sort_by_key(|(at, _)| *at);
    for (_, later) in failed {
        first.joined_by(later);
    }
    first
}

/// One task of a group run where it starts: its crash or its error recorded
/// as unread, since a task started this way has no other receiver.
pub(super) fn ran<E: From<Crashed>, R: Outcome<E>>(
    seen: &mut Group<E>,
    at: usize,
    body: impl FnOnce() -> R,
) {
    let mut outcome = None;
    match caught(|| outcome = Some(body())) {
        Err(crash) => seen.crash(at, crash),
        Ok(()) => {
            if let Some(error) = outcome.and_then(Outcome::failed) {
                seen.failed.push((at, error));
            }
        }
    }
}

/// **A group as its tasks and handles see it**: where a task takes its place,
/// where its crash goes, and where a failure nobody reads waits for the end.
/// Each scope of `task.rs` that hands out [`ChildHandle`]s is one.
pub trait Receiver<E> {
    /// The next task's place in start order, or `None` where a crash
    /// cancelled it.
    fn place(&self) -> Option<usize>;
    /// A task panicked: the group throws [`Crashed`] at its end.
    fn crashed(&self, at: usize, crash: Crashed);
    /// A failure nobody read, kept until the group's end (D21).
    fn unread(&self, at: usize, error: E);
}

/// **Where a child task's end waits for its handle**: filled once, taken by
/// `join`, or handed to the group when the handle is gone.
struct Slot<T, E> {
    ended: Option<Ended<T, E>>,
    joiner: Option<std::task::Waker>,
    /// The handle was dropped neither joined nor cancelled: the end goes to
    /// the group (D11).
    unclaimed: bool,
    /// The handle asked for a cancel (D14); a pausing task stops at its next
    /// pause.
    cancel: bool,
}

type Shared<T, E> = std::sync::Arc<std::sync::Mutex<Slot<T, E>>>;

fn empty_slot<T, E>() -> Shared<T, E> {
    std::sync::Arc::new(std::sync::Mutex::new(Slot {
        ended: None,
        joiner: None,
        unclaimed: false,
        cancel: false,
    }))
}

/// **A task's end, handed to its receiver**: the handle if it still wants it,
/// else the group, which keeps a failure as unread and drops the rest.
fn settle<T, E>(group: &dyn Receiver<E>, slot: &Shared<T, E>, at: usize, end: Ended<T, E>) {
    let mut held = locked(slot);
    if held.unclaimed {
        drop(held);
        if let Ended::Failed(error) = end {
            group.unread(at, error);
        }
        return;
    }
    held.ended = Some(end);
    if let Some(joiner) = held.joiner.take() {
        joiner.wake();
    }
}

/// **A task that never pauses, run where it starts** into its slot: its
/// place, or `None` where a crash before it cancelled it.
fn run_now<T, E>(
    group: &dyn Receiver<E>,
    slot: &Shared<T, E>,
    body: impl FnOnce() -> Result<T, E>,
) -> Option<usize> {
    let Some(at) = group.place() else {
        locked(slot).ended = Some(Ended::Cancelled);
        return None;
    };
    let mut outcome = None;
    let end = match caught(|| outcome = Some(body())) {
        Err(crash) => {
            group.crashed(at, crash.clone());
            Ended::Crashed(crash)
        }
        Ok(()) => match outcome {
            Some(Ok(value)) => Ended::Value(value),
            Some(Err(error)) => Ended::Failed(error),
            None => unreachable!("a body that did not panic ended"),
        },
    };
    settle(group, slot, at, end);
    Some(at)
}

/// **A pausing task's slot, if the task is dropped before it ended** - the
/// group dropping every task after a crash (ADR-328 D8) - says so, so that a
/// `join` waiting on it is woken rather than left waiting.
struct Unended<T, E>(Shared<T, E>);

impl<T, E> Drop for Unended<T, E> {
    fn drop(&mut self) {
        let mut held = locked(&self.0);
        if held.ended.is_none() && !held.unclaimed {
            held.ended = Some(Ended::Cancelled);
            if let Some(joiner) = held.joiner.take() {
                joiner.wake();
            }
        }
    }
}

/// **A pausing task as the group polls it**: its body, stopped at a pause
/// once its handle asked for a cancel (D14), and its end settled into the
/// slot.
async fn paused_task<'g, T: 'g, E: 'g, F>(
    group: &'g dyn Receiver<E>,
    slot: Shared<T, E>,
    at: usize,
    body: impl FnOnce() -> F + 'g,
) where
    F: std::future::Future<Output = Result<T, E>> + 'g,
{
    let _unended = Unended(slot.clone());
    let mut running = Box::pin(body());
    let outcome = std::future::poll_fn(|cx| {
        if locked(&slot).cancel {
            return std::task::Poll::Ready(None);
        }
        running.as_mut().poll(cx).map(Some)
    })
    .await;
    // The teardown of a cancelled task is the drop of its body (ADR-292
    // D13), here, at its pause, before the group's end.
    drop(running);
    let end = match outcome {
        None => Ended::Cancelled,
        Some(Ok(value)) => Ended::Value(value),
        Some(Err(error)) => Ended::Failed(error),
    };
    settle(group, &slot, at, end);
}

/// **A pausing task of `async e` as its handle polls it** (D20, #1226): as
/// [`paused_task`], and a panic in its body is its crash, which the group sees
/// as well (ADR-328 D8), since no group's poll catches it.
async fn held_task<'g, T: 'g, E: 'g, F>(
    group: &'g dyn Receiver<E>,
    slot: Shared<T, E>,
    at: usize,
    body: impl FnOnce() -> F + 'g,
) where
    F: std::future::Future<Output = Result<T, E>> + 'g,
{
    let _unended = Unended(slot.clone());
    let mut body = Some(body);
    let mut running: Option<std::pin::Pin<Box<F>>> = None;
    let outcome = std::future::poll_fn(|cx| {
        if locked(&slot).cancel {
            return std::task::Poll::Ready(Ok(None));
        }
        let mut polled = std::task::Poll::Pending;
        let crash = caught(|| {
            let task = running.get_or_insert_with(|| {
                Box::pin(body.take().expect("a task's body is made once")())
            });
            polled = task.as_mut().poll(cx).map(Some);
        });
        match crash {
            Err(crash) => std::task::Poll::Ready(Err(crash)),
            Ok(()) => polled.map(Ok),
        }
    })
    .await;
    drop(running);
    let end = match outcome {
        Err(crash) => {
            group.crashed(at, crash.clone());
            Ended::Crashed(crash)
        }
        Ok(None) => Ended::Cancelled,
        Ok(Some(Ok(value))) => Ended::Value(value),
        Ok(Some(Err(error))) => Ended::Failed(error),
    };
    settle(group, &slot, at, end);
}

/// **`task::ChildHandle[T]`** (D11): the handle of a child task, bound to its
/// group. It has `join()` and `cancel()`, both of which take it, and
/// `start()` where it is deferred (D12). One dropped neither joined nor
/// cancelled is waited for at its group's end like every task of the group,
/// and its failure is the group's, as an unread one (D11, D21).
///
/// What #988 lowers onto it:
/// * `async(handle) fn { … }` is `scope.handle(body)` for a body that never
///   pauses and `scope.handle_paused(body)` for one that may;
/// * `async(deferred) fn { … }` is `scope.deferred(body)` /
///   `scope.deferred_paused(body)`, and `h.start()` is [`ChildHandle::start`];
/// * `h.join()` is `h.join().await`, `h.cancel()` is [`ChildHandle::cancel`].
///
/// A body hands back `Result<T, E>` in the group's channel `E` (`Ok(value)`
/// where it cannot fail).
pub struct ChildHandle<'g, T, E> {
    slot: Shared<T, E>,
    group: &'g dyn Receiver<E>,
    /// Its place in start order, once it started; `None` before a deferred
    /// handle's `start()`, or where a crash cancelled it.
    at: Option<usize>,
    /// What starts a deferred task (D12), until `start()` takes it.
    deferred: Option<Box<dyn FnOnce() -> Option<usize> + 'g>>,
    /// Made by `deferred`: `start()` is allowed once.
    is_deferred: bool,
    /// Joined or cancelled: its end is nobody else's.
    claimed: bool,
    /// **The rest of a pausing task of `async e`** (ADR-async-5 D6, D20,
    /// #1226), after its first pause: polled where the checker waits, its
    /// `join`. Dropped unjoined, the task is cancelled at that pause.
    held: Option<std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'g>>>,
}

impl<'g, T: 'g, E: 'g> ChildHandle<'g, T, E> {
    fn new(group: &'g dyn Receiver<E>, slot: Shared<T, E>) -> Self {
        ChildHandle {
            slot,
            group,
            at: None,
            deferred: None,
            is_deferred: false,
            claimed: false,
            held: None,
        }
    }

    /// **A handle to a pausing task of `async e`** (ADR-async-5 D6, D20,
    /// #1226): the task starts here, in written order, and runs to its first
    /// pause; the handle holds the rest, which its `join` polls. A crash
    /// before it cancelled it.
    pub(super) async fn held<F>(group: &'g dyn Receiver<E>, body: impl FnOnce() -> F + 'g) -> Self
    where
        F: std::future::Future<Output = Result<T, E>> + 'g,
    {
        let mut handle = ChildHandle::new(group, empty_slot());
        let Some(at) = group.place() else {
            locked(&handle.slot).ended = Some(Ended::Cancelled);
            return handle;
        };
        handle.at = Some(at);
        let mut task: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'g>> =
            Box::pin(held_task(group, handle.slot.clone(), at, body));
        let ended =
            std::future::poll_fn(|cx| std::task::Poll::Ready(task.as_mut().poll(cx).is_ready()))
                .await;
        if !ended {
            handle.held = Some(task);
        }
        handle
    }

    /// **A handle to a task that never pauses**, which runs now, to its end
    /// (D20 at `user_parallelism = no`), unless a crash before it cancelled it.
    pub(super) fn now(
        group: &'g dyn Receiver<E>,
        body: impl FnOnce() -> Result<T, E> + 'g,
    ) -> Self {
        let mut handle = ChildHandle::new(group, empty_slot());
        handle.at = run_now(group, &handle.slot, body);
        handle
    }

    /// **A deferred handle to a task that never pauses** (D12): it runs at
    /// `start()`.
    pub(super) fn deferred_now(
        group: &'g dyn Receiver<E>,
        body: impl FnOnce() -> Result<T, E> + 'g,
    ) -> Self {
        let mut handle = ChildHandle::new(group, empty_slot());
        let slot = handle.slot.clone();
        handle.deferred = Some(Box::new(move || run_now(group, &slot, body)));
        handle.is_deferred = true;
        handle
    }

    /// **A handle to a pausing task**: `hand_over` gives the group the task to
    /// poll beside its function, now (D11) or at `start()` (D12).
    pub(super) fn paused<F>(
        group: &'g dyn Receiver<E>,
        hand_over: impl FnOnce(std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'g>>) + 'g,
        body: impl FnOnce() -> F + 'g,
        deferred: bool,
    ) -> Self
    where
        F: std::future::Future<Output = Result<T, E>> + 'g,
    {
        let mut handle = ChildHandle::new(group, empty_slot());
        let slot = handle.slot.clone();
        let launch = move || {
            let Some(at) = group.place() else {
                locked(&slot).ended = Some(Ended::Cancelled);
                return None;
            };
            hand_over(Box::pin(paused_task(group, slot, at, body)));
            Some(at)
        };
        match deferred {
            true => {
                handle.deferred = Some(Box::new(launch));
                handle.is_deferred = true;
            }
            false => handle.at = launch(),
        }
        handle
    }

    /// **`h.start()`** (D12): start a deferred task; the handle stays. A second
    /// `start()`, or one on a handle that started at once, panics: the
    /// compile-time check is `NK2121`.
    pub fn start(&mut self) {
        match self.deferred.take() {
            Some(launch) => self.at = launch(),
            None if self.is_deferred => panic!("a deferred task was started twice"),
            None => panic!("start() on a task that started at once"),
        }
    }

    /// **`h.cancel()`** (D14): the task stops at its current pause, its values
    /// are torn down, and its end - a failure included - is dropped. A task
    /// that never pauses has ended already, so only its end is dropped; a
    /// deferred one not started never runs. The group's end still waits until
    /// a cancelled pausing task has stopped.
    pub fn cancel(mut self) {
        self.claimed = true;
        self.deferred = None;
        let mut held = locked(&self.slot);
        held.cancel = true;
        held.unclaimed = true;
        held.ended = None;
    }
}

impl<T, E: From<Crashed>> ChildHandle<'_, T, E> {
    /// **`h.join()`**: the task's value, once it has one; its failure, or
    /// [`Crashed`] in the group's channel. A suspension point for a pausing
    /// task; ready at once for one that never pauses. A task a crash before it
    /// cancelled throws [`Crashed`] here too, as the group does at its end.
    pub async fn join(mut self) -> Result<T, E> {
        self.claimed = true;
        assert!(
            self.deferred.is_none(),
            "a deferred task was joined before its start()"
        );
        let slot = self.slot.clone();
        let mut rest = self.held.take();
        let end = std::future::poll_fn(|cx| {
            // A pausing task of `async e` is polled here, where it is waited
            // for (#1226).
            if let Some(task) = rest.as_mut()
                && task.as_mut().poll(cx).is_ready()
            {
                rest = None;
            }
            let mut held = locked(&slot);
            match held.ended.take() {
                Some(end) => std::task::Poll::Ready(end),
                None => {
                    held.joiner = Some(cx.waker().clone());
                    std::task::Poll::Pending
                }
            }
        })
        .await;
        joined_end(end)
    }

    /// **`h.join()` of a task that never pauses** (D6, #988): it has ended
    /// where it started, so its end is there now.
    pub fn join_now(mut self) -> Result<T, E> {
        self.claimed = true;
        let end = locked(&self.slot).ended.take();
        joined_end(end.expect("a task that pauses was joined where the main line cannot wait"))
    }
}

/// What `join` hands back for a task's end.
fn joined_end<T, E: From<Crashed>>(end: Ended<T, E>) -> Result<T, E> {
    match end {
        Ended::Value(value) => Ok(value),
        Ended::Failed(error) => Err(error),
        Ended::Crashed(crash) => Err(E::from(crash)),
        Ended::Cancelled | Ended::OverBudget => Err(E::from(Crashed {
            message: "a task before it in its group crashed, which cancelled it".to_string(),
            site: String::new(),
        })),
    }
}

/// **A handle neither joined nor cancelled** hands its task's end to the
/// group (D11): a failure there now is kept as unread, one still to come is
/// handed over when the task ends.
impl<T, E> Drop for ChildHandle<'_, T, E> {
    fn drop(&mut self) {
        if self.claimed || self.deferred.is_some() {
            return;
        }
        let mut held = locked(&self.slot);
        held.unclaimed = true;
        let end = held.ended.take();
        drop(held);
        if let (Some(Ended::Failed(error)), Some(at)) = (end, self.at) {
            self.group.unread(at, error);
        }
    }
}

/// **A deferred free task**: `async(detached, deferred) fn { … }` (D12, D13),
/// the `TaskHandle[T]` that runs only from `h.start()`. A type of its own
/// because Rust's [`super::TaskHandle::start`] is the call that starts a task.
///
/// What #988 lowers onto it: `Deferred::new(async move { … })`, and
/// `h.start()` as [`Deferred::start`] at `user_parallelism = no`,
/// [`Deferred::start_on_pool`] at `yes`, as `spawn` picks
/// [`super::TaskHandle::start`] or [`super::TaskHandle::start_on_pool`].
pub struct Deferred<F: std::future::Future> {
    body: Option<F>,
    handle: Option<super::TaskHandle<F::Output>>,
}

impl<F: std::future::Future> Deferred<F> {
    /// A handle that runs `body` only from `start()`.
    pub fn new(body: F) -> Self {
        Deferred {
            body: Some(body),
            handle: None,
        }
    }

    fn body(&mut self) -> F {
        match self.body.take() {
            Some(body) => body,
            None => panic!("a deferred task was started twice"),
        }
    }

    /// Stop the task as [`super::TaskHandle::cancel`]; one not started never
    /// runs.
    pub fn cancel(self) {
        if let Some(handle) = self.handle {
            handle.cancel();
        }
    }

    /// Nobody waits for this task (ADR-346 D2); one not started never runs.
    pub fn detach(self) {}
}

impl<F: std::future::Future + 'static> Deferred<F>
where
    F::Output: 'static,
{
    /// **`h.start()`** at `user_parallelism = no`: start the task; the handle
    /// stays. A second `start()` panics (`NK2121` at compile time).
    pub fn start(&mut self) {
        let body = self.body();
        self.handle = Some(super::TaskHandle::start(body));
    }
}

impl<F: std::future::Future + Send + 'static> Deferred<F>
where
    F::Output: Send + 'static,
{
    /// **`h.start()`** at `user_parallelism = yes`, on the pool.
    pub fn start_on_pool(&mut self) {
        let body = self.body();
        self.handle = Some(super::TaskHandle::start_on_pool(body));
    }

    /// The task's value, as [`super::TaskHandle::join`].
    pub async fn join(self) -> Result<F::Output, Crashed> {
        match self.handle {
            Some(handle) => handle.join().await,
            None => panic!("a deferred task was joined before its start()"),
        }
    }
}
