//! **How many tasks of one form run at once** ([ADR-async-5](../../../../docs/specification/adr/adr-async-5.md)
//! D15, D18): the gate of an `async(limit: n, admission: s) { … }`.
//!
//! The main line starts the tasks of a group in written order. Where the gate
//! of the group has no free place, the next task is queued, unstarted, and
//! the main line goes on (D37); where it next waits, the running tasks are
//! polled and the queued ones start as places free, in written order
//! ([`Waiting`]). A place is free where fewer than `n` hold one and the
//! strategy ([`Admission`]) lets one more start; where none runs, one starts
//! whatever the strategy says, so a group always ends. A task is under every
//! gate around it in its function and holds a place in each until it ends,
//! is cancelled or crashes ([`Gate::occupy`]).

use std::cell::RefCell;
use std::rc::Weak;

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
/// ([`Gate::occupy`]); it keeps nothing of theirs.
pub struct Gate {
    limit: usize,
    admission: RefCell<Box<dyn Admission>>,
    state: RefCell<GateState>,
}

struct GateState {
    /// Tasks holding a place.
    running: usize,
    /// Starts the last decision left.
    allowance: i64,
}

impl Gate {
    pub fn new(limit: usize, admission: Box<dyn Admission>) -> Self {
        Gate {
            limit,
            admission: RefCell::new(admission),
            state: RefCell::new(GateState {
                running: 0,
                allowance: 0,
            }),
        }
    }

    /// Whether the next task may start now, `waiting` of them wanting to:
    /// fewer than `limit` hold a place and the strategy lets one more start,
    /// or none holds one. A start it allows is counted.
    pub fn room(&self) -> bool {
        self.room_for(1)
    }

    fn room_for(&self, waiting: i64) -> bool {
        let mut state = self.state.borrow_mut();
        if state.running >= self.limit {
            return false;
        }
        if state.allowance <= 0 {
            let load = Load {
                running: state.running as i64,
                waiting,
                descriptors_in_use: descriptors_in_use(),
                descriptors_allowed: descriptors_allowed(),
            };
            drop(state);
            let decision = self.admission.borrow_mut().decide(load);
            state = self.state.borrow_mut();
            state.allowance = decision.start;
        }
        if state.allowance > 0 {
            state.allowance -= 1;
            return true;
        }
        state.running == 0
    }

    /// **The place a task holds** from its start to its end, its cancel or
    /// its crash; taken where the task starts.
    pub fn occupy(&self) -> Placed<'_> {
        self.state.borrow_mut().running += 1;
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
        let mut state = self.gate.state.borrow_mut();
        state.running = state.running.saturating_sub(1);
    }
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
/// ([`Waiting::during`], [`Waiting::place`]) - the running tasks are polled,
/// and the waiting ones start as places free, in written order. A group
/// inside another gated one of the same function is under both gates
/// ([`Waiting::nested`]).
pub struct Waiting<'x> {
    gates: Vec<&'x Gate>,
    outer: Option<&'x (dyn Line + 'x)>,
    members: RefCell<Vec<Weak<dyn Pollable + 'x>>>,
}

impl<'x> Waiting<'x> {
    /// The tasks of the group whose gate is `gate`.
    pub fn new(gate: &'x Gate) -> Self {
        Waiting {
            gates: vec![gate],
            outer: None,
            members: RefCell::new(Vec::new()),
        }
    }

    /// The tasks of a group inside the gated group of `outer`, in the same
    /// function: each needs a place in every gate around it (D15).
    pub fn nested<'y: 'x>(outer: &'x Waiting<'y>, gate: &'x Gate) -> Self {
        let mut gates: Vec<&'x Gate> = outer.gates.clone();
        gates.push(gate);
        Waiting {
            gates,
            outer: Some(outer),
            members: RefCell::new(Vec::new()),
        }
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
    /// polled, the waiting ones start as places free (D37).
    pub async fn during<F: std::future::Future>(&self, wait: F) -> F::Output {
        let mut wait = std::pin::pin!(wait);
        std::future::poll_fn(|cx| {
            self.pump(cx);
            wait.as_mut().poll(cx)
        })
        .await
    }

    /// **A statement task waits for its place** before it runs to its end
    /// where it starts (D9, D20): until every task written before it has
    /// started and a gate has room for it.
    pub async fn place(&self) {
        std::future::poll_fn(|cx| {
            self.pump(cx);
            if self.queues() {
                return std::task::Poll::Pending;
            }
            std::task::Poll::Ready(())
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
