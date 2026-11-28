//! **The group runtime** ([ADR-async-v6](../../../../docs/specification/adr/adr-async-v6.md)
//! D1, D3, D9, D11, D12, D13, D14, D20, D21).
//!
//! Every task is started in a group and ends before the group does (D1). A
//! group counts its tasks in start order, which is the written order, hands
//! each task's end to its receiver (D3), and keeps every failure nobody read
//! until its own end, where the first in start order is thrown and the others
//! join it, in that order (D21). `task::scope` and its three siblings in
//! `task.rs` are groups of this kind; the forms of async-v6 (`async { }`,
//! `async(handle)`, `async(deferred)`) are lowered onto the same runtime by
//! #988, through the entry points below.
//!
//! **The receiver of a task's end** is its [`ChildHandle`] where it has one,
//! and the group otherwise: a task started by `spawn` on a scope, or one whose
//! handle was dropped neither joined nor cancelled (D11), hands its failure to
//! the group as an unread one.

use super::{Abandoned, Crashed, OnTimeout, Outcome, TimedOut, caught, locked};

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
    /// **The task waited for a pending value whose failure an earlier mention
    /// received** (ADR-async-v6 D8), so it was cancelled where it waited. Not
    /// a failure of its own (D22); a read of it throws [`Abandoned`] (D41).
    Abandoned(Box<Abandoned>),
    /// **Reserved**: a full memory block ends as a [`Crashed`] that says so
    /// (`Crashed::over_budget`, ADR-327 D13), which a channel naming
    /// `task::OverBudget` takes as that, and which cancels no other task.
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
        // A cancelled waiter (D8) is no failure: nothing to keep.
        if crash.cancelled {
            return;
        }
        // A full block fails only its own task (ADR-async-v6 D19).
        self.crashed |= !crash.over_budget();
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
    /// A task panicked: the tasks not yet started are cancelled (ADR-328
    /// D8). The crash itself is the task's end, read where it arrives like
    /// any failure (ADR-async-v6 D22, D35); only one nobody reads is the
    /// group's at its end ([`Receiver::unread_crash`]).
    fn crashed(&self, at: usize);
    /// A crash nobody read, kept until the group's end (D21).
    fn unread_crash(&self, at: usize, crash: Crashed);
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
    /// Its place in start order, for a task that waited for a place
    /// (ADR-async-v6 D37) and so took it after its handle was made.
    at: Option<usize>,
}

type Shared<T, E> = std::sync::Arc<std::sync::Mutex<Slot<T, E>>>;

fn empty_slot<T, E>() -> Shared<T, E> {
    std::sync::Arc::new(std::sync::Mutex::new(Slot {
        ended: None,
        joiner: None,
        unclaimed: false,
        cancel: false,
        at: None,
    }))
}

/// **A task's end, handed to its receiver**: the handle if it still wants it,
/// else the group, which keeps a failure as unread and drops the rest.
fn settle<T, E>(group: &dyn Receiver<E>, slot: &Shared<T, E>, at: usize, end: Ended<T, E>) {
    let mut held = locked(slot);
    if held.unclaimed {
        drop(held);
        match end {
            Ended::Failed(error) => group.unread(at, error),
            Ended::Crashed(crash) => group.unread_crash(at, crash),
            _ => {}
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
    // **The task's own block** (ADR-327 D5): its lists live there, and a
    // full one ends it alone (ADR-async-v6 D19).
    let block = crate::block::for_a_task();
    let entered = crate::block::enter(&block);
    let caught = caught(|| outcome = Some(body()));
    drop(entered);
    let end = match caught {
        Err(crash) if crash.cancelled => Ended::Abandoned(
            crash
                .abandoned
                .unwrap_or_else(|| Box::new(Abandoned::unknown())),
        ),
        Err(crash) => {
            // A full block fails only its own task (ADR-async-v6 D19).
            if !crash.over_budget() {
                group.crashed(at);
            }
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

/// **A pausing task's body in its own block** (ADR-327 D5): the block is in
/// place while the body is made, polled and dropped, also where the group
/// drops the task before it ended, and is handed back after the body.
struct InBlock<F> {
    running: Option<std::pin::Pin<Box<F>>>,
    block: crate::block::Block,
}

impl<F: std::future::Future> InBlock<F> {
    fn new() -> InBlock<F> {
        InBlock {
            running: None,
            block: crate::block::for_a_task(),
        }
    }

    /// Make the body, in the block.
    fn made(&mut self, body: impl FnOnce() -> F) {
        let _block = crate::block::enter(&self.block);
        self.running = Some(Box::pin(body()));
    }

    /// Poll the body, in the block.
    fn poll(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<F::Output> {
        let _block = crate::block::enter(&self.block);
        self.running
            .as_mut()
            .expect("a task's body is made before it is polled")
            .as_mut()
            .poll(cx)
    }
}

impl<F> Drop for InBlock<F> {
    fn drop(&mut self) {
        if let Some(running) = self.running.take() {
            let _block = crate::block::enter(&self.block);
            drop(running);
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
    let mut running = InBlock::new();
    running.made(body);
    let outcome = std::future::poll_fn(|cx| {
        if locked(&slot).cancel {
            return std::task::Poll::Ready(None);
        }
        running.poll(cx).map(Some)
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
    let mut running = InBlock::new();
    let outcome = std::future::poll_fn(|cx| {
        if locked(&slot).cancel {
            return std::task::Poll::Ready(Ok(None));
        }
        let mut polled = std::task::Poll::Pending;
        let crash = caught(|| {
            if let Some(body) = body.take() {
                running.made(body);
            }
            polled = running.poll(cx).map(Some);
        });
        match crash {
            Err(crash) => std::task::Poll::Ready(Err(crash)),
            Ok(()) => polled.map(Ok),
        }
    })
    .await;
    drop(running);
    let end = match outcome {
        Err(crash) if crash.cancelled => Ended::Abandoned(
            crash
                .abandoned
                .unwrap_or_else(|| Box::new(Abandoned::unknown())),
        ),
        Err(crash) => {
            // A full block fails only its own task (ADR-async-v6 D19).
            if !crash.over_budget() {
                group.crashed(at);
            }
            Ended::Crashed(crash)
        }
        Ok(None) => Ended::Cancelled,
        Ok(Some(Ok(value))) => Ended::Value(value),
        Ok(Some(Err(error))) => Ended::Failed(error),
    };
    settle(group, &slot, at, end);
}

/// **A pausing task that waited for a place** (ADR-async-v6 D37): it takes
/// its place in start order where its gate starts it, which is still the
/// written order, since its gate starts the waiting ones first in first out;
/// a crash before that cancelled it.
async fn queued_task<'g, T: 'g, E: 'g, F>(
    group: &'g dyn Receiver<E>,
    slot: Shared<T, E>,
    body: impl FnOnce() -> F + 'g,
) where
    F: std::future::Future<Output = Result<T, E>> + 'g,
{
    let Some(at) = group.place() else {
        let mut held = locked(&slot);
        held.ended = Some(Ended::Cancelled);
        if let Some(joiner) = held.joiner.take() {
            joiner.wake();
        }
        return;
    };
    locked(&slot).at = Some(at);
    held_task(group, slot, at, body).await;
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
    claimed: std::cell::Cell<bool>,
    /// **The rest of a pausing task of `async e`** (ADR-async-v6 D6, D20,
    /// #1226), after its first pause: polled where the checker waits, its
    /// `join`. Dropped unjoined, the task is cancelled at that pause.
    held: std::rc::Rc<Rested<'g>>,
    /// **The cell that holds the rest of a bound pausing task** (Part I 8.2,
    /// #987): where the handle is named after its `let`, the rest lives in a
    /// [`Rest`] the emitted code declares before it and finishes at the end of
    /// that block, so a handle dropped live is waited for there.
    kept: Option<&'g dyn Polled>,
    /// **The binding it starts**, `c` of `let c = async g(a)`: what an
    /// [`Abandoned`] read of it names (ADR-async-v6 D41). Empty where the
    /// lowering named none.
    name: &'static str,
}

/// What a [`ChildHandle`] does with the [`Rest`] cell of its task.
pub trait Polled {
    /// Poll the rest in place, where the checker waits; an ended one is
    /// dropped.
    fn poll_rest(&self, cx: &mut std::task::Context<'_>);
    /// Drop the rest: the task stops at its pause.
    fn cancel_rest(&self);
    /// The cell of the rest, for a gate to poll (D15, D18).
    fn pollable(&self) -> std::rc::Weak<dyn super::Pollable + '_>;
}

/// **The rest of a bound pausing task of `async e`** (Part I 8.2,
/// ADR-async-v6 D11, #987): declared by the emitted code just before the
/// handle's `let`, so it may borrow what the task borrows. `join` polls it
/// through the handle; [`Rest::finish`], at the end of the block, waits for a
/// rest whose handle was dropped live (its failure then reaches the group as
/// an unread one); leaving the block early drops it, at its pause.
pub struct Rest<'b> {
    held: std::rc::Rc<Rested<'b>>,
}

/// **Where the rest of a pausing task lives** after its first pause: shared
/// with the gates of `limit:` and `admission:` around it (ADR-async-v6 D15,
/// D18), which poll it while a sibling waits for a place.
pub(super) struct Rested<'b> {
    task: std::cell::RefCell<Option<Paused<'b>>>,
    /// **It waits for a place** (D37): unstarted, and polled by nobody until
    /// its gate starts it.
    queued: std::cell::Cell<bool>,
}

type Paused<'b> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'b>>;

impl<'b> Rested<'b> {
    fn empty() -> std::rc::Rc<Self> {
        std::rc::Rc::new(Rested {
            task: std::cell::RefCell::new(None),
            queued: std::cell::Cell::new(false),
        })
    }

    /// A task that waits, unstarted, for its gate to start it (D37).
    fn queue(&self, task: Paused<'b>) {
        *self.task.borrow_mut() = Some(task);
        self.queued.set(true);
    }

    /// The task, taken out: it is dropped, at its pause, or never starts.
    fn take(&self) -> Option<Paused<'b>> {
        self.queued.set(false);
        self.task.take()
    }

    /// **Poll it in place** where it runs; `true` where nothing of it is left
    /// to wait for. One that waits for a place is not polled: its gate starts
    /// it (D37). One polled already, further up the same poll, is skipped.
    pub(super) fn poll_in_place(&self, cx: &mut std::task::Context<'_>) -> bool {
        if self.queued.get() {
            return false;
        }
        let Ok(mut held) = self.task.try_borrow_mut() else {
            return false;
        };
        let Some(task) = held.as_mut() else {
            return true;
        };
        if task.as_mut().poll(cx).is_ready() {
            *held = None;
            return true;
        }
        false
    }

    /// Whether it waits for a place.
    pub(super) fn is_queued(&self) -> bool {
        self.queued.get() && self.task.try_borrow().is_ok_and(|held| held.is_some())
    }

    /// Whether it started and has not ended yet.
    pub(super) fn is_running(&self) -> bool {
        !self.queued.get() && self.task.try_borrow().map_or(true, |held| held.is_some())
    }

    /// **Its gate starts it** (D37): from here it runs as any started task,
    /// to its first pause now.
    pub(super) fn begin(&self, cx: &mut std::task::Context<'_>) -> bool {
        self.queued.set(false);
        self.poll_in_place(cx)
    }
}

impl Rest<'_> {
    pub fn new() -> Self {
        Rest {
            held: Rested::empty(),
        }
    }

    /// **The block's end**: the rest still running, if any, is waited for.
    /// One still waiting for a place is started by its gate, which polls
    /// this wait ([`super::Waiting::during`]).
    pub async fn finish(&self) {
        std::future::poll_fn(|cx| {
            if self.held.queued.get() || !self.held.poll_in_place(cx) {
                return std::task::Poll::Pending;
            }
            std::task::Poll::Ready(())
        })
        .await
    }
}

impl Default for Rest<'_> {
    fn default() -> Self {
        Rest::new()
    }
}

impl Polled for Rest<'_> {
    fn poll_rest(&self, cx: &mut std::task::Context<'_>) {
        self.held.poll_in_place(cx);
    }

    fn cancel_rest(&self) {
        drop(self.held.take());
    }

    fn pollable(&self) -> std::rc::Weak<dyn super::Pollable + '_> {
        let cell: std::rc::Weak<Rested<'_>> = std::rc::Rc::downgrade(&self.held);
        cell
    }
}

impl<'g, T: 'g, E: 'g> ChildHandle<'g, T, E> {
    fn new(group: &'g dyn Receiver<E>, slot: Shared<T, E>) -> Self {
        ChildHandle {
            slot,
            group,
            at: None,
            deferred: None,
            is_deferred: false,
            claimed: std::cell::Cell::new(false),
            held: Rested::empty(),
            kept: None,
            name: "",
        }
    }

    /// **The handle of `let c = async …`, named `c`** (ADR-async-v6 D41): a
    /// read of it that throws [`Abandoned`] names it.
    pub fn named(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    /// **A handle to a bound pausing task of `async e`** (Part I 8.2, #987):
    /// as [`ChildHandle::held`], but the rest lives in `rest`, which the
    /// block that declared it finishes at its end.
    pub(super) async fn kept<'b, F>(
        group: &'b dyn Receiver<E>,
        rest: &'g Rest<'b>,
        body: impl FnOnce() -> F + 'b,
    ) -> Self
    where
        'b: 'g,
        T: 'b,
        E: 'b,
        F: std::future::Future<Output = Result<T, E>> + 'b,
    {
        let mut handle = ChildHandle::new(group, empty_slot());
        handle.kept = Some(rest);
        let Some(at) = group.place() else {
            locked(&handle.slot).ended = Some(Ended::Cancelled);
            return handle;
        };
        handle.at = Some(at);
        *rest.held.task.borrow_mut() =
            Some(Box::pin(held_task(group, handle.slot.clone(), at, body)));
        std::future::poll_fn(|cx| {
            rest.poll_rest(cx);
            std::task::Poll::Ready(())
        })
        .await;
        handle
    }

    /// **A handle to a pausing task of `async e`** (ADR-async-v6 D6, D20,
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
            *handle.held.task.borrow_mut() = Some(task);
        }
        handle
    }

    /// **A handle to a pausing task of `async e` that waits for a place**
    /// (ADR-async-v6 D37): unstarted, it is queued in its own cell, and the
    /// main line goes on; its gate starts it when a place frees.
    pub(super) fn held_queued<F>(group: &'g dyn Receiver<E>, body: impl FnOnce() -> F + 'g) -> Self
    where
        F: std::future::Future<Output = Result<T, E>> + 'g,
    {
        let handle = ChildHandle::new(group, empty_slot());
        handle
            .held
            .queue(Box::pin(queued_task(group, handle.slot.clone(), body)));
        handle
    }

    /// [`ChildHandle::held_queued`] for a bound task whose rest lives in
    /// `rest` (Part I 8.2, #987).
    pub(super) fn kept_queued<'b, F>(
        group: &'b dyn Receiver<E>,
        rest: &'g Rest<'b>,
        body: impl FnOnce() -> F + 'b,
    ) -> Self
    where
        'b: 'g,
        T: 'b,
        E: 'b,
        F: std::future::Future<Output = Result<T, E>> + 'b,
    {
        let mut handle = ChildHandle::new(group, empty_slot());
        handle.kept = Some(rest);
        rest.held
            .queue(Box::pin(queued_task(group, handle.slot.clone(), body)));
        handle
    }

    /// **A deferred handle to a bound pausing task** (D12, D37): `start()`
    /// queues the task in `rest`, unstarted, and the main line goes on; the
    /// gate around it, or the wait for it, starts it where the main line next
    /// pauses or waits. Before `start()` the cell holds nothing, so neither
    /// a gate nor the block's end waits for it.
    pub(super) fn kept_deferred<'b, F>(
        group: &'b dyn Receiver<E>,
        rest: &'g Rest<'b>,
        body: impl FnOnce() -> F + 'b,
    ) -> Self
    where
        'b: 'g,
        T: 'b,
        E: 'b,
        F: std::future::Future<Output = Result<T, E>> + 'b,
    {
        let mut handle = ChildHandle::new(group, empty_slot());
        handle.kept = Some(rest);
        let slot = handle.slot.clone();
        handle.deferred = Some(Box::new(move || {
            rest.held.queue(Box::pin(queued_task(group, slot, body)));
            None
        }));
        handle.is_deferred = true;
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

    /// **The rest of its pausing task, for the gates around it** (D15, D18):
    /// polled by a sibling that waits for a place.
    pub fn pollable(&self) -> std::rc::Weak<dyn super::Pollable + 'g> {
        if let Some(rest) = self.kept {
            return rest.pollable();
        }
        let cell: std::rc::Weak<Rested<'g>> = std::rc::Rc::downgrade(&self.held);
        cell
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
        self.deferred = None;
        self.cancel_here();
    }
}

impl<T, E> ChildHandle<'_, T, E> {
    /// What [`ChildHandle::cancel`] does, on a handle that stays a value: a
    /// `join(within:)` with [`OnTimeout::Cancel`] uses it up this way. The
    /// rest of a pausing task of `async e` is dropped here, at its pause.
    fn cancel_here(&self) {
        self.claimed.set(true);
        drop(self.held.take());
        if let Some(rest) = self.kept {
            rest.cancel_rest();
        }
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
    pub async fn join(self) -> Result<T, E> {
        self.claimed.set(true);
        assert!(
            self.deferred.is_none(),
            "a deferred task was joined before its start()"
        );
        let end = std::future::poll_fn(|cx| self.poll_end(cx)).await;
        joined_end(end, self.name)
    }

    /// One poll of the task's end, where it is waited for: a pausing task of
    /// `async e` is polled here (#1226); one waiting for a place, once its
    /// gate started it (D37).
    fn poll_end(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Ended<T, E>> {
        self.held.poll_in_place(cx);
        if let Some(kept) = self.kept {
            kept.poll_rest(cx);
        }
        let mut held = locked(&self.slot);
        match held.ended.take() {
            Some(end) => std::task::Poll::Ready(end),
            None => {
                held.joiner = Some(cx.waker().clone());
                std::task::Poll::Pending
            }
        }
    }

    /// **`h.join(within: d, on_timeout: o)`** (ADR-async-v6 D17): the task's
    /// value, its failure or [`Crashed`] as `join` gives them, or
    /// [`TimedOut`] once `span` ran out. **It lends the handle**: with
    /// [`OnTimeout::Keep`] the task runs on and the handle stays live, joined
    /// again, cancelled, or waited for at its group's end; with
    /// [`OnTimeout::Cancel`] the task is cancelled as [`ChildHandle::cancel`]
    /// cancels it and the handle is used up. The error is boxed, whatever the
    /// group's channel, since a plain group's (`Crashed`) cannot hold a
    /// [`TimedOut`].
    pub async fn join_within(
        &self,
        span: std::time::Duration,
        on_timeout: OnTimeout,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        E: Into<Box<dyn std::error::Error>>,
    {
        use std::future::Future;
        assert!(
            self.deferred.is_none(),
            "a deferred task was joined before its start()"
        );
        let slot = self.slot.clone();
        let mut sleeping = std::pin::pin!(crate::time::sleep(span));
        let end = std::future::poll_fn(|cx| {
            // A pausing task of `async e` is polled here, as in `join`.
            self.held.poll_in_place(cx);
            if let Some(kept) = self.kept {
                kept.poll_rest(cx);
            }
            let mut held = locked(&slot);
            if let Some(end) = held.ended.take() {
                return std::task::Poll::Ready(Some(end));
            }
            held.joiner = Some(cx.waker().clone());
            drop(held);
            match sleeping.as_mut().poll(cx) {
                std::task::Poll::Ready(()) => std::task::Poll::Ready(None),
                std::task::Poll::Pending => std::task::Poll::Pending,
            }
        })
        .await;
        match end {
            Some(end) => {
                self.claimed.set(true);
                joined_end(end, self.name).map_err(Into::into)
            }
            None => {
                // The task runs on: the handle keeps what is left of it.
                locked(&self.slot).joiner = None;
                if on_timeout == OnTimeout::Cancel {
                    self.cancel_here();
                }
                Err(Box::new(TimedOut))
            }
        }
    }

    /// **`h.join()` of a task that never pauses** (D6, #988): it has ended
    /// where it started, so its end is there now.
    pub fn join_now(self) -> Result<T, E> {
        self.claimed.set(true);
        let end = locked(&self.slot).ended.take();
        joined_end(
            end.expect("a task that pauses was joined where the main line cannot wait"),
            self.name,
        )
    }
}

/// **A pending binding several tasks read** (ADR-async-v6 D8, #986): `let b =
/// async f(a)` and `let c = async g(a)` both wait for `a` and read the one
/// value. The first to wait joins `a`'s task and keeps its value here; every
/// other reads it. Where `a` failed, the first receives the error and every
/// other waiter is cancelled where it waits: it unwinds with [`WaitCancelled`],
/// its values torn down, and ends as [`Ended::Cancelled`], which is no failure
/// (D22): the group keeps nothing of it and no fallback runs. One error, one
/// receiver (ADR-346 D1).
///
/// What #986 lowers onto it: `let __task_N = Read::new(group.start_here(…))`,
/// and a mention of `a` in another task as `(*__task_N.join_now()?)`, or
/// `.join().await` for a task that pauses.
pub struct Read<'g, T, E> {
    handle: std::cell::RefCell<Option<ChildHandle<'g, T, E>>>,
    value: std::cell::OnceCell<T>,
    /// The task failed and its first waiter received it.
    failed: std::cell::Cell<bool>,
    /// The tasks waiting for a pausing one besides the one polling it.
    waiting: std::cell::RefCell<Vec<std::task::Waker>>,
    /// **Why every later waiter is cancelled**, once the first received the
    /// failure (ADR-async-v6 D41): the binding's name and what its failure
    /// said, which a read of such a waiter throws as [`Abandoned`].
    why: std::cell::RefCell<Option<Abandoned>>,
    /// The binding, `a`; empty where the lowering named none.
    name: &'static str,
}

impl<'g, T: 'g, E: From<Crashed> + std::fmt::Display + 'g> Read<'g, T, E> {
    pub fn new(handle: ChildHandle<'g, T, E>) -> Self {
        let name = handle.name;
        Read {
            handle: std::cell::RefCell::new(Some(handle)),
            value: std::cell::OnceCell::new(),
            failed: std::cell::Cell::new(false),
            waiting: std::cell::RefCell::new(Vec::new()),
            why: std::cell::RefCell::new(None),
            name,
        }
    }

    /// The value of a task that never pauses: it has ended where it started.
    pub fn join_now(&self) -> Result<&T, E> {
        if let Some(value) = self.value.get() {
            return Ok(value);
        }
        let handle = self.handle.borrow_mut().take();
        match handle {
            Some(handle) => self.kept(handle.join_now()),
            None => self.cancelled_waiter(),
        }
    }

    /// The value of a task that pauses: whoever waits polls it.
    pub async fn join(&self) -> Result<&T, E> {
        std::future::poll_fn(|cx| {
            if self.value.get().is_some() || self.failed.get() {
                return std::task::Poll::Ready(self.read());
            }
            let end = match self.handle.borrow().as_ref() {
                Some(handle) => {
                    handle.claimed.set(true);
                    handle.poll_end(cx)
                }
                None => return std::task::Poll::Ready(self.read()),
            };
            match end {
                std::task::Poll::Ready(end) => {
                    drop(self.handle.borrow_mut().take());
                    for waker in self.waiting.borrow_mut().drain(..) {
                        waker.wake();
                    }
                    std::task::Poll::Ready(self.kept(joined_end(end, self.name)))
                }
                std::task::Poll::Pending => {
                    self.waiting.borrow_mut().push(cx.waker().clone());
                    std::task::Poll::Pending
                }
            }
        })
        .await
    }

    /// The task's end, kept: its value for every waiter, its failure for the
    /// first only.
    fn kept(&self, end: Result<T, E>) -> Result<&T, E> {
        match end {
            Ok(value) => Ok(self.value.get_or_init(|| value)),
            Err(error) => {
                self.failed.set(true);
                *self.why.borrow_mut() = Some(Abandoned::of(self.name, error.to_string()));
                Err(error)
            }
        }
    }

    fn read(&self) -> Result<&T, E> {
        match self.value.get() {
            Some(value) => Ok(value),
            None => self.cancelled_waiter(),
        }
    }

    /// A waiter of a pending value whose failure an earlier mention received:
    /// it cannot go on, and is cancelled where it waits (ADR-async-v6 D8),
    /// carrying why (D41).
    fn cancelled_waiter<V>(&self) -> V {
        let why = self.why.borrow().clone().unwrap_or_else(Abandoned::unknown);
        std::panic::resume_unwind(Box::new(WaitCancelled(Box::new(why))))
    }
}

/// **The payload a cancelled waiter unwinds with** (D8): its task's runner
/// ends it as [`Ended::Abandoned`], with why (D41); `resume_unwind` runs no
/// panic hook.
pub(crate) struct WaitCancelled(pub(crate) Box<Abandoned>);

/// What `join` hands back for a task's end; `name` is the joined binding's,
/// which an [`Abandoned`] read names.
fn joined_end<T, E: From<Crashed>>(end: Ended<T, E>, name: &str) -> Result<T, E> {
    match end {
        Ended::Value(value) => Ok(value),
        Ended::Failed(error) => Err(error),
        Ended::Crashed(crash) => Err(E::from(crash)),
        Ended::Abandoned(why) => Err(E::from(Crashed::abandoned(Box::new(why.of_waiter(name))))),
        Ended::Cancelled | Ended::OverBudget => Err(E::from(Crashed::said(
            "a task before it in its group crashed, which cancelled it",
        ))),
    }
}

/// **A handle neither joined nor cancelled** hands its task's end to the
/// group (D11): a failure there now is kept as unread, one still to come is
/// handed over when the task ends.
impl<T, E> Drop for ChildHandle<'_, T, E> {
    fn drop(&mut self) {
        if self.claimed.get() || self.deferred.is_some() {
            return;
        }
        let mut held = locked(&self.slot);
        held.unclaimed = true;
        let end = held.ended.take();
        let at = self.at.or(held.at);
        drop(held);
        match (end, at) {
            (Some(Ended::Failed(error)), Some(at)) => self.group.unread(at, error),
            (Some(Ended::Crashed(crash)), Some(at)) => self.group.unread_crash(at, crash),
            _ => {}
        }
    }
}

/// **A deferred free task**: `async(detached, deferred) fn { … }` (D12, D13),
/// the `TaskHandle[T]` that runs only from `h.start()`. A type of its own
/// because Rust's [`super::TaskHandle::start`] is the call that starts a task.
///
/// What #2512 lowers onto it: `Deferred::new(move || start)`, where `start`
/// is the very call `spawn` writes ([`super::TaskHandle::start`] or
/// `start_on_pool`, [`super::FailingTask`], [`super::BoxedTask`]), so the
/// setting and the body's channel pick the handle as they do for `spawn`;
/// `h.start()` is [`Deferred::start`], which makes that call. `join`,
/// `join_within`, `cancel` and `detach` are the started handle's.
pub struct Deferred<S, H> {
    start: Option<S>,
    handle: Option<H>,
}

impl<S: FnOnce() -> H, H> Deferred<S, H> {
    /// A handle whose task `start` starts, only from `start()`.
    pub fn new(start: S) -> Self {
        Deferred {
            start: Some(start),
            handle: None,
        }
    }

    /// **`h.start()`**: start the task; the handle stays. A second `start()`
    /// panics (`NK2121` at compile time).
    pub fn start(&mut self) {
        match self.start.take() {
            Some(start) => self.handle = Some(start()),
            None => panic!("a deferred task was started twice"),
        }
    }
}

impl<S, H> Deferred<S, H> {
    fn started(self) -> H {
        match self.handle {
            Some(handle) => handle,
            None => panic!("a deferred task was joined before its start()"),
        }
    }

    fn started_ref(&self) -> &H {
        match &self.handle {
            Some(handle) => handle,
            None => panic!("a deferred task was joined before its start()"),
        }
    }
}

impl<S, T> Deferred<S, super::TaskHandle<T>> {
    /// Stop the task as [`super::TaskHandle::cancel`]; one not started never
    /// runs.
    pub fn cancel(self) {
        if let Some(handle) = self.handle {
            handle.cancel();
        }
    }

    /// Nobody waits for this task (ADR-346 D2); one not started never runs.
    pub fn detach(self) {}

    /// As [`super::TaskHandle::join_within`].
    pub async fn join_within(
        &self,
        span: std::time::Duration,
        on_timeout: super::OnTimeout,
    ) -> Result<T, super::Late> {
        self.started_ref().join_within(span, on_timeout).await
    }
}

impl<S, T: Send + 'static> Deferred<S, super::TaskHandle<T>> {
    /// The task's value, as [`super::TaskHandle::join`].
    pub async fn join(self) -> Result<T, Crashed> {
        self.started().join().await
    }
}

impl<S, T, E> Deferred<S, super::FailingTask<T, E>> {
    /// As [`super::FailingTask::cancel`].
    pub fn cancel(self) {
        if let Some(handle) = self.handle {
            handle.cancel();
        }
    }

    /// As [`super::FailingTask::detach`]; one not started never runs.
    pub fn detach(self) {
        if let Some(handle) = self.handle {
            handle.detach();
        }
    }
}

impl<S, T: 'static, E: 'static + From<Crashed>> Deferred<S, super::FailingTask<T, E>> {
    /// As [`super::FailingTask::join`].
    pub async fn join(self) -> Result<T, E> {
        self.started().join().await
    }
}

impl<S, T: 'static, E: 'static + From<Crashed> + From<super::TimedOut>>
    Deferred<S, super::FailingTask<T, E>>
{
    /// As [`super::FailingTask::join_within`].
    pub async fn join_within(
        &self,
        span: std::time::Duration,
        on_timeout: super::OnTimeout,
    ) -> Result<T, E> {
        self.started_ref().join_within(span, on_timeout).await
    }
}

impl<S, T> Deferred<S, super::BoxedTask<T>> {
    /// As [`super::BoxedTask::cancel`].
    pub fn cancel(self) {
        if let Some(handle) = self.handle {
            handle.cancel();
        }
    }

    /// As [`super::BoxedTask::detach`]; one not started never runs.
    pub fn detach(self) {
        if let Some(handle) = self.handle {
            handle.detach();
        }
    }
}

impl<S, T: Send + 'static> Deferred<S, super::BoxedTask<T>> {
    /// As [`super::BoxedTask::join`].
    pub async fn join(self) -> Result<T, Box<dyn std::error::Error>> {
        self.started().join().await
    }

    /// As [`super::BoxedTask::join_within`].
    pub async fn join_within(
        &self,
        span: std::time::Duration,
        on_timeout: super::OnTimeout,
    ) -> Result<T, Box<dyn std::error::Error>> {
        self.started_ref().join_within(span, on_timeout).await
    }
}
