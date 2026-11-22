//! `std::supervisor`: a supervisor runs tasks and restarts them when they end
//! ([ADR-328](../../../docs/specification/adr/adr-328.md), Part II 12.8).
//!
//! ```nika
//! supervisor::run([
//!     supervisor::child(fn { refresh_loop(cache) }),
//!     supervisor::child(fn { serve(config) }; restart: supervisor::Kind::Transient),
//! ]; strategy: supervisor::Strategy::OneForOne)
//! ```
//!
//! **The list's order is the start order and the dependency** (D3): children
//! start in list order and stop in reverse. The strategy says which siblings
//! restart with a crashed child, the child's kind which ends are restarted,
//! and the policy when. `run` returns only when the supervisor gives up, and
//! throws [`Escalated`] then, after stopping every child.
//!
//! **When, a policy decides** (D5): a value of a type implementing
//! [`Restart`], `std`'s default [`Backoff`] where the child names none. The
//! trait, the types it speaks in and `Backoff` are Nikaia, `supervisor.nika`,
//! lowered to `supervisor.rs` and included here.

use std::time::Instant;

use crate::task::TaskHandle;

/// The lowered half, in a module of its own so that only it lifts the crate's
/// `deny(unsafe_code)` for the emitter's `proven` calls (ADR-218 D5), as
/// `std::text`'s does; it reads spans with `time`'s methods. A lint of the
/// lowered text is allowed here, not fixed in it (`Backoff::new` has no
/// `Default`: `Backoff()` is how Nikaia makes one).
#[allow(unsafe_code, clippy::new_without_default)]
mod lowered {
    #[allow(unused_imports)]
    use crate::time::{DurationExt, DurationIn};
    include!("supervisor.rs");
}
pub use lowered::*;

/// Which siblings restart with a child that crashed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// The child alone.
    OneForOne,
    /// Every child.
    OneForAll,
    /// The child and every child after it in the list.
    RestForOne,
}

/// Which ends of a child are restarted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Every end, a normal one too.
    Permanent,
    /// A panic or a thrown error, not a normal end.
    Transient,
    /// None; a crash is reported.
    Temporary,
}

/// **What `run` throws when it gives up**: the policy of a child that crashed
/// said so. Every child was stopped first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Escalated {
    /// What the last crash said.
    pub message: String,
}

impl crate::process::Ending for Escalated {
    fn exit_code(&self) -> Option<i32> {
        None
    }
}

/// **A child's fatal error, handed on** (ADR-346 D7): it is not restarted, every
/// child stops, and `run` throws this, which is fatal itself, so only a `catch`
/// arm that names it takes it, or one that names the child's own error type,
/// which it carries. It says what the child's error said, and ends the program
/// as that error would have (an `Exit`'s code, or `1`).
#[derive(Debug)]
pub struct Handed {
    /// What the child's error said.
    pub message: String,
    exit: Option<i32>,
    code: i32,
    /// The child's error itself, where its type can cross threads.
    itself: Option<Box<dyn std::error::Error + Send>>,
}

impl Handed {
    /// The child's error itself, if it was carried.
    pub(crate) fn itself(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.itself
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }

    /// The child's error itself, taken out.
    pub(crate) fn into_itself(self) -> Option<Box<dyn std::error::Error>> {
        self.itself.map(|e| e as Box<dyn std::error::Error>)
    }
}

impl PartialEq for Handed {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message && self.exit == other.exit && self.code == other.code
    }
}

impl crate::process::Ending for Handed {
    fn exit_code(&self) -> Option<i32> {
        self.exit
    }

    fn fatal_code(&self) -> Option<i32> {
        Some(self.code)
    }
}

impl std::fmt::Display for Handed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Handed {}

/// How `run` ends: it gave up, or it handed a fatal error on.
#[derive(Debug, PartialEq)]
pub enum Stopped {
    Escalated(Escalated),
    Handed(Handed),
}

impl crate::process::Ending for Stopped {
    fn exit_code(&self) -> Option<i32> {
        match self {
            Stopped::Escalated(_) => None,
            Stopped::Handed(handed) => handed.exit,
        }
    }

    fn fatal_code(&self) -> Option<i32> {
        match self {
            Stopped::Escalated(_) => None,
            Stopped::Handed(handed) => Some(handed.code),
        }
    }
}

impl std::fmt::Display for Stopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stopped::Escalated(e) => e.fmt(f),
            Stopped::Handed(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Stopped {}

impl std::fmt::Display for Escalated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the supervisor gave up: {}", self.message)
    }
}

impl std::error::Error for Escalated {}

/// What a child's function ends with: nothing, or what it threw.
pub type Outcome = Result<(), Box<dyn std::error::Error>>;

/// A child at `user_parallelism = no`.
pub struct Child {
    body: crate::func::Kept<dyn Fn() -> crate::func::Boxed<Outcome>>,
    kind: Kind,
    /// Asked at every restart, and changed by the asking: `decide` takes
    /// `ref mut self`.
    policy: std::cell::RefCell<Box<dyn Restart>>,
}

/// A child where tasks run on threads.
pub struct PoolChild {
    body: crate::func::Kept<dyn Fn() -> crate::func::SendBoxed<Outcome> + Send + Sync>,
    kind: Kind,
    policy: std::sync::Mutex<Box<dyn Restart + Send>>,
}

/// `supervisor::child(fn { … }; restart: …, policy: …)`.
pub fn child(
    body: crate::func::Kept<dyn Fn() -> crate::func::Boxed<Outcome>>,
    restart: Kind,
    policy: impl Restart + 'static,
) -> Child {
    Child {
        body,
        kind: restart,
        policy: std::cell::RefCell::new(Box::new(policy)),
    }
}

/// [`child`] where tasks run on threads.
pub fn child_on_pool(
    body: crate::func::Kept<dyn Fn() -> crate::func::SendBoxed<Outcome> + Send + Sync>,
    restart: Kind,
    policy: impl Restart + Send + 'static,
) -> PoolChild {
    PoolChild {
        body,
        kind: restart,
        policy: std::sync::Mutex::new(Box::new(policy)),
    }
}

/// A fatal error's `Exit` code, the status it sets, and the error itself.
type Fatal = (Option<i32>, i32, Option<Box<dyn std::error::Error + Send>>);

/// How one attempt of a child ended: normally, or with what it said and, for a
/// fatal error, what [`Fatal`] keeps of it.
type Ended = Option<(String, Option<Fatal>)>;

/// The message a child's outcome carries, where it is a failure.
fn ended(outcome: Outcome) -> Ended {
    use crate::process::Ending;
    outcome.err().map(|error| {
        let said = error.to_string();
        let fatal = error
            .fatal_code()
            .map(|code| (error.exit_code(), code, crate::task::crossed(error)));
        (said, fatal)
    })
}

/// **Run the supervisor** until it gives up (ADR-328 D1-D4).
pub async fn run(children: &[Child], strategy: Strategy) -> Result<(), Stopped> {
    let kinds: Vec<Kind> = children.iter().map(|c| c.kind).collect();
    supervise(
        &kinds,
        strategy,
        |at| {
            let body = children[at].body.clone();
            TaskHandle::start(async move { ended(body().await) })
        },
        |at, history| children[at].policy.borrow_mut().decide(history),
    )
    .await
}

/// [`run`] where tasks run on threads.
pub async fn run_on_pool(children: &[PoolChild], strategy: Strategy) -> Result<(), Stopped> {
    let kinds: Vec<Kind> = children.iter().map(|c| c.kind).collect();
    supervise(
        &kinds,
        strategy,
        |at| {
            let body = children[at].body.clone();
            TaskHandle::start_on_pool(async move { ended(body().await) })
        },
        |at, history| {
            children[at]
                .policy
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .decide(history)
        },
    )
    .await
}

/// The supervisor itself, over a way to start child `at` and to ask child
/// `at`'s policy: written once for both executors, and for the tests' policies.
///
/// **The history is the supervisor's**, per child: how long the attempt that
/// just ended ran, how many failures the child has had, and how long since the
/// first (D5). Its cycle is the child's life; a policy that starts a new one
/// after a healthy run - [`Backoff`] - keeps where it began itself.
pub(crate) async fn supervise(
    kinds: &[Kind],
    strategy: Strategy,
    start: impl Fn(usize) -> TaskHandle<Ended>,
    decide: impl Fn(usize, Failures) -> Next,
) -> Result<(), Stopped> {
    let count = kinds.len();
    let mut running: Vec<Option<TaskHandle<Ended>>> =
        (0..count).map(|at| Some(start(at))).collect();
    let mut started: Vec<Instant> = vec![Instant::now(); count];
    let mut failures: Vec<i64> = vec![0; count];
    let mut first: Vec<Option<Instant>> = vec![None; count];
    loop {
        if running.iter().all(Option::is_none) {
            return Ok(());
        }
        let (at, outcome) = FirstEnded { running: &running }.await;
        running[at] = None;
        let crash = match outcome {
            Ok(None) => None,
            Ok(Some((thrown, None))) => Some(thrown),
            // **A fatal error is not restarted** (ADR-346 D7): every child
            // stops and the supervisor hands it on.
            Ok(Some((thrown, Some((exit, code, itself))))) => {
                for handle in running.iter_mut().rev() {
                    if let Some(handle) = handle.take() {
                        handle.cancel();
                    }
                }
                return Err(Stopped::Handed(Handed {
                    message: thrown,
                    exit,
                    code,
                    itself,
                }));
            }
            Err(crashed) => Some(crashed.message),
        };
        let restarted = match kinds[at] {
            Kind::Permanent => true,
            Kind::Transient => crash.is_some(),
            Kind::Temporary => {
                if let Some(said) = &crash {
                    eprintln!("supervisor: a temporary child crashed and is not restarted: {said}");
                }
                false
            }
        };
        if !restarted {
            continue;
        }
        let said = crash.unwrap_or_else(|| "the child ended".to_string());
        // **The policy of the child that crashed decides for the siblings it
        // takes along** (Part II 12.8).
        let now = Instant::now();
        failures[at] += 1;
        let history = Failures {
            last_uptime: now.duration_since(started[at]),
            consecutive: failures[at],
            since_first: now.duration_since(*first[at].get_or_insert(now)),
        };
        match decide(at, history) {
            Next::Escalate => {
                for handle in running.iter_mut().rev() {
                    if let Some(handle) = handle.take() {
                        handle.cancel();
                    }
                }
                return Err(Stopped::Escalated(Escalated { message: said }));
            }
            Next::Delay(span) => crate::time::sleep(span).await,
            Next::Immediate => {}
        }
        let again: Vec<usize> = match strategy {
            Strategy::OneForOne => vec![at],
            Strategy::OneForAll => (0..count).collect(),
            Strategy::RestForOne => (at..count).collect(),
        };
        // Stopped in reverse order, started in list order.
        for &sibling in again.iter().rev() {
            if let Some(handle) = running[sibling].take() {
                handle.cancel();
            }
        }
        for &sibling in &again {
            running[sibling] = Some(start(sibling));
            started[sibling] = Instant::now();
        }
    }
}

/// The first of the running children to end, and how.
struct FirstEnded<'a> {
    running: &'a [Option<TaskHandle<Ended>>],
}

impl std::future::Future for FirstEnded<'_> {
    type Output = (usize, Result<Ended, crate::task::Crashed>);

    fn poll(
        self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        for (at, handle) in self.running.iter().enumerate() {
            if let Some(handle) = handle
                && let std::task::Poll::Ready(outcome) = handle.poll_ended(context)
            {
                return std::task::Poll::Ready((at, outcome));
            }
        }
        std::task::Poll::Pending
    }
}
