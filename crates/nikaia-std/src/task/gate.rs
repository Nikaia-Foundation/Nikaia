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

/// **What the runtime counts for a strategy** (D18, ADR-327 D15): the tasks
/// of the form running and waiting, the descriptors the process holds and may
/// hold (`0` allowed where the system does not say), and the memory: what the
/// system reports (the cgroup's limit where one is set, else the memory
/// available; `0` where it does not say) and what the tasks' blocks hold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Load {
    pub running: i64,
    pub waiting: i64,
    pub descriptors_in_use: i64,
    pub descriptors_allowed: i64,
    pub memory_reported: i64,
    pub memory_held: i64,
}

/// **How many tasks may start until the next decision, and the budget each
/// of them gets** (D18, ADR-327 D7, D15): the bytes a task starting now may
/// hold, which it keeps until it ends; less than a segment is a segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decision {
    pub start: i64,
    pub budget: i64,
}

/// **Who decides when the next task of a form starts** (D18,
/// `admission:`): asked on the runtime's beat, where a task wants to start
/// and the last decision's starts are used up.
pub trait Admission {
    fn decide(&mut self, load: Load) -> Decision;
}

/// **`std`'s strategy** (D18, ADR-327 D15): tasks start while descriptors
/// are free, a quarter of the free ones at a time, and keep a reserve for the
/// rest of the program. The memory not held yet is shared among the tasks
/// running and wanting to start, and one share more for the rest of the
/// program: that share is a starting task's budget, and none starts where it
/// is less than a segment. Where the system reports no memory, the budget is
/// not bounded.
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
        Adaptive::decide(self, load)
    }
}

impl Adaptive {
    /// **`std`'s decision for `load`** (ADR-async-5 D18, ADR-327 D15), as a
    /// task that wants to start asks it; inherent as well, so a program that
    /// calls `.decide` on the strategy reaches it without the trait in scope.
    pub fn decide(&mut self, load: Load) -> Decision {
        let start = if load.descriptors_allowed <= 0 {
            load.waiting.max(1)
        } else {
            let free = load.descriptors_allowed - load.descriptors_in_use - self.reserve;
            (free / 4).max(0)
        };
        if load.memory_reported <= 0 {
            return Decision {
                start,
                budget: i64::MAX,
            };
        }
        let free = (load.memory_reported - load.memory_held).max(0);
        let shares = load.running.max(0) + load.waiting.max(1) + 1;
        let budget = free / shares;
        let segment = crate::block::BLOCK as i64;
        Decision {
            start: if budget < segment { 0 } else { start },
            budget: budget.max(segment),
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

/// **`admission: s` on a form that starts one task whose task never
/// pauses** (D15, D18): the strategy is asked where the task starts, and the
/// task runs there to its end whatever it says, as every task that never
/// pauses does (D20 at `user_parallelism = no`): none of its form runs then.
pub fn asked(mut strategy: Box<dyn Admission>) {
    crate::block::granted(strategy.decide(load_of(0, 1)).budget);
}

/// **`admission: s` on `async(detached) fn { … }`** (D15, D18), which may
/// run on another thread.
pub fn admitted_sent(strategy: impl Admission + Send + 'static) -> Box<dyn Admission + Send> {
    Box::new(strategy)
}

/// **A detached task waits, unstarted, until its strategy admits it** (D15,
/// D18, D37): awaited first in its body, the strategy asked on each beat of
/// the runtime until it lets the task start. Its block is in place already,
/// empty, and takes the decision's budget before the body runs (ADR-327 D15).
pub async fn admission_first(mut strategy: Box<dyn Admission + Send>) {
    std::future::poll_fn(|cx| {
        let decision = strategy.decide(load_of(0, 1));
        if decision.start > 0 {
            crate::block::budget_running(decision.budget);
            return std::task::Poll::Ready(());
        }
        cx.waker().wake_by_ref();
        std::task::Poll::Pending
    })
    .await
}

/// The load a strategy is shown: `running` and `waiting` tasks of its form,
/// the descriptors and the memory.
pub(crate) fn load_of(running: i64, waiting: i64) -> Load {
    Load {
        running,
        waiting,
        descriptors_in_use: descriptors_in_use(),
        descriptors_allowed: descriptors_allowed(),
        memory_reported: memory_reported(),
        memory_held: i64::try_from(crate::block::held()).unwrap_or(i64::MAX),
    }
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
    /// The budget the last decision gives each of them (ADR-327 D15).
    budget: i64,
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
                    budget: 0,
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
        // descriptors. It has the budget of the gate's last decision, or the
        // program's before the first (ADR-327 D15).
        if state.running == 0 {
            if state.budget > 0 {
                crate::block::granted(state.budget);
            }
            return true;
        }
        if state.allowance <= 0 {
            let running = state.running as i64;
            drop(state);
            let decision = self
                .inner
                .admission
                .borrow_mut()
                .decide(load_of(running, waiting));
            state = self.state();
            state.allowance = decision.start;
            state.budget = decision.budget.max(1);
        }
        if state.allowance > 0 {
            state.allowance -= 1;
            crate::block::granted(state.budget);
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
    members: RefCell<Vec<Member<'x>>>,
    /// **The strategy of the task started last** (`admission:` on a form
    /// that starts one task, D15, D18), where it did not admit it yet: the
    /// task's [`Waiting::keep`] takes it.
    admitting: RefCell<Option<Box<dyn Admission>>>,
}

/// **A task of the group**: the cell of its rest, and the strategy of its
/// own `admission:` that has not admitted it yet, if any (D15, D18).
struct Member<'x> {
    cell: Weak<dyn Pollable + 'x>,
    admission: Rc<RefCell<Option<Box<dyn Admission>>>>,
}

/// The strategy of one task, as the gate asks it.
type Own = Rc<RefCell<Option<Box<dyn Admission>>>>;

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
            admitting: RefCell::new(None),
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
            admitting: RefCell::new(None),
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
            .filter_map(|member| member.cell.upgrade())
            .filter(|member| member.queued())
            .count() as i64
    }

    fn running_members(&self) -> i64 {
        self.members
            .borrow()
            .iter()
            .filter_map(|member| member.cell.upgrade())
            .filter(|member| member.running())
            .count() as i64
    }

    /// **Whether the next task, with a strategy of its own, waits for a
    /// place** (`admission:` on a form that starts one task, D15, D18, D37):
    /// as [`Waiting::queues`], and its own strategy is asked first; one that
    /// does not admit it yet is kept for the task's [`Waiting::keep`], and
    /// asked again where the main line next pauses or waits.
    pub fn queues_admitted(&self, mut admission: Box<dyn Admission>) -> bool {
        let waiting = self.queued_members();
        if waiting > 0 {
            *self.admitting.borrow_mut() = Some(admission);
            return true;
        }
        let decision = admission.decide(load_of(self.running_members(), waiting + 1));
        if decision.start <= 0 {
            *self.admitting.borrow_mut() = Some(admission);
            return true;
        }
        let queued = !self.room(waiting + 1);
        // The task's own strategy sizes its block, not the gate's.
        crate::block::granted(decision.budget);
        queued
    }

    /// Whether the task's own strategy admits it, `waiting` wanting to start;
    /// `forced` where the main line waits and none of the group runs, so it
    /// starts whatever the strategy says, as a group's task does (D15).
    fn own_admits(&self, own: &Own, waiting: i64, forced: bool) -> bool {
        let mut own = own.borrow_mut();
        let Some(strategy) = own.as_mut() else {
            return true;
        };
        let decision = strategy.decide(load_of(self.running_members(), waiting));
        if forced || decision.start > 0 {
            *own = None;
            crate::block::granted(decision.budget);
            return true;
        }
        false
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
            self.pump_waiting(cx, true);
            if ended.is_none() {
                match wait.as_mut().poll(cx) {
                    std::task::Poll::Ready(value) => ended = Some(value),
                    std::task::Poll::Pending => {
                        let Some(parent) = self.parent.as_ref().filter(|frame| frame.holds.get())
                        else {
                            return std::task::Poll::Pending;
                        };
                        parent.give_back();
                        self.pump_waiting(cx, true);
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
        let admission = Rc::new(RefCell::new(self.admitting.borrow_mut().take()));
        self.members.borrow_mut().push(Member { cell, admission });
        handle
    }
}

impl Line for Waiting<'_> {
    fn pump(&self, cx: &mut std::task::Context<'_>) {
        self.pump_waiting(cx, false);
    }
}

impl Waiting<'_> {
    /// [`Line::pump`]; `waits` where the main line waits here (a `join`,
    /// the end of a block), so a task whose own strategy has not admitted
    /// it starts where none of the group runs (D15, D37).
    fn pump_waiting(&self, cx: &mut std::task::Context<'_>, waits: bool) {
        if let Some(outer) = self.outer {
            outer.pump(cx);
        }
        loop {
            let members: Vec<_> = self
                .members
                .borrow()
                .iter()
                .filter_map(|member| {
                    member
                        .cell
                        .upgrade()
                        .map(|cell| (cell, member.admission.clone()))
                })
                .collect();
            let mut waiting = members.iter().filter(|(member, _)| member.queued()).count() as i64;
            let forced = waits && !members.iter().any(|(member, _)| member.running());
            let mut progressed = false;
            let mut blocked = false;
            for (member, own) in &members {
                if member.queued() {
                    // First in first out: none starts before one written
                    // earlier that still waits. One its own strategy does
                    // not admit yet is asked again where the main line
                    // next pauses or waits.
                    if blocked || !self.own_admits(own, waiting, forced) {
                        blocked = true;
                        continue;
                    }
                    if !self.room(waiting) {
                        for gate in &self.gates {
                            gate.want(cx);
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
                .retain(|member| member.cell.strong_count() > 0);
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

/// **The memory the system reports** (ADR-327 D15): the cgroup's limit where
/// one is set (v2, then v1), else the memory available; `0` where the system
/// does not say.
pub(crate) fn memory_reported() -> i64 {
    let read = |path: &str| std::fs::read_to_string(path).ok();
    let limit = read("/sys/fs/cgroup/memory.max")
        .or_else(|| read("/sys/fs/cgroup/memory/memory.limit_in_bytes"))
        .and_then(|text| text.trim().parse::<i64>().ok())
        // v1 writes a limit near `i64::MAX` where none is set.
        .filter(|&limit| limit > 0 && limit < i64::MAX / 2);
    if let Some(limit) = limit {
        return limit;
    }
    read("/proc/meminfo")
        .and_then(|info| {
            info.lines()
                .find(|line| line.starts_with("MemAvailable:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|kib| kib.parse::<i64>().ok())
        })
        .map_or(0, |kib| kib.saturating_mul(1024))
}
