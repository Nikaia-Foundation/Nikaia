//! The one place a Nikaia program runs two things at once.
//!
//! The emitter names what is in this module and nothing else, so which vehicle
//! carries an overlap is this file's decision rather than a shape baked into
//! every generated program (ADR-292).
//!
//! **Two things a program can ask for, and they are different questions:**
//!
//! * [`TaskHandle`] is a `spawn` - one piece of work the executor owns, joined
//!   later or not at all ([ADR-055](../../../docs/specification/adr/adr-055.md)
//!   D5).
//! * `overlap2`..`overlap8` are an `overlap { … }` block - every branch in
//!   flight in one pass, one function per arity
//!   ([ADR-292](../../../docs/specification/adr/adr-292.md) D2).
//! * `race2`..`race8` are a `select { … }` block - the same branches in flight,
//!   and the **first** to finish is the one that is kept
//!   ([ADR-292](../../../docs/specification/adr/adr-292.md) D12). The pair D4
//!   names, one module apart from nothing.
//!
//! Both take **futures**, because a branch **is** one: an `async` block that
//! borrows what is around it and is started once, where a closure would add a
//! call and take nothing away. *Not* because the language below has no `async`
//! closure - it has one, measured
//! ([ADR-277](../../../docs/specification/adr/adr-277.md) D13), and this line
//! used to say otherwise. What ran on the pool
//! instead — `both`, the closure pair ADR-292 D6 chose for a group the
//! *compiler* put together — went with the automatic grouping itself
//! (ADR-292 D1).

/// **What a `spawn` hands back** (Part I 8.2,
/// [ADR-055](../../../docs/specification/adr/adr-055.md) D5).
///
/// ```nika
/// let handle = spawn fn { process(path) }
/// let result = handle.join()
/// ```
///
/// `.join()` is where two tasks meet again, and it is an `.await`: the task
/// fills a slot and wakes whoever is waiting on it, so joining gives the thread
/// up rather than holding it. That is what makes Part II 11.2's *"uniform API"*
/// true - the same two lines mean the same thing at either setting of
/// `user_parallelism`, and the only difference is how many threads the executor
/// has.
///
/// **A task nobody joins still runs** (D5), which Part I 8.2's own example
/// needs: the executor owns the task, so dropping the handle drops the handle
/// and not the work.
pub struct TaskHandle<T> {
    slot: std::sync::Arc<crate::rt::exec::Slot<T>>,
    asked: std::sync::Arc<Cancelled>,
}

/// **The request a [`TaskHandle::cancel`] makes**
/// ([ADR-292](../../../docs/specification/adr/adr-292.md) D14).
///
/// A flag and the waker of whoever is running the task, which is what makes the
/// request *prompt* rather than eventual: without the waker a task parked on
/// I/O would sit in the queue until that I/O answered, and only then notice it
/// had been cancelled.
struct Cancelled {
    asked: std::sync::atomic::AtomicBool,
    waking: std::sync::Mutex<Option<std::task::Waker>>,
}

impl Cancelled {
    fn nobody_asked() -> std::sync::Arc<Cancelled> {
        std::sync::Arc::new(Cancelled {
            asked: std::sync::atomic::AtomicBool::new(false),
            waking: std::sync::Mutex::new(None),
        })
    }
}

/// A task's body, wrapped in what makes it cancellable.
///
/// **The body is held in an `Option` so that it can be dropped early**, and
/// dropping it is the whole of D2: a future dropped at its suspension point
/// tears its values down, a `cleanup` that pauses is adopted by the runtime and
/// bounded by the `cleanup-deadline`
/// ([ADR-297](../../../docs/specification/adr/adr-297.md) D5), and nobody waits
/// for any of it.
struct Cancellable<F, T, Q: crate::cleanup::Queue> {
    body: Option<std::pin::Pin<Box<F>>>,
    slot: std::sync::Arc<crate::rt::exec::Slot<T>>,
    asked: std::sync::Arc<Cancelled>,
    /// **The cleanups this task parked**
    /// ([ADR-297](../../../docs/specification/adr/adr-297.md) D1): swapped in
    /// while it is polled, so that what it parks and what it settles are its
    /// own, and handed to the runtime as orphans when it is over.
    queues: Q,
}

impl<F: std::future::Future<Output = T>, T, Q: crate::cleanup::Queue + Unpin> std::future::Future
    for Cancellable<F, T, Q>
{
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<()> {
        let me = self.get_mut();
        me.queues.swap();
        let polled = me.step(context);
        me.queues.swap();
        // **What is still parked when the task is over is nobody's to settle**:
        // a cancelled task's values, and one that died after the last settle
        // point. The runtime finishes them before the program ends (ADR-297
        // D5).
        if polled.is_ready() {
            me.queues.orphan();
        }
        polled
    }
}

/// **A task dropped before it was over** - the runtime abandoning it at the
/// deadline, or a queue that goes with its thread - tears its values down in
/// its own queues too, so what they park is an orphan like a cancelled task's
/// and not a stranger's to settle (ADR-297 D5).
impl<F, T, Q: crate::cleanup::Queue> Drop for Cancellable<F, T, Q> {
    fn drop(&mut self) {
        if self.body.is_some() {
            self.queues.swap();
            self.body = None;
            self.queues.swap();
        }
        self.queues.orphan();
    }
}

impl<F: std::future::Future<Output = T>, T, Q: crate::cleanup::Queue> Cancellable<F, T, Q> {
    /// One poll of the body, with the task's own queues in place.
    fn step(&mut self, context: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        if self.asked.asked.load(std::sync::atomic::Ordering::Acquire) {
            // **The teardown is the drop**, and it happens here rather than in
            // `cancel` because this is the task's own thread and its pause
            // point.
            self.body = None;
            return std::task::Poll::Ready(());
        }
        let Some(body) = self.body.as_mut() else {
            return std::task::Poll::Ready(());
        };
        // **A panic ends this task and nothing else**
        // ([ADR-326](../../../docs/specification/adr/adr-326.md) D4), at both
        // settings: caught at the task's edge, after the hook has said where it
        // happened. The body is dropped, so its values are torn down and its
        // cleanups run as a cancelled task's do, and whoever joins it is told.
        let polled = {
            let _inside = InATask::enter();
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body.as_mut().poll(context)))
        };
        let polled = match polled {
            Ok(polled) => polled,
            Err(payload) => {
                self.body = None;
                self.slot.fail(Crashed {
                    message: panic_message(payload.as_ref()),
                    site: SITE.with(|held| std::mem::take(&mut *held.borrow_mut())),
                });
                return std::task::Poll::Ready(());
            }
        };
        match polled {
            std::task::Poll::Ready(value) => {
                self.body = None;
                self.slot.fill(value);
                std::task::Poll::Ready(())
            }
            std::task::Poll::Pending => {
                *self.asked.waking.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(context.waker().clone());
                std::task::Poll::Pending
            }
        }
    }
}

/// **What `join` throws for a task that panicked**
/// ([ADR-328](../../../docs/specification/adr/adr-328.md) D8): the crash
/// happened in another task, so the joiner's own state is whole and the crash
/// is an error it may catch, as any other is.
///
/// `message` is what the panic said; `site` is where, as the hook named it
/// (`src/main.nika:4`), or empty where no `.nika` line is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crashed {
    pub message: String,
    pub site: String,
}

impl std::fmt::Display for Crashed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.site.is_empty() {
            true => write!(f, "the task crashed: {}", self.message),
            false => write!(f, "the task crashed at {}: {}", self.site, self.message),
        }
    }
}

impl std::error::Error for Crashed {}

thread_local! {
    /// **Where the last panic on this thread happened**, as the hook named it:
    /// the catch at the task's edge has the payload and not the location.
    static SITE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// The hook's half of [`Crashed::site`]: where a task's panic happened.
pub(crate) fn panicked_at(site: String) {
    SITE.with(|held| *held.borrow_mut() = site);
}

/// What a panic said, in the two shapes a payload comes in.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "it panicked".to_string())
}

thread_local! {
    /// How many task bodies this thread is polling right now: the panic hook
    /// asks it, to say a task stopped rather than the program.
    static IN_A_TASK: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// **Whether the code running on this thread is a task's** rather than
/// `main`'s ([ADR-326](../../../docs/specification/adr/adr-326.md) D4): a panic
/// there ends the task, and the program goes on.
pub fn in_a_task() -> bool {
    IN_A_TASK.with(|depth| depth.get() > 0)
}

/// One poll of a task's body, counted for [`in_a_task`] while it lasts.
struct InATask;

impl InATask {
    fn enter() -> InATask {
        IN_A_TASK.with(|depth| depth.set(depth.get() + 1));
        InATask
    }
}

impl Drop for InATask {
    fn drop(&mut self) {
        IN_A_TASK.with(|depth| depth.set(depth.get() - 1));
    }
}

impl<T: 'static> TaskHandle<T> {
    /// Start `body` as a task, and hand back the handle to its value.
    ///
    /// **The captures have already moved**, because the emitter writes the body
    /// as an `async move` block - which is Part I 8.3's implicit move and Rust's
    /// `move` meeting at the same place, with `NK2101` in front of it for the
    /// data the parent still wanted.
    pub fn start(body: impl std::future::Future<Output = T> + 'static) -> TaskHandle<T> {
        let slot = crate::rt::exec::Slot::empty();
        let asked = Cancelled::nobody_asked();
        crate::rt::exec::start(Cancellable {
            body: Some(Box::pin(body)),
            slot: slot.clone(),
            asked: asked.clone(),
            queues: crate::cleanup::Queues::default(),
        });
        TaskHandle { slot, asked }
    }
}

impl<T> TaskHandle<T> {
    /// Whether the task has ended, without taking the handle: what a
    /// supervisor asks of every child at once (ADR-328).
    pub(crate) fn poll_ended(
        &self,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<T, Crashed>> {
        self.slot.take(context)
    }

    /// **Stop the task** ([ADR-292](../../../docs/specification/adr/adr-292.md)
    /// D3), with exactly the semantics losing a `select` has.
    ///
    /// The task stops at its current pause point, its values are torn down, and
    /// a `cleanup` that pauses is adopted by the runtime and finished in the
    /// background. The caller does not wait for any of that, because it should
    /// not pay for it (D2).
    ///
    /// **It takes the handle**, the way `join` does, and that is what makes
    /// §4's open question — *can a cancelled task be observed to have been
    /// cancelled?* — one no program can ask: after this there is no handle to
    /// ask with. It also means a cancelled task is never joined, so nothing
    /// waits for a value that is not coming.
    pub fn cancel(self) {
        self.asked
            .asked
            .store(true, std::sync::atomic::Ordering::Release);
        // **Wake it so that it notices now.** A task parked on I/O has a clear
        // alarm, and the executor polls what its alarm says is ready.
        let waking = self
            .asked
            .waking
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(waker) = waking {
            waker.wake();
        }
        // And the thread that is not this one, at `user_parallelism = yes`.
        crate::rt::ring_the_bell();
    }
}

impl<T: Send + 'static> TaskHandle<T> {
    /// The same, on the **pool**, where another thread may pick the task up
    /// ([ADR-055](../../../docs/specification/adr/adr-055.md) §6 step 1's `yes`
    /// half, [ADR-037](../../../docs/specification/adr/adr-037.md) D2).
    ///
    /// **Two functions and not one with a bound**, because the bound is the
    /// difference and it belongs to the build rather than to the language. The
    /// emitter writes this line at `user_parallelism = yes` and [`start`] at
    /// `no`, from one Nikaia `spawn` — so a program at the default is never
    /// asked for a `Send` its setting does not need, which is what keeps
    /// [ADR-312](../../../docs/specification/adr/adr-312.md) D9's plain count
    /// reachable from inside a task.
    ///
    /// [`start`]: TaskHandle::start
    pub fn start_on_pool(
        body: impl std::future::Future<Output = T> + Send + 'static,
    ) -> TaskHandle<T> {
        let slot = crate::rt::exec::Slot::empty();
        let asked = Cancelled::nobody_asked();
        crate::rt::exec::start_on_pool(Cancellable {
            body: Some(Box::pin(body)),
            slot: slot.clone(),
            asked: asked.clone(),
            queues: crate::cleanup::SendQueues::default(),
        });
        TaskHandle { slot, asked }
    }

    /// The task's value, once it has one. **A suspension point**, not a wait.
    /// A task that panicked has none, and this throws [`Crashed`] (ADR-328 D8).
    pub async fn join(self) -> Result<T, Crashed> {
        crate::rt::exec::Waiting::on(self.slot).await
    }
}

impl<T> TaskHandle<T> {
    /// **Nobody waits for this task** ([ADR-346](../../../docs/specification/adr/adr-346.md)
    /// D2): it runs on, and the handle is given up.
    pub fn detach(self) {}
}

/// **A task whose body can fail** ([ADR-346](../../../docs/specification/adr/adr-346.md)
/// D5): `join` hands on the body's error, in the channel of the function that
/// started it, beside [`Crashed`].
pub struct FailingTask<T, E> {
    handle: TaskHandle<Result<T, E>>,
    watch: Watch,
}

impl<T: 'static, E: 'static> FailingTask<T, E> {
    /// Start a body that hands back a `Result`; `report` is what its error
    /// says in the log if nobody waits for it (ADR-346 D6).
    pub fn start(
        body: impl std::future::Future<Output = Result<T, E>> + 'static,
        report: fn(&E) -> Report,
    ) -> Self {
        let watch = Watch::armed();
        let told = watch.clone_state();
        FailingTask {
            handle: TaskHandle::start(async move {
                let outcome = body.await;
                if let Err(error) = &outcome {
                    told.failed(report(error));
                }
                outcome
            }),
            watch,
        }
    }
}

impl<T: Send + 'static, E: Send + 'static> FailingTask<T, E> {
    /// The same, on the pool.
    pub fn start_on_pool(
        body: impl std::future::Future<Output = Result<T, E>> + Send + 'static,
        report: fn(&E) -> Report,
    ) -> Self {
        let watch = Watch::armed();
        let told = watch.clone_state();
        FailingTask {
            handle: TaskHandle::start_on_pool(async move {
                let outcome = body.await;
                if let Err(error) = &outcome {
                    told.failed(report(error));
                }
                outcome
            }),
            watch,
        }
    }
}

impl<T, E> FailingTask<T, E> {
    /// Stop the task, as [`TaskHandle::cancel`].
    pub fn cancel(self) {
        self.watch.disarm();
        self.handle.cancel()
    }

    /// **Nobody waits for this task** (ADR-346 D6): its error goes to the log
    /// at `main`'s end. Dropping the handle - a bare `spawn` (D2) - is the same.
    pub fn detach(self) {}
}

impl<T: 'static, E: 'static + From<Crashed>> FailingTask<T, E> {
    /// The task's value, or its body's error, or [`Crashed`] in that channel.
    pub async fn join(self) -> Result<T, E> {
        self.watch.disarm();
        match crate::rt::exec::Waiting::on(self.handle.slot).await {
            Ok(outcome) => outcome,
            Err(crashed) => Err(E::from(crashed)),
        }
    }
}

/// **What `join(within:)` throws when the time ran out**
/// ([ADR-346](../../../docs/specification/adr/adr-346.md) D4): the task was
/// cancelled, and its cleanups ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedOut;

impl std::fmt::Display for TimedOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the task did not end in time")
    }
}

impl std::error::Error for TimedOut {}

/// The two ways `TaskHandle::join(within:)` fails; a channel holding both takes
/// it apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Late {
    Crashed(Crashed),
    TimedOut(TimedOut),
}

impl std::fmt::Display for Late {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Late::Crashed(e) => e.fmt(f),
            Late::TimedOut(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Late {}

impl<T> TaskHandle<T> {
    /// Wait for the slot, at most `span`; on time-out, cancel the task.
    async fn within(self, span: std::time::Duration) -> Option<Result<T, Crashed>> {
        use std::future::Future;
        let mut waiting = std::pin::pin!(crate::rt::exec::Waiting::on(self.slot.clone()));
        let mut sleeping = std::pin::pin!(crate::time::sleep(span));
        let outcome = std::future::poll_fn(|context| {
            if let std::task::Poll::Ready(value) = waiting.as_mut().poll(context) {
                return std::task::Poll::Ready(Some(value));
            }
            if sleeping.as_mut().poll(context).is_ready() {
                return std::task::Poll::Ready(None);
            }
            std::task::Poll::Pending
        })
        .await;
        if outcome.is_none() {
            self.cancel();
        }
        outcome
    }

    /// **`join(within: d)`** (ADR-346 D4): the value, or [`Crashed`], or
    /// [`TimedOut`] once `span` ran out, the task cancelled.
    pub async fn join_within(self, span: std::time::Duration) -> Result<T, Late> {
        match self.within(span).await {
            Some(Ok(value)) => Ok(value),
            Some(Err(crashed)) => Err(Late::Crashed(crashed)),
            None => Err(Late::TimedOut(TimedOut)),
        }
    }
}

impl<T: 'static, E: 'static + From<Crashed> + From<TimedOut>> FailingTask<T, E> {
    /// `join(within:)` of a task whose body can fail.
    pub async fn join_within(self, span: std::time::Duration) -> Result<T, E> {
        self.watch.disarm();
        match self.handle.within(span).await {
            Some(Ok(outcome)) => outcome,
            Some(Err(crashed)) => Err(E::from(crashed)),
            None => Err(E::from(TimedOut)),
        }
    }
}

/// An error in the opaque channel on its way from a task on the pool to its
/// joiner. A `Box<dyn Error>` is not `Send`, so what crosses is what the
/// joiner can still act on: the message, and the code of an `Exit`.
#[derive(Debug)]
pub struct Carried {
    message: String,
    exit: Option<i32>,
    fatal: Option<i32>,
    /// The error itself, when its type is one of the program's that can
    /// cross threads ([`crossing`]): a typed `catch` at the join takes it.
    itself: Option<Box<dyn std::error::Error + Send>>,
    /// Where it was raised, so that it is raised there again on this side.
    site: Option<&'static str>,
}

type Crosser = fn(
    Box<dyn std::error::Error>,
) -> Result<Box<dyn std::error::Error + Send>, Box<dyn std::error::Error>>;

/// The program's own error types that can cross threads: `main` registers
/// each before its first statement ([`Cross`]).
static CROSSING: std::sync::Mutex<Vec<Crosser>> = std::sync::Mutex::new(Vec::new());

/// **Register one of the program's error types** so that a task on the pool
/// hands it to its joiner as itself (ADR-346 D5).
pub fn crossing<T: std::error::Error + Send + 'static>() {
    CROSSING
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(|error| match error.downcast::<T>() {
            Ok(found) => Ok(found),
            Err(other) => Err(other),
        });
}

/// **The emitter's way to [`crossing`]** for a type whose `Send` it cannot
/// see: `(&&Cross::<T>::new()).register()` with both traits in scope registers
/// a `Send` type and passes over any other, which crosses as its message.
pub struct Cross<T>(std::marker::PhantomData<fn() -> T>);

impl<T> Cross<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Cross(std::marker::PhantomData)
    }
}

/// The [`Cross`] of a type that can cross threads.
pub trait CrossSend {
    fn register(&self);
}

impl<T: std::error::Error + Send + 'static> CrossSend for &Cross<T> {
    fn register(&self) {
        crossing::<T>()
    }
}

/// The [`Cross`] of any other type.
pub trait CrossNot {
    fn register(&self);
}

impl<T> CrossNot for Cross<T> {
    fn register(&self) {}
}

/// The error under a `Raised` envelope as its own box, if a registered type.
pub(crate) fn crossed(
    error: Box<dyn std::error::Error>,
) -> Option<Box<dyn std::error::Error + Send>> {
    let mut error = match error.downcast::<crate::error::Raised>() {
        Ok(raised) => raised.into_inner(),
        Err(error) => error,
    };
    let crossers = CROSSING.lock().unwrap_or_else(|e| e.into_inner()).clone();
    for crosser in crossers {
        match crosser(error) {
            Ok(found) => return Some(found),
            Err(back) => error = back,
        }
    }
    None
}

impl crate::process::Ending for Carried {
    fn exit_code(&self) -> Option<i32> {
        self.exit
    }

    fn fatal_code(&self) -> Option<i32> {
        self.fatal
    }
}

impl Carried {
    fn from_box(error: Box<dyn std::error::Error>) -> Self {
        use crate::process::Ending;
        let message = error.to_string();
        let exit = error.exit_code();
        let fatal = error.fatal_code();
        let site = crate::error::raised_origin(&*error);
        Carried {
            message,
            exit,
            fatal,
            itself: crossed(error),
            site,
        }
    }

    fn into_box(self) -> Box<dyn std::error::Error> {
        if let Some(itself) = self.itself {
            return match self.site {
                Some(origin) => crate::error::raised_at(itself, origin),
                None => itself,
            };
        }
        match self.exit {
            Some(code) => Box::new(crate::process::Exit::Code(i64::from(code))),
            None => Box::new(self),
        }
    }
}

impl std::fmt::Display for Carried {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Carried {}

/// **A task on the pool whose body fails in the opaque channel**
/// ([ADR-346](../../../docs/specification/adr/adr-346.md) D5): a
/// `Box<dyn Error>` is not `Send`, so it crosses as [`Carried`].
pub struct BoxedTask<T> {
    handle: TaskHandle<Result<T, Carried>>,
    watch: Watch,
}

impl<T: Send + 'static> BoxedTask<T> {
    /// Start a body that hands back a `Result` in the opaque channel.
    pub fn start_on_pool(
        body: impl std::future::Future<Output = Result<T, Box<dyn std::error::Error>>> + Send + 'static,
        _report: fn(&Box<dyn std::error::Error>) -> Report,
    ) -> Self {
        let watch = Watch::armed();
        let told = watch.clone_state();
        BoxedTask {
            handle: TaskHandle::start_on_pool(async move {
                let outcome = body.await.map_err(Carried::from_box);
                if let Err(carried) = &outcome {
                    told.failed(Report {
                        text: carried.message.clone(),
                        code: carried.fatal,
                    });
                }
                outcome
            }),
            watch,
        }
    }

    /// `join(within:)` of such a task.
    pub async fn join_within(
        self,
        span: std::time::Duration,
    ) -> Result<T, Box<dyn std::error::Error>> {
        self.watch.disarm();
        match self.handle.within(span).await {
            Some(Ok(Ok(value))) => Ok(value),
            Some(Ok(Err(carried))) => Err(carried.into_box()),
            Some(Err(crashed)) => Err(Box::new(crashed)),
            None => Err(Box::new(TimedOut)),
        }
    }

    /// The task's value, or its body's error, or [`Crashed`] in the box.
    pub async fn join(self) -> Result<T, Box<dyn std::error::Error>> {
        self.watch.disarm();
        match crate::rt::exec::Waiting::on(self.handle.slot).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(carried)) => Err(carried.into_box()),
            Err(crashed) => Err(Box::new(crashed)),
        }
    }
}

impl<T> BoxedTask<T> {
    /// Stop the task, as [`TaskHandle::cancel`].
    pub fn cancel(self) {
        self.watch.disarm();
        self.handle.cancel()
    }

    /// Nobody waits for this task.
    pub fn detach(self) {}
}

/// **What a detached task's error leaves for the log** (ADR-346 D6): its long
/// form, and the code of a fatal `Exit` in it.
pub struct Report {
    text: String,
    code: Option<i32>,
}

/// **The emitter's way to a [`Report`]** for a channel it names: written as
/// `(&&Probe(e)).report()` with both traits in scope, a channel that can hold
/// an `Exit` ([`crate::process::Ending`]) answers with its code, any other
/// with none.
pub struct Probe<'a, E>(pub &'a E);

/// The [`Probe`] of a channel that can end the program.
pub trait ProbeEnding {
    fn report(&self) -> Report;
}

impl<E: crate::process::Ending + std::fmt::Display> ProbeEnding for &Probe<'_, E> {
    fn report(&self) -> Report {
        Report {
            text: self.0.to_string(),
            code: self.0.fatal_code(),
        }
    }
}

/// The [`Probe`] of any other channel.
pub trait ProbePlain {
    fn report(&self) -> Report;
}

impl<E: std::fmt::Display> ProbePlain for Probe<'_, E> {
    fn report(&self) -> Report {
        Report {
            text: self.0.to_string(),
            code: None,
        }
    }
}

#[derive(Default)]
struct WatchState {
    disarmed: bool,
    detached: bool,
    report: Option<Report>,
}

/// **Whether anybody waits for a failing task** (ADR-346 D6): disarmed by
/// `join` and `cancel`; dropped armed, the task is detached, and an error it
/// had or will have goes to [`DETACHED`].
pub(crate) struct Watch(std::sync::Arc<std::sync::Mutex<WatchState>>);

impl Watch {
    fn armed() -> Watch {
        Watch(std::sync::Arc::default())
    }

    /// The body's side of the watch, which does not detach when dropped.
    fn clone_state(&self) -> Told {
        Told(self.0.clone())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, WatchState> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn disarm(&self) {
        self.lock().disarmed = true;
    }
}

/// The body's side of a [`Watch`].
pub(crate) struct Told(std::sync::Arc<std::sync::Mutex<WatchState>>);

impl Told {
    /// The body ended with an error.
    fn failed(&self, report: Report) {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.detached {
            detached_failed(report);
        } else {
            state.report = Some(report);
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let mut state = self.lock();
        if state.disarmed || state.detached {
            return;
        }
        state.detached = true;
        if let Some(report) = state.report.take() {
            detached_failed(report);
        }
    }
}

/// The errors of detached tasks, in the order they arrived.
static DETACHED: std::sync::Mutex<Vec<Report>> = std::sync::Mutex::new(Vec::new());

/// **An error nobody joins**, from a task `std` runs itself (ADR-351 D6: a
/// server's handler): said at `main`'s end with the detached tasks' errors.
pub(crate) fn detached_error(text: String, code: Option<i32>) {
    detached_failed(Report { text, code });
}

fn detached_failed(report: Report) {
    DETACHED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(report);
}

/// **At `main`'s end, after the drain** (ADR-346 D6): every detached task's
/// error on standard error, and the code of the first fatal one, which
/// replaces the status the program would end with.
pub fn report_detached() -> Option<i32> {
    let reports = std::mem::take(&mut *DETACHED.lock().unwrap_or_else(|e| e.into_inner()));
    let mut first = None;
    for report in reports {
        eprintln!("Error in a detached task: {}", report.text);
        if first.is_none() {
            first = report.code;
        }
    }
    first
}

thread_local! {
    /// How many `overlap` and `select` blocks this thread is polling a branch
    /// of right now.
    static OPEN_BLOCKS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// **A branch of an `overlap` or a `select` is being polled on this thread**
/// ([ADR-263](../../../docs/specification/adr/adr-263.md) D2).
///
/// A block's branches are no tasks: they are polled one after another on the
/// block's own thread (ADR-292 D4). Its first branch starts its read before the
/// second branch has started anything, so the runtime has nothing in flight at
/// that moment - and a read made on the calling thread there would run to its
/// end before the second branch began, which is the sum ADR-292 D6 refuses.
/// So a file operation asks this before it takes the calling thread.
///
/// **Counted per poll, not per block**, because a block in a task on the pool
/// may be polled by one worker and then another: a count held across polls
/// would be raised on one thread and lowered on a different one.
pub(crate) fn inside_a_block() -> bool {
    OPEN_BLOCKS.with(|open| open.get() > 0)
}

/// One poll of a block's branches, counted for [`inside_a_block`] while it
/// lasts.
struct OpenBlock;

impl OpenBlock {
    fn enter() -> OpenBlock {
        OPEN_BLOCKS.with(|open| open.set(open.get() + 1));
        OpenBlock
    }
}

impl Drop for OpenBlock {
    fn drop(&mut self) {
        OPEN_BLOCKS.with(|open| open.set(open.get() - 1));
    }
}

/// **Every branch of an `overlap { … }`, all of them in flight**
/// (Part I 8.1.2, [ADR-292](../../../docs/specification/adr/adr-292.md) D2).
///
/// One arity per macro expansion, because the branches have different types and
/// a tuple of futures is what that means in the language below. The emitter
/// writes `task::overlap3(a, b, c).await` and the arity is how many statements
/// the block had.
///
/// **Flat and not nested, which is D6.** A pair joined with a pair -
/// `two(a, two(b, c))` - polls `a` to completion before `b` is ever started, so
/// a block holding a computation and two reads would cost their sum: the naive
/// order D6 names and refuses. Polling every branch in one pass costs `max` instead: a branch that
/// suspends returns `Pending` at its first suspension point and the next branch
/// is started at once.
///
/// **The order it polls in is the order it is given**, and the emitter hands the
/// branches over with the ones that can pause first — which is D6's rule, read
/// off the ledger's `sync` column. The results go back into written order at the
/// call, so this never has to know about it.
///
/// **A failure is the caller's**, not this function's: a branch that can fail
/// hands back a `Result` like any other value, and D5's "the first in written
/// order wins" is the emitter's `?` on the tuple rather than a rule here.
macro_rules! overlapping {
    ($name:ident, $($branch:ident : $result:ident),+) => {
        // One parameter per branch is what an arity *is*, so the argument count
        // is the point rather than a smell: `overlap8` takes eight branches
        // because a block of eight has eight.
        #[allow(non_snake_case, clippy::too_many_arguments)]
        pub async fn $name<$($branch, $result),+>($($branch: $branch),+) -> ($($result),+)
        where
            $($branch: std::future::Future<Output = $result>),+
        {
            $(let mut $branch = Box::pin($branch);)+
            $(let mut $result: Option<$result> = None;)+

            std::future::poll_fn(move |context| {
                let _open = OpenBlock::enter();
                $(
                    if $result.is_none() {
                        if let std::task::Poll::Ready(value) = $branch.as_mut().poll(context) {
                            $result = Some(value);
                        }
                    }
                )+
                if $($result.is_some())&&+ {
                    return std::task::Poll::Ready((
                        $($result.take().expect("checked just above")),+
                    ));
                }
                // No waker is rung, for the reason `interleave` gives: what a
                // branch waits for is the I/O, and the executor is the only
                // thing on this thread that parks.
                std::task::Poll::Pending
            })
            .await
        }
    };
}

/// **A block's one outcome, out of its branches'**
/// ([ADR-292](../../../docs/specification/adr/adr-292.md) D5,
/// [ADR-292](../../../docs/specification/adr/adr-292.md) D18).
///
/// The arguments are the branches' results **in written order**, which is the
/// only order the source has and therefore the only one that makes the outcome
/// reproducible. `?` left to right is that rule, written as the language
/// below's own control flow rather than as a comparison this makes.
///
/// **Why it is a function and not a `?` per branch at the call.** An
/// `overlap { … } catch { … }` hands the block's outcome to a handler, and a
/// handler needs the `Result` rather than the value - a `?` in the middle of
/// the block would leave the function instead. One place that turns *n*
/// outcomes into one is also the place [ADR-292](../../../docs/specification/adr/adr-292.md)
/// puts the `secondary` list the day the later failures stop being dropped:
/// this function is the only code that sees them all.
macro_rules! combining {
    ($name:ident, $($branch:ident : $value:ident),+) => {
        #[allow(non_snake_case, clippy::too_many_arguments)]
        pub fn $name<$($value),+, E: crate::error::Joined>(
            $($branch: Result<$value, E>),+
        ) -> Result<($($value),+), E> {
            // **Every failure, and the first in written order is the block's**
            // ([ADR-292](../../../docs/specification/adr/adr-292.md) D5,
            // [ADR-292](../../../docs/specification/adr/adr-292.md) D9). The
            // block waited for every branch, so when this runs every outcome is
            // known and the list is a fact rather than a race - which is the
            // reason D2 gives for not cancelling the rest at the first failure.
            let mut failed: Vec<E> = Vec::new();
            $(
                let $branch = match $branch {
                    Ok(value) => Some(value),
                    Err(e) => {
                        failed.push(e);
                        None
                    }
                };
            )+
            let mut failed = failed.into_iter();
            if let Some(mut first) = failed.next() {
                for later in failed {
                    first.joined_by(later);
                }
                return Err(first);
            }
            Ok(($($branch.expect("no branch failed")),+))
        }
    };
}

combining!(combine2, A: RA, B: RB);
combining!(combine3, A: RA, B: RB, C: RC);
combining!(combine4, A: RA, B: RB, C: RC, D: RD);
combining!(combine5, A: RA, B: RB, C: RC, D: RD, E2: RE);
combining!(combine6, A: RA, B: RB, C: RC, D: RD, E2: RE, F: RF);
combining!(combine7, A: RA, B: RB, C: RC, D: RD, E2: RE, F: RF, G: RG);
combining!(combine8, A: RA, B: RB, C: RC, D: RD, E2: RE, F: RF, G: RG, H: RH);

overlapping!(overlap2, A: RA, B: RB);
overlapping!(overlap3, A: RA, B: RB, C: RC);
overlapping!(overlap4, A: RA, B: RB, C: RC, D: RD);
overlapping!(overlap5, A: RA, B: RB, C: RC, D: RD, E: RE);
overlapping!(overlap6, A: RA, B: RB, C: RC, D: RD, E: RE, F: RF);
overlapping!(overlap7, A: RA, B: RB, C: RC, D: RD, E: RE, F: RF, G: RG);
overlapping!(overlap8, A: RA, B: RB, C: RC, D: RD, E: RE, F: RF, G: RG, H: RH);

/// **Which arm of a `select { … }` won** (Part II 12.4,
/// [ADR-292](../../../docs/specification/adr/adr-292.md) D12).
///
/// One enum per arity, for the reason `overlap` has one function per arity: the
/// arms have different types, and in the language below that is what a sum of
/// them means. The variants are **ordinals** and not letters, so the generated
/// `match` reads as the source does — `Race2::Second(_)` is the second arm of
/// the block (Part III C.1).
///
/// **The losers are dropped**, and that is D2 rather than an implementation
/// detail: a future dropped at a suspension point tears its values down, a
/// `cleanup` that pauses is adopted by the runtime and bounded by the
/// `cleanup-deadline` ([ADR-297](../../../docs/specification/adr/adr-297.md)
/// D3), and the winner does not wait for any of it. The language below drops
/// the losing futures when `race<n>` returns, so the mechanism is the one this
/// runtime already had.
macro_rules! racing {
    ($name:ident, $won:ident, $($branch:ident : $result:ident : $variant:ident),+) => {
        #[derive(Debug)]
        pub enum $won<$($result),+> {
            $($variant($result)),+
        }

        // One parameter per arm is what an arity *is*, the same way an
        // `overlap`'s is.
        #[allow(non_snake_case, clippy::too_many_arguments)]
        pub async fn $name<$($branch, $result),+>($($branch: $branch),+) -> $won<$($result),+>
        where
            $($branch: std::future::Future<Output = $result>),+
        {
            $(let mut $branch = Box::pin($branch);)+

            std::future::poll_fn(move |context| {
                let _open = OpenBlock::enter();
                // **In written order, and the first that is ready wins.** Two
                // arms ready in the same pass is a tie, and the written order
                // is what breaks it - which is the same rule
                // [ADR-292](../../../docs/specification/adr/adr-292.md) D5 uses
                // for an `overlap`'s failures, said about a value instead.
                $(
                    if let std::task::Poll::Ready(value) = $branch.as_mut().poll(context) {
                        return std::task::Poll::Ready($won::$variant(value));
                    }
                )+
                // No waker is rung, for the reason `overlap` gives: what a
                // branch waits for is the I/O or the clock, and the executor is
                // the only thing on this thread that parks.
                std::task::Poll::Pending
            })
            .await
        }
    };
}

racing!(race2, Race2, A: RA: First, B: RB: Second);
racing!(race3, Race3, A: RA: First, B: RB: Second, C: RC: Third);
racing!(race4, Race4, A: RA: First, B: RB: Second, C: RC: Third, D: RD: Fourth);
racing!(
    race5, Race5, A: RA: First, B: RB: Second, C: RC: Third, D: RD: Fourth, E: RE: Fifth
);
racing!(
    race6, Race6, A: RA: First, B: RB: Second, C: RC: Third, D: RD: Fourth, E: RE: Fifth,
    F: RF: Sixth
);
racing!(
    race7, Race7, A: RA: First, B: RB: Second, C: RC: Third, D: RD: Fourth, E: RE: Fifth,
    F: RF: Sixth, G: RG: Seventh
);
racing!(
    race8, Race8, A: RA: First, B: RB: Second, C: RC: Third, D: RD: Fourth, E: RE: Fifth,
    F: RF: Sixth, G: RG: Seventh, H: RH: Eighth
);

/// One half of a [`crate::fs::read_both`], finished as `fs::read_to_string`
/// would have finished it.
///
/// `fs::read_to_string` is a read and then a UTF-8 check, and the pair performs
/// the read - so this is the check, on exactly the bytes that came back,
/// reporting exactly the failure the sequential program would have reported. It
/// is `std`'s own function and not a line written twice into every program that
/// reads two files at once.
///
/// **`what` is the path**, because since
/// [ADR-280](../../../docs/specification/adr/adr-280.md) D5 the failure names
/// what it was about, and the pair's own read does not carry it. Without it
/// the claim above stops being true: `read_to_string` would say which file and
/// this would not.
pub fn as_text(
    bytes: Result<Vec<u8>, std::io::Error>,
    what: &str,
) -> Result<String, crate::io::IoError> {
    match bytes {
        Ok(bytes) => crate::fs::text(bytes, what),
        Err(error) => Err(crate::io::IoError::of(error, what)),
    }
}

/// **What a scope's task hands back** ([ADR-346](../../../docs/specification/adr/adr-346.md)
/// D7): nothing, or its body's error in the scope's channel `E`.
pub trait Outcome<E> {
    /// The body's error, if it ended with one.
    fn failed(self) -> Option<E>;
}

impl<E> Outcome<E> for () {
    fn failed(self) -> Option<E> {
        None
    }
}

impl<E> Outcome<E> for Result<(), E> {
    fn failed(self) -> Option<E> {
        self.err()
    }
}

/// **A task of a scope that cannot fail**, as the emitter writes it: the bound
/// says it hands back nothing, so a body that only panics is one too.
pub fn plain<F: FnOnce()>(body: F) -> F {
    body
}

/// [`plain`] for a task that pauses.
pub fn plain_paused<F: FnOnce() -> P, P: std::future::Future<Output = ()>>(body: F) -> F {
    body
}

/// **A scope's channel where its tasks run on the pool**: what crosses a thread
/// on the way to the scope's end. A typed channel crosses as itself; the box
/// crosses as [`Carried`], which keeps a registered type as itself.
pub trait Crosses: Sized {
    type Sent: Send;
    fn sent(self) -> Self::Sent;
    fn back(sent: Self::Sent) -> Self;
}

impl Crosses for Crashed {
    type Sent = Crashed;
    fn sent(self) -> Crashed {
        self
    }
    fn back(sent: Crashed) -> Self {
        sent
    }
}

impl Crosses for Box<dyn std::error::Error> {
    type Sent = Carried;
    fn sent(self) -> Carried {
        Carried::from_box(self)
    }
    fn back(sent: Carried) -> Self {
        sent.into_box()
    }
}

/// **The box as a scope's channel** (ADR-346 D7), where a `catch` sits on the
/// scope: its tasks fail in `Box<dyn Error>`, the scope in this. The box's
/// object lifetime is inferred inside a function, and a `for<'scope>` closure
/// that names it asks everything its tasks borrow to be `'static` (E0373); a
/// type with no lifetime of its own lets them borrow, as a typed channel does.
#[derive(Debug)]
pub struct Boxed(pub Box<dyn std::error::Error>);

impl From<Crashed> for Boxed {
    fn from(crash: Crashed) -> Boxed {
        Boxed(Box::new(crash))
    }
}

impl From<Boxed> for Box<dyn std::error::Error> {
    fn from(boxed: Boxed) -> Box<dyn std::error::Error> {
        boxed.0
    }
}

impl crate::error::Joined for Boxed {
    fn joined_by(&mut self, later: Boxed) {
        crate::error::Joined::joined_by(&mut self.0, later.0)
    }
}

impl Outcome<Boxed> for Result<(), Box<dyn std::error::Error>> {
    fn failed(self) -> Option<Boxed> {
        self.err().map(Boxed)
    }
}

impl Crosses for Boxed {
    type Sent = Carried;
    fn sent(self) -> Carried {
        Carried::from_box(self.0)
    }
    fn back(sent: Carried) -> Self {
        Boxed(sent.into_box())
    }
}

impl<E> Crosses for crate::error::Thrown<E>
where
    crate::error::Thrown<E>: Send,
{
    type Sent = Self;
    fn sent(self) -> Self {
        self
    }
    fn back(sent: Self) -> Self {
        sent
    }
}

/// A crash has nothing to join: the first one is what the scope throws.
impl crate::error::Joined for Crashed {
    fn joined_by(&mut self, _later: Crashed) {}
}

/// **How a scope ends** (D7, as `overlap` does, ADR-292 D5): the first failure
/// in start order is thrown, every later one joins it in that order.
/// **The scope's own function failed** (ADR-346 D7): its error is the scope's
/// like a task's, in the place after every task it started, so a task's
/// earlier failure comes first and the function's joins it.
fn body_failed<E, X>(ended: Result<(), X>, mut record: impl FnMut(E)) -> bool
where
    Result<(), X>: Outcome<E>,
{
    match ended.failed() {
        Some(error) => {
            record(error);
            true
        }
        None => false,
    }
}

/// What a scope whose function may throw hands back once it has ended: the
/// function's value, which is there whenever nothing failed.
pub fn valued<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => unreachable!("a scope whose function failed throws"),
    }
}

/// The function's value, or its error kept as the scope's.
fn settled_with<T, X, E>(ended: Result<T, X>, record: impl FnMut(E)) -> Option<T>
where
    Result<(), X>: Outcome<E>,
{
    match ended {
        Ok(value) => Some(value),
        Err(error) => {
            body_failed(Err(error), record);
            None
        }
    }
}

impl<'scope, E: From<Crashed>> Scope<'scope, '_, E> {
    /// The scope's function ended: its value, or its error kept as the scope's.
    pub fn settled<T, X>(&self, ended: Result<T, X>) -> Option<T>
    where
        Result<(), X>: Outcome<E>,
    {
        settled_with(ended, |error| {
            let mut seen = self.seen.borrow_mut();
            let at = seen.started;
            seen.failed.push((at, error));
        })
    }
}

impl<'scope, E: Crosses + From<Crashed>> PoolScope<'_, 'scope, E> {
    /// [`Scope::settled`] where tasks run on the pool.
    pub fn settled<T, X>(&self, ended: Result<T, X>) -> Option<T>
    where
        Result<(), X>: Outcome<E>,
    {
        settled_with(ended, |error: E| {
            let mut seen = locked(&self.seen);
            let at = seen.started;
            seen.failed.push((at, error.sent()));
        })
    }
}

impl<'scope, E: From<Crashed> + 'scope> PausedScope<'scope, E> {
    /// [`Scope::settled`] for a scope whose tasks may pause.
    pub fn settled<T, X>(&self, ended: Result<T, X>) -> Option<T>
    where
        Result<(), X>: Outcome<E>,
    {
        settled_with(ended, |error| {
            let mut seen = locked(&self.seen);
            let at = seen.started;
            seen.failed.push((at, error));
        })
    }
}

impl<'scope, E: Crosses + From<Crashed> + 'scope> PausedPoolScope<'scope, E> {
    /// [`Scope::settled`] for a scope whose tasks may pause, on the pool.
    pub fn settled<T, X>(&self, ended: Result<T, X>) -> Option<T>
    where
        Result<(), X>: Outcome<E>,
    {
        settled_with(ended, |error: E| {
            let mut seen = locked(&self.seen);
            let at = seen.started;
            seen.failed.push((at, error.sent()));
        })
    }
}

fn ended<T, E: crate::error::Joined>(value: T, mut failed: Vec<(usize, E)>) -> Result<T, E> {
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

/// What a scope has seen of its tasks: how many started, whether one crashed,
/// and the failures, each with its place in start order.
struct Seen<E> {
    started: usize,
    crashed: bool,
    failed: Vec<(usize, E)>,
}

impl<E> Seen<E> {
    fn new() -> Self {
        Seen {
            started: 0,
            crashed: false,
            failed: Vec::new(),
        }
    }

    /// The next task's place, or `None` where a crash cancelled it.
    fn start(&mut self) -> Option<usize> {
        if self.crashed {
            return None;
        }
        self.started += 1;
        Some(self.started - 1)
    }
}

impl<E: From<Crashed>> Seen<E> {
    fn crash(&mut self, at: usize, crash: Crashed) {
        self.crashed = true;
        self.failed.push((at, E::from(crash)));
    }
}

/// One task of a scope run where it starts: its crash or its error recorded.
fn ran<E: From<Crashed>, R: Outcome<E>>(seen: &mut Seen<E>, at: usize, body: impl FnOnce() -> R) {
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

/// **A scope's tasks** (Part II 12.7, ADR-328 D8): they borrow what the
/// function around the scope holds, and the scope waits for every one of them.
///
/// At `user_parallelism = no` everything runs on one thread, so a task that
/// never pauses runs where it is started, to its end; a scope with a task that
/// pauses is a [`PausedScope`] (ADR-328 D10). A task that panicked
/// cancels the ones not yet started, and the scope throws [`Crashed`] where it
/// ends. A task whose body fails hands its error on in the scope's channel `E`
/// (ADR-346 D7). `'env` is what the tasks borrow, which outlives the scope, as
/// `std::thread::scope` has it.
pub struct Scope<'scope, 'env: 'scope, E = Crashed> {
    seen: std::cell::RefCell<Seen<E>>,
    scope: std::marker::PhantomData<&'scope mut &'scope ()>,
    env: std::marker::PhantomData<&'env mut &'env ()>,
}

impl<'scope, E: From<Crashed>> Scope<'scope, '_, E> {
    /// Start a task that never pauses: it runs now, unless a task before it
    /// crashed, which cancelled it.
    pub fn spawn<R: Outcome<E>>(&self, body: impl FnOnce() -> R + 'scope) {
        let Some(at) = self.seen.borrow_mut().start() else {
            return;
        };
        let mut seen = Seen::new();
        ran(&mut seen, at, body);
        let mut held = self.seen.borrow_mut();
        held.crashed |= seen.crashed;
        held.failed.append(&mut seen.failed);
    }
}

/// **Run `body` with a scope, and wait for its tasks** (Part II 12.7): the
/// first failure in start order is what this throws, after `body` and every
/// task have ended; the later ones join it.
pub fn scope<'env, T, E: From<Crashed> + crate::error::Joined>(
    body: impl for<'scope> FnOnce(&'scope Scope<'scope, 'env, E>) -> T,
) -> Result<T, E> {
    let scope = Scope {
        seen: std::cell::RefCell::new(Seen::new()),
        scope: std::marker::PhantomData,
        env: std::marker::PhantomData,
    };
    let value = body(&scope);
    let failed = std::mem::take(&mut scope.seen.borrow_mut().failed);
    ended(value, failed)
}

/// **The same scope where tasks run on every core** (`user_parallelism = yes`):
/// each task is the pool's, and the scope waits for all of them. A task here
/// never pauses - a scope with one that does is a [`PausedPoolScope`] - so
/// one that started runs to its end; a crash cancels only the tasks not yet
/// started.
pub struct PoolScope<'pool, 'scope, E: Crosses = Crashed> {
    pool: &'pool rayon::Scope<'scope>,
    seen: std::sync::Arc<std::sync::Mutex<Seen<E::Sent>>>,
}

impl<'scope, E: Crosses + From<Crashed>> PoolScope<'_, 'scope, E>
where
    E::Sent: 'scope,
{
    /// Start a task on the pool.
    pub fn spawn<R: Outcome<E>>(&self, body: impl FnOnce() -> R + Send + 'scope) {
        let seen = self.seen.clone();
        let Some(at) = locked(&seen).start() else {
            return;
        };
        self.pool.spawn(move |_| {
            if locked(&seen).crashed {
                return;
            }
            let mut own = Seen::new();
            ran::<E, R>(&mut own, at, body);
            let mut held = locked(&seen);
            held.crashed |= own.crashed;
            for (at, error) in own.failed {
                held.failed.push((at, error.sent()));
            }
        });
    }
}

/// [`scope`] at `user_parallelism = yes`.
pub fn scope_on_pool<'scope, T: Send, E: Crosses + From<Crashed> + crate::error::Joined>(
    body: impl FnOnce(&PoolScope<'_, 'scope, E>) -> T + Send,
) -> Result<T, E>
where
    E::Sent: 'scope,
{
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Seen::new()));
    let value = rayon::scope(|pool| {
        body(&PoolScope {
            pool,
            seen: seen.clone(),
        })
    });
    let failed: Vec<(usize, E)> = std::mem::take(&mut locked(&seen).failed)
        .into_iter()
        .map(|(at, sent)| (at, E::back(sent)))
        .collect();
    ended(value, failed)
}

/// A task of a scope that pauses: a future the scope holds and polls.
type Paused<'scope> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'scope>>;

/// The same, where the scope's own future crosses threads (`user_parallelism = yes`).
type PausedSend<'scope> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'scope>>;

/// A task of a scope at `user_parallelism = yes` that never pauses, waiting for the pool.
type Plain<'scope> = Box<dyn FnOnce() + Send + 'scope>;

/// **A scope with tasks that pause** ([ADR-328](../../../docs/specification/adr/adr-328.md)
/// D10): a task that never pauses runs where it is started, as in [`Scope`];
/// one that pauses lives in the scope's own future and is polled there, never
/// handed to an executor, so it may borrow what the scope's function holds,
/// and dropping the scope drops it.
///
/// The tasks started since the scope last looked wait in `arrived`, and every
/// poll moves them to the scope's own list before it returns, so the queue is
/// empty whenever the scope is not being polled. It is `ManuallyDrop` for the
/// borrow checker's sake: a task may borrow the scope, and the queue, empty,
/// needs no destructor that could see one.
pub struct PausedScope<'scope, E = Crashed> {
    seen: std::sync::Mutex<Seen<E>>,
    arrived: std::mem::ManuallyDrop<std::sync::Mutex<Vec<Paused<'scope>>>>,
}

impl<E> Default for PausedScope<'_, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'scope, E> PausedScope<'scope, E> {
    /// A scope, which the call that runs it borrows for as long as its tasks
    /// may: the caller makes it, so `'scope` is a lifetime of the caller's and
    /// not one the scope's function has to be general over.
    pub fn new() -> Self {
        PausedScope {
            seen: std::sync::Mutex::new(Seen::new()),
            arrived: std::mem::ManuallyDrop::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}

impl<'scope, E: From<Crashed> + 'scope> PausedScope<'scope, E> {
    /// Start a task that never pauses: it runs now, unless a task crashed.
    pub fn spawn<R: Outcome<E>>(&self, body: impl FnOnce() -> R + 'scope) {
        let Some(at) = locked(&self.seen).start() else {
            return;
        };
        let mut own = Seen::new();
        ran(&mut own, at, body);
        let mut held = locked(&self.seen);
        held.crashed |= own.crashed;
        held.failed.append(&mut own.failed);
    }

    /// Start a task that pauses: the scope polls it beside the others.
    /// `async` only so that a call that starts one reads as the pausing call
    /// it is; it hands the task over and returns.
    pub async fn spawn_paused<F, R: Outcome<E>>(&'scope self, body: impl FnOnce() -> F + 'scope)
    where
        F: std::future::Future<Output = R> + 'scope,
    {
        let Some(at) = locked(&self.seen).start() else {
            return;
        };
        locked(&self.arrived).push(Box::pin(async move {
            if let Some(error) = body().await.failed() {
                locked(&self.seen).failed.push((at, error));
            }
        }));
    }
}

/// **Run `body` with a scope whose tasks may pause, and wait for all of them**
/// (Part II 12.7, ADR-328 D10). The scope's function and every task are polled
/// together, so tasks that wait on each other complete. A task that panicked
/// drops the ones still running, so their cleanups run, and cancels the ones
/// not started; the first failure in start order is thrown once the function
/// has ended (ADR-346 D7).
///
/// `body` is the scope's function as a future that names `scope` itself, not
/// a closure handed it: a closure taking the scope by reference is a
/// higher-ranked one, and `rustc` cannot then show the future `Send`.
pub async fn scope_paused<'scope, T, E: From<Crashed> + crate::error::Joined>(
    scope: &'scope PausedScope<'scope, E>,
    body: impl std::future::Future<Output = T> + 'scope,
) -> Result<T, E> {
    let value = polled(
        body,
        || std::mem::take(&mut *locked(&scope.arrived)),
        |crash| {
            let mut seen = locked(&scope.seen);
            let at = seen.started;
            seen.crash(at, crash);
        },
    )
    .await;
    let failed = std::mem::take(&mut locked(&scope.seen).failed);
    ended(value, failed)
}

/// **[`PausedScope`] where tasks that never pause run on every core**
/// (`user_parallelism = yes`): those run on the pool, all of them, once the
/// scope's function has ended; the tasks that pause take turns on the scope's
/// own thread, beside each other - concurrently, not in parallel (D10).
///
/// `arrived` and `plain` are `ManuallyDrop` as [`PausedScope`]'s queue is, and
/// empty by the time the scope ends.
pub struct PausedPoolScope<'scope, E: Crosses = Crashed> {
    seen: std::sync::Mutex<Seen<E::Sent>>,
    arrived: std::mem::ManuallyDrop<std::sync::Mutex<Vec<PausedSend<'scope>>>>,
    plain: std::mem::ManuallyDrop<std::sync::Mutex<Vec<Plain<'scope>>>>,
}

impl<E: Crosses> Default for PausedPoolScope<'_, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Crosses> PausedPoolScope<'_, E> {
    /// A scope, made by the caller as [`PausedScope::new`] is.
    pub fn new() -> Self {
        PausedPoolScope {
            seen: std::sync::Mutex::new(Seen::new()),
            arrived: std::mem::ManuallyDrop::new(std::sync::Mutex::new(Vec::new())),
            plain: std::mem::ManuallyDrop::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}

impl<'scope, E: Crosses + From<Crashed> + 'scope> PausedPoolScope<'scope, E>
where
    E::Sent: 'scope,
{
    /// Start a task that never pauses, on the pool.
    pub fn spawn<R: Outcome<E>>(&'scope self, body: impl FnOnce() -> R + Send + 'scope) {
        let Some(at) = locked(&self.seen).start() else {
            return;
        };
        let seen = &self.seen;
        locked(&self.plain).push(Box::new(move || {
            if locked(seen).crashed {
                return;
            }
            let mut own = Seen::new();
            ran::<E, R>(&mut own, at, body);
            let mut held = locked(seen);
            held.crashed |= own.crashed;
            for (at, error) in own.failed {
                held.failed.push((at, error.sent()));
            }
        }));
    }

    /// Start a task that pauses: the scope polls it beside the others.
    pub async fn spawn_paused<F, R: Outcome<E>>(
        &'scope self,
        body: impl FnOnce() -> F + Send + 'scope,
    ) where
        F: std::future::Future<Output = R> + Send + 'scope,
    {
        let Some(at) = locked(&self.seen).start() else {
            return;
        };
        let seen = &self.seen;
        locked(&self.arrived).push(Box::pin(async move {
            if let Some(error) = body().await.failed() {
                locked(seen).failed.push((at, error.sent()));
            }
        }));
    }
}

/// [`scope_paused`] at `user_parallelism = yes`.
pub async fn scope_paused_on_pool<'scope, T, E: Crosses + From<Crashed> + crate::error::Joined>(
    scope: &'scope PausedPoolScope<'scope, E>,
    body: impl std::future::Future<Output = T> + 'scope,
) -> Result<T, E> {
    let value = polled(
        body,
        || std::mem::take(&mut *locked(&scope.arrived)),
        |crash| {
            let mut seen = locked(&scope.seen);
            let at = seen.started;
            seen.crashed = true;
            seen.failed.push((at, E::from(crash).sent()));
        },
    )
    .await;
    let plain = std::mem::take(&mut *locked(&scope.plain));
    if !plain.is_empty() && !locked(&scope.seen).crashed {
        rayon::scope(|pool| {
            for body in plain {
                pool.spawn(move |_| body());
            }
        });
    }
    let failed: Vec<(usize, E)> = std::mem::take(&mut locked(&scope.seen).failed)
        .into_iter()
        .map(|(at, sent)| (at, E::back(sent)))
        .collect();
    ended(value, failed)
}

/// A scope's lock: a task that panicked holding it changed nothing it guards.
fn locked<T>(held: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    held.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// **The scope's function and its pausing tasks, polled together** (D10):
/// `arrived` hands over the tasks started since it was last asked, and a task
/// that panicked is `crashed`, after which every running task is dropped.
async fn polled<T, F: std::future::Future<Output = ()> + ?Sized>(
    body: impl std::future::Future<Output = T>,
    arrived: impl Fn() -> Vec<std::pin::Pin<Box<F>>>,
    crashed: impl Fn(Crashed),
) -> T {
    let mut body = std::pin::pin!(body);
    let mut value = None;
    let mut running: Vec<std::pin::Pin<Box<F>>> = Vec::new();
    std::future::poll_fn(|cx| {
        if value.is_none()
            && let std::task::Poll::Ready(done) = body.as_mut().poll(cx)
        {
            value = Some(done);
        }
        running.extend(arrived());
        let mut at = 0;
        while at < running.len() {
            let _inside = InATask::enter();
            let task = &mut running[at];
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task.as_mut().poll(cx)))
            {
                Ok(std::task::Poll::Ready(())) => drop(running.swap_remove(at)),
                Ok(std::task::Poll::Pending) => at += 1,
                Err(payload) => {
                    crashed(Crashed {
                        message: panic_message(payload.as_ref()),
                        site: SITE.with(|held| std::mem::take(&mut *held.borrow_mut())),
                    });
                    // Dropped, so each one's cleanup runs (D8).
                    running.clear();
                    drop(arrived());
                    break;
                }
            }
            running.extend(arrived());
        }
        match (value.is_some(), running.is_empty()) {
            (true, true) => std::task::Poll::Ready(()),
            _ => std::task::Poll::Pending,
        }
    })
    .await;
    value.expect("the scope's function ended")
}

/// One task of a scope, its panic caught at its edge as a task's is (ADR-326 D4).
fn caught(body: impl FnOnce()) -> Result<(), Crashed> {
    let _inside = InATask::enter();
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)).map_err(|payload| Crashed {
        message: panic_message(payload.as_ref()),
        site: SITE.with(|held| std::mem::take(&mut *held.borrow_mut())),
    })
}

#[cfg(test)]
mod scoped {
    use super::*;

    #[test]
    fn a_scope_lends_to_its_tasks_and_waits_for_them() {
        let data = [1, 2, 3];
        let sum = std::cell::Cell::new(0);
        let made = scope::<_, Crashed>(|s| {
            s.spawn(|| sum.set(sum.get() + data.iter().sum::<i32>()));
            s.spawn(|| sum.set(sum.get() + data.len() as i32));
            7
        });
        assert_eq!((made, sum.get(), data.len()), (Ok(7), 9, 3));
    }

    #[test]
    fn a_crash_cancels_the_tasks_after_it_and_is_thrown_at_the_end() {
        let ran = std::cell::Cell::new(0);
        let made = scope::<_, Crashed>(|s| {
            s.spawn(|| ran.set(ran.get() + 1));
            s.spawn::<()>(|| panic!("boom"));
            s.spawn(|| ran.set(ran.get() + 10));
        });
        assert_eq!(made.map_err(|crash| crash.message), Err("boom".to_string()));
        assert_eq!(ran.get(), 1);
    }

    #[test]
    fn a_pool_scope_runs_its_tasks_on_the_pool_and_waits() {
        let data: Vec<i64> = (1..=100).collect();
        let total = std::sync::atomic::AtomicI64::new(0);
        let made = scope_on_pool::<_, Crashed>(|s| {
            for half in data.chunks(50) {
                let total = &total;
                s.spawn(move || {
                    total.fetch_add(half.iter().sum(), std::sync::atomic::Ordering::SeqCst);
                });
            }
        });
        assert!(made.is_ok());
        assert_eq!(total.into_inner(), 5050);
        let crashed = scope_on_pool::<_, Crashed>(|s| s.spawn::<()>(|| panic!("on the pool")));
        assert_eq!(
            crashed.map_err(|crash| crash.message),
            Err("on the pool".to_string())
        );
    }
    /// Polls `future` to its end on this thread; every pause below wakes
    /// itself, so a waker that does nothing is enough.
    fn run<T>(future: impl std::future::Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        loop {
            if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut cx) {
                return value;
            }
        }
    }

    /// One pause: the task gives its turn up once.
    async fn pause() {
        let mut paused = false;
        std::future::poll_fn(|cx| match std::mem::replace(&mut paused, true) {
            true => std::task::Poll::Ready(()),
            false => {
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
        })
        .await
    }

    /// Waits until `flag` is set, pausing in between.
    async fn until(flag: &std::cell::Cell<bool>) {
        while !flag.get() {
            pause().await;
        }
    }

    /// A value whose drop is counted: a task's cleanup.
    struct Cleaned<'a>(&'a std::cell::Cell<i32>);

    impl Drop for Cleaned<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    /// **A task that pauses borrows, and the scope waits for it** (ADR-328
    /// D10), beside one that does not.
    #[test]
    fn a_paused_scope_lends_to_its_pausing_tasks_and_waits() {
        let config = String::from("abc");
        let seen = std::cell::RefCell::new(Vec::new());
        let made = run(async {
            let s = &PausedScope::<Crashed>::new();
            scope_paused(s, async {
                s.spawn(|| seen.borrow_mut().push(0));
                s.spawn_paused(async || {
                    pause().await;
                    seen.borrow_mut().push(config.len());
                })
                .await;
                5
            })
            .await
        });
        assert_eq!(made.ok(), Some(5));
        assert_eq!(*seen.borrow(), [0, 3]);
    }

    /// **The tasks are polled together, not one after the other** (D10): two
    /// that each wait for what the other does complete.
    #[test]
    fn pausing_tasks_that_wait_on_each_other_complete() {
        let (a, b) = (std::cell::Cell::new(false), std::cell::Cell::new(false));
        let made = run(async {
            let s = &PausedScope::<Crashed>::new();
            scope_paused(s, async {
                s.spawn_paused(async || {
                    until(&b).await;
                    a.set(true);
                })
                .await;
                s.spawn_paused(async || {
                    b.set(true);
                    until(&a).await;
                })
                .await;
            })
            .await
        });
        assert!(made.is_ok());
        assert!(a.get() && b.get());
    }

    /// **A crash drops the running tasks, so their cleanups run** (D8, D10),
    /// and the scope throws it where it ends.
    #[test]
    fn a_crash_after_a_pause_drops_the_others_with_their_cleanups() {
        let cleaned = std::cell::Cell::new(0);
        let finished = std::cell::Cell::new(0);
        let made = run(async {
            let s = &PausedScope::<Crashed>::new();
            scope_paused(s, async {
                for _ in 0..2 {
                    s.spawn_paused::<_, ()>(async || {
                        let _held = Cleaned(&cleaned);
                        loop {
                            pause().await;
                        }
                    })
                    .await;
                }
                s.spawn_paused::<_, ()>(async || {
                    pause().await;
                    panic!("after a pause");
                })
                .await;
                s.spawn(|| finished.set(1));
            })
            .await
        });
        assert_eq!(
            made.map_err(|crash| crash.message),
            Err("after a pause".to_string())
        );
        assert_eq!((cleaned.get(), finished.get()), (2, 1));
    }

    /// **At `yes`** the tasks that do not pause run on the pool, and the scope's
    /// future crosses threads.
    #[test]
    fn a_paused_pool_scope_runs_both_kinds() {
        fn crosses<T: Send>(value: T) -> T {
            value
        }
        let config = String::from("abcd");
        let total = std::sync::atomic::AtomicUsize::new(0);
        let made = run(crosses(async {
            let s = &PausedPoolScope::<Crashed>::new();
            scope_paused_on_pool(s, async {
                s.spawn(|| {
                    total.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                });
                s.spawn_paused(async || {
                    total.fetch_add(config.len(), std::sync::atomic::Ordering::SeqCst);
                })
                .await;
            })
            .await
        }));
        assert!(made.is_ok());
        assert_eq!(total.into_inner(), 5);
    }
}
