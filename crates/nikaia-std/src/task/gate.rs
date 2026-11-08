//! **How many tasks of one form run at once** ([ADR-async-5](../../../../docs/specification/adr/adr-async-5.md)
//! D15, D18): the gate of an `async(limit: n, admission: s) { … }`.
//!
//! The main line starts the tasks of a group in written order. Where the gate
//! of the group has no free place, the next task waits there, unstarted,
//! while the main line polls the running ones ([`Waiting::place`]) - a task
//! of a group is polled where it is waited for (D20), so nothing else would -
//! until one of them ends. A place is free where fewer than `n` hold one and
//! the strategy ([`Admission`]) lets one more start; where none runs, one
//! starts whatever the strategy says, so a group always ends. A task is under
//! every gate around it in its function and holds a place in each until it
//! ends, is cancelled or crashes ([`Gate::occupy`]).

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
    /// Whether it has not ended yet.
    fn running(&self) -> bool;
}

impl Pollable for super::group::Rested<'_> {
    fn poll_here(&self, cx: &mut std::task::Context<'_>) -> bool {
        let Ok(mut held) = self.try_borrow_mut() else {
            return false;
        };
        let Some(task) = held.as_mut() else {
            return false;
        };
        if task.as_mut().poll(cx).is_ready() {
            *held = None;
            return true;
        }
        false
    }

    fn running(&self) -> bool {
        self.try_borrow().map_or(true, |held| held.is_some())
    }
}

/// **The gate of one `async { … }` with `limit:` or `admission:`** (D15,
/// D18): the places, how many are held, and the strategy. The tasks hold a
/// place through it ([`Gate::occupy`]); it keeps nothing of theirs.
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

    /// Whether the next task may start now: fewer than `limit` hold a place
    /// and the strategy lets one more start, or none holds one.
    pub fn room(&self) -> bool {
        let mut state = self.state.borrow_mut();
        if state.running >= self.limit {
            return false;
        }
        if state.allowance <= 0 {
            let load = Load {
                running: state.running as i64,
                waiting: 1,
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
    /// its crash; taken where the task starts, once [`Waiting::place`] or
    /// [`Gate::room`] found one.
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

/// **The running tasks of a gate, as its main line sees them** (D15, D20):
/// the cells of their rests, held weakly, so the main line can poll them
/// while the next task waits, unstarted, for a place. Declared inside the
/// group's function before its first task, and used on its main line only.
pub struct Waiting<'x> {
    members: RefCell<Vec<Weak<dyn Pollable + 'x>>>,
}

impl<'x> Waiting<'x> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Waiting {
            members: RefCell::new(Vec::new()),
        }
    }

    /// **Wait for a place in `gate`**, polling the running tasks until one
    /// ends and the gate has room, in written order since the main line
    /// starts the tasks one after the other. Where none of them can be polled
    /// any more, the task starts: nothing it could wait for is left.
    pub async fn place(&self, gate: &Gate) {
        std::future::poll_fn(|cx| {
            loop {
                if gate.room() {
                    return std::task::Poll::Ready(());
                }
                let members: Vec<_> = self
                    .members
                    .borrow()
                    .iter()
                    .filter_map(Weak::upgrade)
                    .collect();
                if !members.iter().any(|member| member.running()) {
                    return std::task::Poll::Ready(());
                }
                let mut ended = false;
                for member in &members {
                    ended |= member.poll_here(cx);
                }
                drop(members);
                self.members
                    .borrow_mut()
                    .retain(|member| member.strong_count() > 0);
                if !ended {
                    return std::task::Poll::Pending;
                }
            }
        })
        .await
    }

    /// **A pausing task started under the gate**: its rest is kept, to poll it
    /// while a later task waits for a place.
    pub fn keep<'h: 'x, T: 'h, E: 'h>(
        &self,
        handle: super::ChildHandle<'h, T, E>,
    ) -> super::ChildHandle<'h, T, E> {
        let cell: Weak<dyn Pollable + 'x> = handle.pollable();
        self.members.borrow_mut().push(cell);
        handle
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
