//! **How many tasks of one form run at once** ([ADR-async-5](../../../../docs/specification/adr/adr-async-5.md)
//! D15, D18): the gate of an `async(limit: n, admission: s) { … }`.
//!
//! The main line starts the tasks of a group in written order. Where the gate
//! of the group has no free place, the next task is queued, unstarted, and
//! the main line goes on (D37); where it next pauses or waits, the running
//! tasks are polled and the queued ones start as places free, in written
//! order ([`Waiting`]). A statement task queues as any other does. A place is free where fewer than `n` hold one and the
//! strategy ([`Admission`]) lets one more start; where none runs, one starts
//! whatever the strategy says, so a group always ends. A task is under every
//! gate around it, in its function and in the tasks whose bodies started it,
//! and holds a place in each until it ends, is cancelled or crashes
//! ([`Frame`]); while it waits for its own children it gives its places back
//! and takes them again before it goes on (D38).

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// **What the runtime counts for a strategy** (D18): the tasks of the form
/// running and waiting, and the descriptors the process holds and may hold
/// (`0` allowed where the system does not say). The memory budget joins it
/// with #490.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Load {
    pub running: i64,
    pub waiting: i64,
    pub descriptors_in_use: i64,
    pub descriptors_allowed: i64,
}

/// **How many tasks may start until the next decision** (D18).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decision {
    pub start: i64,
}

/// **Who decides when the next task of a form starts** (D18,
/// `admission:`): asked on the runtime's beat, where a task wants to start
/// and the last decision's starts are used up.
pub trait Admission {
    fn decide(&mut self, load: Load) -> Decision;
}

/// **`std`'s strategy** (D18): tasks start while descriptors are free,
/// a quarter of the free ones at a time, and keep a reserve for the rest
/// of the program. It counts tasks and descriptors; the memory budget joins
/// it with #490.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Adaptive {
    reserve: i64,
}

/// `task::Adaptive()`: `std`'s strategy, the one a form without
/// `admission:` has.
#[allow(non_snake_case)]
pub fn Adaptive() -> Adaptive {
    Adaptive { reserve: 64 }
}

impl Default for Adaptive {
    fn default() -> Self {
        Adaptive()
    }
}

impl Admission for Adaptive {
    fn decide(&mut self, load: Load) -> Decision {
        if load.descriptors_allowed <= 0 {
            return Decision {
                start: load.waiting.max(1),
            };
        }
        let free = load.descriptors_allowed - load.descriptors_in_use - self.reserve;
        Decision {
            start: (free / 4).max(0),
        }
    }
}

/// **`limit: n` as the gate takes it**: a value below `1` panics where the
/// group starts, naming the option (D15), as a fold's does (ADR-372 D6).
#[track_caller]
pub fn limit(n: i64) -> usize {
    if n < 1 {
        crate::abort::panic(format!("`limit:` is {n}, and it has to be at least 1"));
    }
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// **`admission: s` as the gate takes it** (D18).
pub fn admitted(strategy: impl Admission + 'static) -> Box<dyn Admission> {
    Box::new(strategy)
}

/// **The rest of a task, as a gate polls it**: `true` where it ended.
pub trait Pollable {
    fn poll_here(&self, cx: &mut std::task::Context<'_>) -> bool;
    /// Whether it started and has not ended yet.
    fn running(&self) -> bool;
    /// Whether it waits, unstarted, for a place (D37).
    fn queued(&self) -> bool;
    /// Start it, a place being free: it runs to its first pause. `true`
    /// where it ended there.
    fn begin(&self, cx: &mut std::task::Context<'_>) -> bool;
}

impl Pollable for super::group::Rested<'_> {
    fn poll_here(&self, cx: &mut std::task::Context<'_>) -> bool {
        self.is_running() && self.poll_in_place(cx)
    }

    fn running(&self) -> bool {
        self.is_running()
    }

    fn queued(&self) -> bool {
        self.is_queued()
    }

    fn begin(&self, cx: &mut std::task::Context<'_>) -> bool {
        Rested::begin(self, cx)
    }
}

use super::group::Rested;

/// **The gate of one `async { … }`** (D15, D18, D37): the places, how many
/// are held, and the strategy. The tasks hold a place through it
/// ([`Gate::occupy`], [`Frame`]); it keeps nothing of theirs. Shared: the
/// tasks a task's body starts, in another function, are under it too (D15).
#[derive(Clone)]
pub struct Gate {
    inner: Rc<GateInner>,
}

struct GateInner {
    limit: usize,
    admission: RefCell<Box<dyn Admission>>,
    state: RefCell<GateState>,
}

struct GateState {
    /// Tasks holding a place.
    running: usize,
    /// Starts the last decision left.
    allowance: i64,
    /// What waits for a place to free: woken where one does.
    wanting: Vec<std::task::Waker>,
}

impl Gate {
    pub fn new(limit: usize, admission: Box<dyn Admission>) -> Self {
        Gate {
            inner: Rc::new(GateInner {
                limit,
                admission: RefCell::new(admission),
                state: RefCell::new(GateState {
                    running: 0,
                    allowance: 0,
                    wanting: Vec::new(),
                }),
            }),
        }
    }

    fn state(&self) -> std::cell::RefMut<'_, GateState> {
        self.inner.state.borrow_mut()
    }

    fn take_place(&self) {
        self.state().running += 1;
    }

    /// A place freed: what waits for one is woken.
    fn give_place(&self) {
        let wanting = {
            let mut state = self.state();
            state.running = state.running.saturating_sub(1);
            std::mem::take(&mut state.wanting)
        };
        for waker in wanting {
            waker.wake();
        }
    }

    /// Wake `cx` where a place frees.
    fn want(&self, cx: &std::task::Context<'_>) {
        let mut state = self.state();
        if !state.wanting.iter().any(|w| w.will_wake(cx.waker())) {
            state.wanting.push(cx.waker().clone());
        }
    }

    /// Whether the next task may start now, `waiting` of them wanting to:
    /// fewer than `limit` hold a place and the strategy lets one more start,
    /// or none holds one. A start it allows is counted.
    pub fn room(&self) -> bool {
        self.room_for(1)
    }

    fn room_for(&self, waiting: i64) -> bool {
        let mut state = self.state();
        if state.running >= self.inner.limit {
            return false;
        }
        // Where none runs, one starts whatever the strategy says, so it is
        // not asked: a group whose tasks run one after the other never counts
        // descriptors.
        if state.running == 0 {
            return true;
        }
        if state.allowance <= 0 {
            let load = Load {
                running: state.running as i64,
                waiting,
                descriptors_in_use: descriptors_in_use(),
                descriptors_allowed: descriptors_allowed(),
            };
            drop(state);
            let decision = self.inner.admission.borrow_mut().decide(load);
            state = self.state();
            state.allowance = decision.start;
        }
        if state.allowance > 0 {
            state.allowance -= 1;
            return true;
        }
        false
    }

    /// **The place a task holds** from its start to its end, its cancel or
    /// its crash; taken where the task starts.
    pub fn occupy(&self) -> Placed<'_> {
        self.take_place();
        Placed { gate: self }
    }
}

/// **A place held**: given back where the task ends, is cancelled or
/// crashes.
pub struct Placed<'a> {
    gate: &'a Gate,
}

impl Drop for Placed<'_> {
    fn drop(&mut self) {
        self.gate.give_place();
    }
}

/// **The places one pausing task holds** (D15, D38): one in each gate around
/// it, taken where it starts and given back where it ends, is cancelled or
/// crashes; given back while it waits for its own children, and taken again,
/// as a waiting task takes its place, before it goes on. While the task is
/// polled it is the current frame, so a group its body starts, in whatever
/// function, puts its tasks under these gates too.
pub struct Frame {
    gates: Vec<Gate>,
    holds: Cell<bool>,
}

thread_local! {
    static CURRENT: RefCell<Option<Rc<Frame>>> = const { RefCell::new(None) };
}

impl Frame {
    fn holding(gates: Vec<Gate>) -> Rc<Frame> {
        for gate in &gates {
            gate.take_place();
        }
        Rc::new(Frame {
            gates,
            holds: Cell::new(true),
        })
    }

    /// The frame of the task being polled, if any.
    fn current() -> Option<Rc<Frame>> {
        CURRENT.with(|current| current.borrow().clone())
    }

    /// **It waits for its children** (D38): its places are free meanwhile.
    fn give_back(&self) {
        if self.holds.replace(false) {
            for gate in &self.gates {
                gate.give_place();
            }
        }
    }

    /// **Its children ended**: it takes its places again where every gate
    /// has room, and is woken where one frees otherwise.
    fn take_again(&self, cx: &std::task::Context<'_>) -> bool {
        if self.holds.get() {
            return true;
        }
        if !self.gates.iter().all(|gate| gate.room_for(1)) {
            for gate in &self.gates {
                gate.want(cx);
            }
            return false;
        }
        for gate in &self.gates {
            gate.take_place();
        }
        self.holds.set(true);
        true
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        self.give_back();
    }
}

/// While a task is polled, its frame is the current one.
struct Entered(Option<Rc<Frame>>);

impl Entered {
    fn enter(frame: &Rc<Frame>) -> Entered {
        Entered(CURRENT.with(|current| current.replace(Some(frame.clone()))))
    }
}

impl Drop for Entered {
    fn drop(&mut self) {
        let before = self.0.take();
        CURRENT.with(|current| *current.borrow_mut() = before);
    }
}

/// **A pausing task under `gates`** (D15, D38): it holds a place in each
/// from its first poll to its end, and is the current frame while polled.
pub(crate) async fn placed<F: std::future::Future>(gates: Vec<Gate>, body: F) -> F::Output {
    let frame = Frame::holding(gates);
    let mut body = std::pin::pin!(body);
    std::future::poll_fn(|cx| {
        let _entered = Entered::enter(&frame);
        body.as_mut().poll(cx)
    })
    .await
}

/// **What starts the waiting tasks of a group** (D37): asked wherever its
/// main line waits.
pub trait Line {
    /// Poll the running tasks, and start the waiting ones a place frees for,
    /// first in first out.
    fn pump(&self, cx: &mut std::task::Context<'_>);
}

/// **The tasks of a gated group, as its main line sees them** (D15, D18,
/// D37): the cells of their rests, held weakly, in written order, those that
/// run and those that wait, unstarted, for a place. Declared inside the
/// group's function right after its gate, and used on its main line only.
///
/// A task over the limit, or one the strategy does not admit yet, is queued
/// in its own cell and the main line goes on. Where the main line waits -
/// a `join`, the end of a block, a statement task's start
/// ([`Waiting::during`]) or any other pause ([`Waiting::pumping`]) - the
/// running tasks are polled, and the waiting ones start as places free, in
/// written order. A group inside another gated one of the same function is
/// under both gates ([`Waiting::nested`]); one inside a task's body is under
/// the gates of that task too, and the task gives its places back while
/// its main line waits for its children (D38).
pub struct Waiting<'x> {
    gates: Vec<Gate>,
    outer: Option<&'x (dyn Line + 'x)>,
    /// The task whose body this group is in, if it holds places (D38).
    parent: Option<Rc<Frame>>,
    members: RefCell<Vec<Weak<dyn Pollable + 'x>>>,
}

impl<'x> Waiting<'x> {
    /// The tasks of the group whose gate is `gate`, under the gates of the
    /// task whose body starts the group, if any (D15).
    pub fn new(gate: &Gate) -> Self {
        let parent = Frame::current();
        let mut gates = parent
            .as_ref()
            .map_or_else(Vec::new, |frame| frame.gates.clone());
        gates.push(gate.clone());
        Waiting {
            gates,
            outer: None,
            parent,
            members: RefCell::new(Vec::new()),
        }
    }

    /// The tasks of a group inside the gated group of `outer`, in the same
    /// function: each needs a place in every gate around it (D15).
    pub fn nested<'y: 'x>(outer: &'x Waiting<'y>, gate: &Gate) -> Self {
        let mut gates = outer.gates.clone();
        gates.push(gate.clone());
        Waiting {
            gates,
            outer: Some(outer),
            parent: outer.parent.clone(),
            members: RefCell::new(Vec::new()),
        }
    }

    /// The gates a task of this group needs a place in.
    pub(crate) fn gates(&self) -> Vec<Gate> {
        self.gates.clone()
    }

    /// Whether a place is free in every gate, `waiting` tasks wanting one.
    fn room(&self, waiting: i64) -> bool {
        self.gates.iter().all(|gate| gate.room_for(waiting))
    }

    fn queued_members(&self) -> i64 {
        self.members
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .filter(|member| member.queued())
            .count() as i64
    }

    /// **Whether the next task waits for a place** (D37): one before it waits
    /// still, so written order keeps it waiting, or a gate has no room. Asked
    /// where it starts; a start it allows is counted.
    pub fn queues(&self) -> bool {
        let waiting = self.queued_members();
        waiting > 0 || !self.room(waiting + 1)
    }

    /// **The main line waits here for `wait`**, a `join` or the end of a
    /// block, and the group's tasks run meanwhile: the running ones are
    /// polled, the waiting ones start as places free (D37). Inside a task's
    /// body the task gives its places back while it waits, and takes them
    /// again before it goes on (D38).
    pub async fn during<F: std::future::Future>(&self, wait: F) -> F::Output {
        let mut wait = std::pin::pin!(wait);
        let mut ended = None;
        std::future::poll_fn(|cx| {
            // Its tasks run on while it waits to take its places again: one
            // of them may hold the place it waits for.
            self.pump(cx);
            if ended.is_none() {
                match wait.as_mut().poll(cx) {
                    std::task::Poll::Ready(value) => ended = Some(value),
                    std::task::Poll::Pending => {
                        let Some(parent) = self.parent.as_ref().filter(|frame| frame.holds.get())
                        else {
                            return std::task::Poll::Pending;
                        };
                        parent.give_back();
                        self.pump(cx);
                        match wait.as_mut().poll(cx) {
                            std::task::Poll::Ready(value) => ended = Some(value),
                            std::task::Poll::Pending => return std::task::Poll::Pending,
                        }
                    }
                }
            }
            if let Some(parent) = &self.parent
                && !parent.take_again(cx)
            {
                return std::task::Poll::Pending;
            }
            match ended.take() {
                Some(value) => std::task::Poll::Ready(value),
                None => std::task::Poll::Pending,
            }
        })
        .await
    }

    /// **The main line of the group runs here**: wherever it pauses, the
    /// group's running tasks are polled and the waiting ones start as places
    /// free (D37), as where it waits for one of them.
    pub async fn pumping<F: std::future::Future>(&self, body: F) -> F::Output {
        let mut body = std::pin::pin!(body);
        std::future::poll_fn(|cx| {
            let polled = body.as_mut().poll(cx);
            if polled.is_pending() {
                self.pump(cx);
            }
            polled
        })
        .await
    }

    /// **A task started under the gate**, at once or waiting for a place: its
    /// rest is kept, to poll it, or start it, where the main line waits.
    pub fn keep<'h: 'x, T: 'h, E: 'h>(
        &self,
        handle: super::ChildHandle<'h, T, E>,
    ) -> super::ChildHandle<'h, T, E> {
        let cell: Weak<dyn Pollable + 'x> = handle.pollable();
        self.members.borrow_mut().push(cell);
        handle
    }
}

impl Line for Waiting<'_> {
    fn pump(&self, cx: &mut std::task::Context<'_>) {
        if let Some(outer) = self.outer {
            outer.pump(cx);
        }
        loop {
            let members: Vec<_> = self
                .members
                .borrow()
                .iter()
                .filter_map(Weak::upgrade)
                .collect();
            let mut waiting = members.iter().filter(|member| member.queued()).count() as i64;
            let mut progressed = false;
            let mut blocked = false;
            for member in &members {
                if member.queued() {
                    // First in first out: none starts before one written
                    // earlier that still waits.
                    if blocked || !self.room(waiting) {
                        if !blocked {
                            for gate in &self.gates {
                                gate.want(cx);
                            }
                        }
                        blocked = true;
                        continue;
                    }
                    waiting -= 1;
                    member.begin(cx);
                    progressed = true;
                } else if member.running() && member.poll_here(cx) {
                    progressed = true;
                }
            }
            drop(members);
            self.members
                .borrow_mut()
                .retain(|member| member.strong_count() > 0);
            if !progressed {
                return;
            }
        }
    }
}

/// **A pause of a gated group's main line** (D37): `f().on_line(&line)`
/// awaits `f()` while the group's running tasks are polled and its waiting
/// ones start as places free.
pub trait OnLine: std::future::Future + Sized {
    #[allow(async_fn_in_trait)]
    async fn on_line(self, line: &Waiting<'_>) -> Self::Output {
        line.pumping(self).await
    }
}

impl<F: std::future::Future> OnLine for F {}

/// The descriptors the process holds; `0` where the system does not say.
fn descriptors_in_use() -> i64 {
    std::fs::read_dir("/proc/self/fd").map_or(0, |entries| entries.count() as i64)
}

/// The descriptors the process may hold, its soft limit; `0` where the
/// system does not say.
fn descriptors_allowed() -> i64 {
    let Ok(limits) = std::fs::read_to_string("/proc/self/limits") else {
        return 0;
    };
    limits
        .lines()
        .find(|line| line.starts_with("Max open files"))
        .and_then(|line| line.split_whitespace().nth(3))
        .and_then(|soft| soft.parse().ok())
        .unwrap_or(0)
}
