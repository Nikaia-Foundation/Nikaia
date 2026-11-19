//! What `Locked[T]` is in the machine — two shapes behind one surface
//! ([ADR-281](../../../docs/specification/adr/adr-281.md)).
//!
//! **The shape is decided per value and not per build setting** (D1–D3): where
//! `contracts::sharing` proves that nothing crosses a thread with a value, its
//! lock is [`Local`]; where it does not, and at every value that does cross, it
//! is [`Crossing`]. The floor is the one that is safe, exactly as the reference
//! count's floor is the atomic one.
//!
//! **Both say the same thing when a program re-enters one lock** (D4), and that
//! is what the crossing shape pays its owner check for. A borrow flag says so by
//! itself; a bare `Mutex` would hang for ever without saying anything, and at one
//! user thread a hang is the whole program plus
//! [ADR-297](../../../docs/specification/adr/adr-297.md) D6's shutdown deadline.
//! Two behaviours for one written type, picked by an analysis the reader cannot
//! see, is the thing D4 refuses.
//!
//! **Neither is a pause.** `access` takes a lambda that runs straight through
//! ([ADR-005](../../../docs/specification/adr/adr-005.md) D6), so nothing is held
//! across a suspension point and a wait here is bounded by somebody else's
//! compute rather than by somebody else's waiting.

use std::cell::RefCell;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// The one message, so re-entering reads the same whichever shape a value got.
///
/// **There used to be a second one beside this**, `emptied`, for a lock whose
/// `update` had failed and left nothing behind. It is gone with the state it
/// named: since [ADR-281](../../../docs/specification/adr/adr-281.md) D12 the
/// block is handed the **address** in the lock, so nothing is ever moved out
/// and no slot is ever empty. A panic in the block leaves the value changed as
/// far as the block got, which is D4 rather than a state to report.
///
/// `#[track_caller]` out to the `access` that called it, so the location the
/// panic hook is handed is the generated line that wrote the access and not a
/// line of this file — without it
/// [ADR-300](../../../docs/specification/adr/adr-300.md) D9's table has nothing
/// to look up.
#[cold]
#[inline(never)]
#[track_caller]
fn reentered() -> ! {
    panic!("this lock is already held by the same task: `access` cannot be re-entered")
}

/// **A door to a value a panicking task held** ([ADR-327](../../../docs/specification/adr/adr-327.md)
/// D1, D2): what it holds may be changed half-way, so it is not read. The
/// panic ends the task that opened the door (ADR-326 D4), as the first one's
/// ended the task that held it.
#[cold]
#[inline(never)]
#[track_caller]
fn poisoned() -> ! {
    panic!(
        "a task panicked while it held this lock, so the value may be half-changed: it is never read again"
    )
}

/// **How a supervised value comes back from its poison**
/// ([ADR-327](../../../docs/specification/adr/adr-327.md) D3,
/// [ADR-328](../../../docs/specification/adr/adr-328.md) D9): the restart
/// policy a supervisor's child has, asked by the door that finds the poison.
pub struct Phase {
    policy: Box<dyn crate::supervisor::Restart + Send>,
    /// When the value was last built, for how long the attempt ran.
    built: std::time::Instant,
    /// The history the policy decides from, as a supervisor keeps a child's:
    /// how many times it was poisoned, and when first.
    failures: i64,
    first: Option<std::time::Instant>,
    /// A delay the policy asked for: a door before it panics.
    waiting_until: Option<std::time::Instant>,
    /// The policy gave up: the value stays poisoned.
    given_up: bool,
}

impl std::fmt::Debug for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Phase")
            .field("failures", &self.failures)
            .field("given_up", &self.given_up)
            .finish_non_exhaustive()
    }
}

impl Phase {
    fn new(policy: impl crate::supervisor::Restart + Send + 'static) -> std::sync::Mutex<Phase> {
        std::sync::Mutex::new(Phase {
            policy: Box::new(policy),
            built: std::time::Instant::now(),
            failures: 0,
            first: None,
            waiting_until: None,
            given_up: false,
        })
    }

    /// Whether the door that found the poison rebuilds now; it panics where
    /// the policy waits or gave up.
    #[track_caller]
    fn recover(phase: &std::sync::Mutex<Phase>) {
        let mut phase = phase.lock().unwrap_or_else(|held| held.into_inner());
        if phase.given_up {
            gave_up();
        }
        let now = std::time::Instant::now();
        if phase.waiting_until.is_none() {
            phase.failures += 1;
            let history = crate::supervisor::Failures {
                last_uptime: now.duration_since(phase.built),
                consecutive: phase.failures,
                since_first: now.duration_since(*phase.first.get_or_insert(now)),
            };
            match phase.policy.decide(history) {
                crate::supervisor::Next::Immediate => {}
                crate::supervisor::Next::Delay(span) => phase.waiting_until = Some(now + span),
                crate::supervisor::Next::Escalate => {
                    phase.given_up = true;
                    gave_up();
                }
            }
        }
        if let Some(until) = phase.waiting_until {
            if now < until {
                being_rebuilt();
            }
            phase.waiting_until = None;
        }
        phase.built = now;
    }
}

/// A door to a supervised value during its policy's delay (ADR-328 D9).
#[cold]
#[inline(never)]
#[track_caller]
fn being_rebuilt() -> ! {
    panic!(
        "this value is being rebuilt after a task panicked while it held it: try again after the delay"
    )
}

/// A door to a supervised value whose policy gave up (ADR-328 D9).
#[cold]
#[inline(never)]
#[track_caller]
fn gave_up() -> ! {
    panic!("this value was rebuilt too often and its policy gave up: it stays poisoned")
}

/// The builder of a supervised value, which `Debug` cannot show.
pub struct Builder<F: ?Sized>(Box<F>);

impl<F: ?Sized> std::fmt::Debug for Builder<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Builder")
    }
}

/// **What a `set(neu; after: seen)` failed with**
/// ([ADR-281](../../../docs/specification/adr/adr-281.md) D26): the lock no
/// longer holds the value that was seen.
///
/// An error like any other — caught, declared, retried, or handed to the
/// caller, which in a server is the honest 409. It carries **nothing**: the
/// value that is in the lock now is not in it, because reading it would be a
/// second acquisition and a caller that wants it takes the door again and gets
/// a fresh stamp. What the name has to say is *somebody got there first*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overtaken;

impl std::fmt::Display for Overtaken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the lock no longer holds the value that was seen")
    }
}

impl std::error::Error for Overtaken {}

/// A lock for a value the analysis proved never crosses a thread.
///
/// A borrow flag, which is one non-atomic write and a branch. It is what every
/// lock is at `user_parallelism = no` (D2), because nothing a user writes can
/// cross there and the mutex would charge exactly what that switch exists to
/// save.
#[derive(Debug, Default)]
pub struct Local<T> {
    /// **The value itself, and no `Option` around it**
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D12).
    ///
    /// It used to be an `Option` so that `update` could move the value out and
    /// back without a `Default` bound and without unsafe code, and a lambda
    /// that failed left the slot `None` — a state every other door then had to
    /// report. D1 takes the cause away rather than the symptom: `update` is
    /// handed the **address**, so nothing is moved out and no slot is ever
    /// empty. What a panicking block leaves behind is D4's, and for this shape
    /// that is a value changed as far as the block got.
    ///
    /// **In a block of its own** (ADR-327 D5, #1536): what a door places in
    /// the block while it runs is the value's, not the task's that opened
    /// it, and a full one poisons the value (D6).
    inner: RefCell<crate::block::Housed<T>>,
    /// **Set when a block panicked while it held the value** (ADR-327 D1):
    /// every door after that panics rather than read it.
    poisoned: std::cell::Cell<bool>,
    /// How to build it again, for `SharedMut::supervised` (ADR-327 D3).
    #[allow(clippy::type_complexity)]
    rebuild: Option<(Builder<dyn Fn() -> T>, std::sync::Mutex<Phase>)>,
}

impl<T> Local<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(crate::block::Housed::new(value)),
            poisoned: std::cell::Cell::new(false),
            rebuild: None,
        }
    }

    /// **`SharedMut(value)` as the compiler writes it** (ADR-327 D9, #1751):
    /// the value's deep size, as `size` measures it, moves from the creator
    /// to the shared value at its creation, as `set` moves it, so what the
    /// value frees inside its block was counted there.
    pub fn weighed(value: T, size: impl FnOnce(&T) -> usize) -> Self {
        let size = size(&value);
        Self {
            inner: RefCell::new(crate::block::Housed::moved_in(value, size)),
            poisoned: std::cell::Cell::new(false),
            rebuild: None,
        }
    }

    /// `SharedMut::supervised(fn { … })` as the compiler writes it: the kept
    /// function a program hands over.
    pub fn supervised_kept<F: Fn() -> T + ?Sized + 'static>(
        build: crate::func::Kept<F>,
        policy: impl crate::supervisor::Restart + Send + 'static,
    ) -> Self {
        Self::supervised(move || (build.0)(), policy)
    }

    /// **A value that is built again when a panic poisons it**
    /// ([ADR-327](../../../docs/specification/adr/adr-327.md) D3,
    /// [ADR-328](../../../docs/specification/adr/adr-328.md) D9), when
    /// `policy` says so - as a supervisor's child is restarted.
    pub fn supervised(
        build: impl Fn() -> T + 'static,
        policy: impl crate::supervisor::Restart + Send + 'static,
    ) -> Self {
        Self {
            inner: RefCell::new(crate::block::Housed::built(&build)),
            poisoned: std::cell::Cell::new(false),
            rebuild: Some((Builder(Box::new(build)), Phase::new(policy))),
        }
    }

    /// The borrow, for a door: refused when the value is held already
    /// (re-entered) or was left by a panic (poisoned), rebuilt first where it
    /// is supervised.
    #[track_caller]
    #[inline]
    fn opened(&self) -> std::cell::RefMut<'_, crate::block::Housed<T>> {
        // **Re-entry first**: a door inside the block finds the value held,
        // and that is what it is told, not that the value is poisoned.
        let Ok(mut held) = self.inner.try_borrow_mut() else {
            reentered()
        };
        if self.poisoned.get() {
            match &self.rebuild {
                Some((build, phase)) => {
                    Phase::recover(phase);
                    held.inside(|value| *value = (build.0)());
                    self.poisoned.set(false);
                }
                None => poisoned(),
            }
        }
        held
    }

    /// The borrow, marked poisoned if the block run under it panics.
    #[track_caller]
    #[inline]
    fn held<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut held = self.opened();
        // **Set for as long as the block runs**, and cleared only when it
        // returns: a block that panics never returns, so the mark stays.
        // Counted with callgrind, a release `update` is 5 instructions with
        // the mark and without it; a guard asking `thread::panicking` on the
        // way out made it 26.
        self.poisoned.set(true);
        let result = held.inside(f);
        self.poisoned.set(false);
        result
    }

    /// **The borrow with no poison mark** (ADR-327 D11): for a block that
    /// cannot leave the value half-changed, because it only reads it or works
    /// on a copy. A panic in it releases the borrow and poisons nothing.
    /// The value's block is not in place: what the block builds leaves the
    /// door, so it counts against the task that opened it (D9, #1554).
    #[track_caller]
    #[inline]
    fn unmarked<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(self.opened().get_mut())
    }

    /// Part II 12.2's first door: a copy out. No code of the user's runs while
    /// the lock is open, which is why this needs no condition at all.
    #[track_caller]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.read(|value| value.clone())
    }

    /// …and the second: replacing. The new value is computed outside, so the
    /// lock is open for the duration of one store.
    ///
    /// **The value's deep size moves from the setter to the shared value**
    /// (ADR-327 D9): where it does not fit the value's block, the store
    /// panics with the block in place, and the value is poisoned (D6).
    #[track_caller]
    pub fn set(&self, value: T)
    where
        T: crate::block::DeepSize,
    {
        let size = crate::block::deep_size(&value);
        crate::block::hand_over(size);
        self.held(|held| {
            crate::block::received(size);
            *held = value
        })
    }

    /// **Reading in place** ([ADR-281](../../../docs/specification/adr/adr-281.md)
    /// D1): the lambda is handed the value where it lies and may not change it,
    /// so nothing is copied to answer a question about a large value.
    #[track_caller]
    pub fn access<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.read(f)
    }

    /// **Where locked data changes**
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D12): the block is
    /// handed the **address** in the lock, changes the value in place, and
    /// returns nothing.
    ///
    /// Nothing is moved out and no slot is ever empty, which is what took the
    /// `Option` away. D2's other row — a copy and a compare-and-swap, for a
    /// value that fits a machine word — is a **speed** choice on top of this
    /// one and is not built; the block runs exactly once here, which D3 allows
    /// (*may* run more than once is a licence, not a requirement).
    #[track_caller]
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.held(f)
    }

    /// **`update` of a value that copies** (ADR-327 D11): the block works on
    /// a copy, put in place at the block's end, so a panic in the block leaves
    /// the value as it was and poisons nothing. The compiler writes it where
    /// the value is a number, a `bool` or a `char`.
    #[track_caller]
    pub fn update_copied(&self, f: impl FnOnce(&mut T))
    where
        T: Copy,
    {
        self.unmarked(|held| on_a_copy(held, f))
    }

    /// **The one door for a stamped value**
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D26).
    ///
    /// `kasse.set(neu; after: stand)` is, by definition,
    /// `update fn(mut v) { if v == stand { v = neu } else { throw Overtaken } }`
    /// — and this is that block, written once. The witness is the value
    /// itself: *store `neu` if the lock still holds what I saw*. If it does,
    /// every decision taken on what was seen still holds; if it does not, the
    /// caller is told rather than overwriting somebody else's work.
    ///
    /// **The comparison is of the whole value**, which is what `PartialEq`
    /// buys and what makes this honest for a large one: a ten-thousand-entry
    /// list is compared entry by entry, and a program that minds writes the
    /// `update` block with a version field of its own
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D12).
    /// [ADR-281](../../../docs/specification/adr/adr-281.md) D13's
    /// compare-and-swap is the same operation for a word-sized value and is
    /// the speed row that is not built.
    ///
    /// **One lock acquisition and not two.** A `get` followed by a `set` is
    /// the mistake D4 refuses; the compare and the store happen while the lock
    /// is open, which is the whole of what this door is for.
    ///
    /// **The witness is a view and the value is not**, which is the difference
    /// between them: the value is stored and the witness is only read. A
    /// witness taken by value would be *moved* out of the caller, and reading
    /// what was seen after asking whether it still holds is an ordinary thing
    /// to write — so the ledger says `seen: &$T` and the caller keeps it.
    ///
    /// **The value's deep size moves as [`set`](Self::set)'s does** (ADR-327
    /// D9, #1554), and only where it is stored: an overtaken value stays
    /// the setter's, and is dropped outside the lock, against the setter.
    #[track_caller]
    pub fn set_after(&self, value: T, seen: &T) -> Result<(), Overtaken>
    where
        T: PartialEq + crate::block::DeepSize,
    {
        let size = crate::block::deep_size(&value);
        let mut value = Some(value);
        crate::block::hand_over(size);
        let stored = self.held(|held| match *held == *seen {
            true => {
                crate::block::received(size);
                if let Some(value) = value.take() {
                    *held = value;
                }
                true
            }
            false => false,
        });
        if stored {
            return Ok(());
        }
        // Back to the setter, which held it a moment ago: no new memory,
        // so nothing to refuse.
        crate::block::take_over(size);
        Err(Overtaken)
    }

    /// **Exclusive even to read**, and that is not an oversight.
    ///
    /// Two reads of one lock do not conflict, so a shared borrow would let a
    /// nested `access` through. The crossing shape cannot: a mutex is exclusive
    /// and its owner check fires. One written type may not behave two ways
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D9), and of the two
    /// ways to agree, refusing is the one that keeps Part II 12.2's rule whole -
    /// re-entering a lock is a defect in code that is not well-formed, whichever
    /// door it happens through.
    ///
    /// **A read poisons nothing** (ADR-327 D11): the block cannot change the
    /// value, so a panic in it leaves the value whole.
    #[track_caller]
    fn read<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.unmarked(|held| f(held))
    }
}

/// A lock for a value that crosses a thread, or that the analysis could not
/// prove does not.
///
/// An OS mutex **with an owner check**, and the check is D4's price rather than
/// an extra: without it this shape hangs where [`Local`] reports, and one written
/// type would mean two behaviours. Measured at +2.0 ns, ×1.15 against a bare
/// `Mutex` (`docs/mutex-floor.md` §4.3).
#[derive(Debug, Default)]
pub struct Crossing<T> {
    /// The value itself, for the reason [`Local`]'s is
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D12).
    /// In a block of its own, as [`Local`]'s (ADR-327 D5).
    inner: Mutex<crate::block::Housed<T>>,
    /// Which task holds it, or `0` for nobody.
    ///
    /// **An atomic and not a second `Mutex`**, which is a correctness point
    /// before it is a cost: the mark is written *under* the guard, so it names
    /// the task that actually holds the lock rather than one that hopes to. It is
    /// **read** before the acquisition, because once that blocks there is nothing
    /// left to report to - and a read that races tells us nothing about *our own*
    /// re-entry, which is the only thing this answers, because only we ever write
    /// our own id here.
    ///
    /// Measured: as a `Mutex<Option<u64>>` written before the acquisition, one
    /// door cost three mutex acquisitions and 63.5 ns; this is 17.2
    /// (`benches/lockfree`).
    held_by: AtomicU64,
    /// How to build it again, for `SharedMut::supervised` (ADR-327 D3).
    #[allow(clippy::type_complexity)]
    rebuild: Option<(
        Builder<dyn Fn() -> T + Send + Sync>,
        std::sync::Mutex<Phase>,
    )>,
}

impl<T> Crossing<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Mutex::new(crate::block::Housed::new(value)),
            held_by: AtomicU64::new(NOBODY),
            rebuild: None,
        }
    }

    /// [`Local::weighed`], where the value crosses threads (#1751).
    pub fn weighed(value: T, size: impl FnOnce(&T) -> usize) -> Self {
        let size = size(&value);
        Self {
            inner: Mutex::new(crate::block::Housed::moved_in(value, size)),
            held_by: AtomicU64::new(NOBODY),
            rebuild: None,
        }
    }

    /// `SharedMut::supervised(fn { … })` where the value crosses threads.
    pub fn supervised_kept<F: Fn() -> T + ?Sized + Send + Sync + 'static>(
        build: crate::func::Kept<F>,
        policy: impl crate::supervisor::Restart + Send + 'static,
    ) -> Self {
        Self::supervised(move || (build.0)(), policy)
    }

    /// [`Local::supervised`], where the value crosses threads.
    pub fn supervised(
        build: impl Fn() -> T + Send + Sync + 'static,
        policy: impl crate::supervisor::Restart + Send + 'static,
    ) -> Self {
        Self {
            inner: Mutex::new(crate::block::Housed::built(&build)),
            held_by: AtomicU64::new(NOBODY),
            rebuild: Some((Builder(Box::new(build)), Phase::new(policy))),
        }
    }

    /// Part II 12.2's first two doors, which run no code of the user's.
    #[track_caller]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.access(|value| value.clone())
    }

    /// [`Local::set`]'s deep size moves here too (ADR-327 D9): a value that
    /// does not fit poisons the mutex.
    #[track_caller]
    pub fn set(&self, value: T)
    where
        T: crate::block::DeepSize,
    {
        let size = crate::block::deep_size(&value);
        crate::block::hand_over(size);
        self.hold(|slot| {
            crate::block::received(size);
            *slot = value
        });
    }

    /// **Reading in place** ([ADR-281](../../../docs/specification/adr/adr-281.md)
    /// D1), poisoning nothing (ADR-327 D11).
    #[track_caller]
    pub fn access<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.unmarked(|slot| f(slot))
    }

    /// **Where locked data changes**
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D12): the block is
    /// handed the address under the guard, changes the value in place, and
    /// returns nothing. The lock is held across the whole of it, so nothing
    /// sees a gap — and there is no gap to see, because nothing is moved out.
    #[track_caller]
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.hold(f);
    }

    /// [`Local::update_copied`], where the value crosses threads: the copy is
    /// put in place before the guard is released.
    #[track_caller]
    pub fn update_copied(&self, f: impl FnOnce(&mut T))
    where
        T: Copy,
    {
        self.unmarked(|slot| on_a_copy(slot, f))
    }

    /// **The one door for a stamped value**
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D26), the
    /// crossing shape. [`Local::set_after`] carries the reasoning; what is
    /// different here is only that the lock is a mutex, so the compare and the
    /// store happen under the guard the owner check already took.
    ///
    /// **The value's deep size moves as [`set`](Self::set)'s does** (ADR-327
    /// D9, #1554), and only where it is stored: an overtaken value stays
    /// the setter's, and is dropped outside the lock, against the setter.
    #[track_caller]
    pub fn set_after(&self, value: T, seen: &T) -> Result<(), Overtaken>
    where
        T: PartialEq + crate::block::DeepSize,
    {
        let size = crate::block::deep_size(&value);
        let mut value = Some(value);
        crate::block::hand_over(size);
        let stored = self.hold(|slot| match *slot == *seen {
            true => {
                crate::block::received(size);
                if let Some(value) = value.take() {
                    *slot = value;
                }
                true
            }
            false => false,
        });
        if stored {
            return Ok(());
        }
        // Back to the setter, which held it a moment ago: no new memory,
        // so nothing to refuse.
        crate::block::take_over(size);
        Err(Overtaken)
    }

    /// The owner check and the guard, around whatever the door does with the
    /// slot — written once, because every door needs both and a door that
    /// forgot one would be the hang this shape carries the check to prevent.
    #[track_caller]
    fn hold<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        self.hold_in(true, f)
    }

    /// [`hold`](Self::hold), with the value's block in place or not: a door
    /// that leaves the value whole builds nothing that stays in it (D9,
    /// #1554).
    #[track_caller]
    fn hold_in<R>(&self, in_block: bool, f: impl FnOnce(&mut T) -> R) -> R {
        let me = current_task();
        // **Read before the acquisition**, because once that blocks there is
        // nothing left to report to - which is the whole reason this shape
        // carries a mark at all.
        //
        // **And it is a build option** ([ADR-281](../../../docs/specification/adr/adr-281.md)
        // D8, Part I 1.2): under D2 a re-entrant acquisition cannot happen in a
        // correct compiler, so this is self-control of that rule rather than
        // error handling, and it is a guarantee that may be declined. For every
        // program that obeys the nesting rule both builds behave identically —
        // the switch decides only whether a violation is **noticed**.
        //
        // **It reaches this shape and not [`Local`]'s**, which is the honest
        // scope rather than an omission: the mark below is this compiler's, and
        // declining it costs an atomic load and store per acquisition
        // ([ADR-281](../../../docs/specification/adr/adr-281.md) D6 makes that
        // free at one thread and D3 charges it only on values that cross).
        // `Local`'s check is a `RefCell`'s own borrow flag — the language
        // below's, which no switch of ours declines.
        #[cfg(feature = "reentrancy-check")]
        if self.held_by.load(Ordering::Relaxed) == me {
            reentered();
        }

        // **A poisoned value is never read** (ADR-327 D1, D2): a task that
        // panicked while it held the lock may have left it half-changed, and
        // the mutex says so. The door panics in the task that opens it.
        let mut guard = match self.inner.lock() {
            Ok(guard) => guard,
            Err(left) => match &self.rebuild {
                Some((build, phase)) => {
                    Phase::recover(phase);
                    let mut guard = left.into_inner();
                    guard.inside(|value| *value = (build.0)());
                    self.inner.clear_poison();
                    guard
                }
                None => poisoned(),
            },
        };
        // **Written under the guard**, so the mark names the holder rather than
        // a hopeful: before, it was set before the acquisition, and while one
        // task waited the mark said *its* name although another held the lock.
        self.held_by.store(me, Ordering::Relaxed);
        // **Cleared on the way out, a panic's way too**: a mark left behind
        // would make the next door read as a re-entry rather than as the
        // poisoned value it is.
        let _released = Released(&self.held_by);
        if in_block {
            guard.inside(f)
        } else {
            f(guard.get_mut())
        }
    }

    /// **The guard with no poison** (ADR-327 D11), for a block that cannot
    /// leave the value half-changed: `std`'s mutex poisons itself when its
    /// guard is dropped by a panic, so the panic is caught while the guard is
    /// held, the guard is dropped as on any other way out, and the panic goes
    /// on from there. The panic hook has run already where it began. The
    /// value's block is not in place, as in [`Local`]'s (D9, #1554).
    #[track_caller]
    fn unmarked<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let ended = self.hold_in(false, |slot| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(slot)))
        });
        match ended {
            Ok(result) => result,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
}

/// The block on a copy of `slot`, put in place where the block ends; a panic
/// in it leaves `slot` as it was (ADR-327 D11).
#[inline(always)]
fn on_a_copy<T: Copy>(slot: &mut T, f: impl FnOnce(&mut T)) {
    let mut copy = *slot;
    f(&mut copy);
    *slot = copy;
}

/// Clears [`Crossing`]'s owner mark when the block ends, however it ends.
struct Released<'a>(&'a AtomicU64);

impl Drop for Released<'_> {
    fn drop(&mut self) {
        self.0.store(NOBODY, Ordering::Relaxed);
    }
}

/// Who is asking, for the owner check.
///
/// The thread is the unit today, because a task does not move between threads
/// while it holds a lock: `access` takes a lambda that runs straight through, so
/// there is no suspension point between the two lines above for a task to be
/// moved at ([ADR-005](../../../docs/specification/adr/adr-005.md) D6). When a
/// task identity exists this is the one line that changes.
fn current_task() -> u64 {
    WHO.with(|who| *who)
}

/// Nobody holds it. `WHO` never answers this, so the two can never be confused.
const NOBODY: u64 = 0;

thread_local! {
    /// **Computed once per thread**, because it used to be a SipHash of the
    /// thread id on every single door - measured at a large share of what a door
    /// cost (`benches/lockfree`).
    ///
    /// `| 1` so it is never [`NOBODY`]: the hash is opaque and one value of it
    /// would otherwise mean "unheld".
    static WHO: u64 = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);
        hasher.finish() | 1
    };
}

// **Two tests no program can make yet** (#2088): they check that `update`
// and `access` do not copy the value, by its buffer's address, which no
// Nikaia program observes and no condition input reaches. They stay here
// until it is decided how a test written in Nikaia shows that a door does
// not copy; every other test of this file is a program in
// `tests/compiler/tests/` (#2189).
#[cfg(test)]
mod tests {
    use super::*;

    /// **`update` moves, it does not copy**, which is the finding that made
    /// `access` stop needing to write (D2). The list's buffer is the same
    /// allocation after an append, so nothing was copied to add one entry.
    #[test]
    fn update_appends_without_copying_the_value() {
        // Room to spare, so a `push` does not grow the buffer: what is being
        // asked is whether `update` copies, not whether `Vec` reallocates.
        let mut start = Vec::with_capacity(8);
        start.extend([1_i64, 2, 3]);
        let local = Local::new(start);
        let before = local.access(|v| v.as_ptr());
        local.update(|v| v.push(4));
        assert_eq!(local.access(|v| v.len()), 4);
        assert_eq!(
            local.access(|v| v.as_ptr()),
            before,
            "the same buffer: an append is a move and not a copy"
        );
    }

    /// And reading in place answers a question about a large value without
    /// taking a copy of it (D1) — the one thing neither `get` nor `update` does.
    #[test]
    fn access_reads_a_large_value_in_place() {
        let local = Local::new((0..10_000_i64).collect::<Vec<_>>());
        let before = local.access(|v| v.as_ptr());
        assert_eq!(local.access(|v| v.len()), 10_000);
        assert_eq!(local.access(|v| v.as_ptr()), before, "nothing was copied");
    }
}

/// **A value that fits a machine word, as the word itself**
/// ([ADR-281](../../../docs/specification/adr/adr-281.md) D13, D14;
/// [ADR-281](../../../docs/specification/adr/adr-281.md)).
///
/// The third shape, and the one [`Crossing`] gives way to where the value is a
/// number, a truth value or a character and nothing takes it in a door over
/// several locks. There is no lock: `update` runs the block on a **copy**,
/// compare-and-swaps the result in, and on a collision runs it again on a
/// fresh copy — which ADR-281 D14 permits, because a block is `sync`, touches
/// no lock and changes nothing outside `v`, so the run that lost leaves no
/// trace. A panic in the block leaves the word as it was (D4).
///
/// Measured against the crossing shape at ×0.63–×0.67 uncontended and several
/// times less under contention (`docs/lock-free.md` §2). It never replaces
/// [`Local`], which a compare-and-swap loses to by ×6.5.
///
/// **Not a [`Door`]**, deliberately: a door over two locks holds both at once,
/// and a retry cannot be held. The compiler gives such a value the crossing
/// shape; were it to get this one, the program would not compile rather than
/// race.
pub struct Word<T> {
    bits: AtomicU64,
    of: std::marker::PhantomData<T>,
}

/// **What fits a word**: a value that is its own bits, and back.
pub trait Bits: Copy {
    /// The value as 64 bits.
    fn to_bits(self) -> u64;
    /// The value those bits are.
    fn from_bits(bits: u64) -> Self;
}

macro_rules! bits_by_cast {
    ($($t:ty),*) => {$(
        impl Bits for $t {
            fn to_bits(self) -> u64 {
                self as u64
            }
            fn from_bits(bits: u64) -> Self {
                bits as $t
            }
        }
    )*};
}
bits_by_cast!(i8, i16, i32, i64, u8, u16, u32, u64, isize, usize);

impl Bits for f64 {
    fn to_bits(self) -> u64 {
        f64::to_bits(self)
    }
    fn from_bits(bits: u64) -> Self {
        f64::from_bits(bits)
    }
}

impl Bits for f32 {
    fn to_bits(self) -> u64 {
        u64::from(f32::to_bits(self))
    }
    fn from_bits(bits: u64) -> Self {
        f32::from_bits(bits as u32)
    }
}

impl Bits for bool {
    fn to_bits(self) -> u64 {
        u64::from(self)
    }
    fn from_bits(bits: u64) -> Self {
        bits != 0
    }
}

impl Bits for char {
    fn to_bits(self) -> u64 {
        u64::from(u32::from(self))
    }
    fn from_bits(bits: u64) -> Self {
        // Only a `char` is ever stored, so the bits are always one.
        char::from_u32(bits as u32).unwrap_or('\0')
    }
}

impl<T: Bits> Word<T> {
    pub fn new(value: T) -> Self {
        Self {
            bits: AtomicU64::new(value.to_bits()),
            of: std::marker::PhantomData,
        }
    }

    /// The value, read.
    pub fn get(&self) -> T {
        T::from_bits(self.bits.load(Ordering::Acquire))
    }

    /// The value, written.
    pub fn set(&self, value: T) {
        self.bits.store(value.to_bits(), Ordering::Release);
    }

    /// **Reading in place**, which for a word is reading a copy: nothing can
    /// change what the block was handed.
    pub fn access<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.get())
    }

    /// `update` of a value that copies (ADR-327 D11), which this shape's
    /// `update` is already.
    pub fn update_copied(&self, f: impl FnMut(&mut T)) {
        self.update(f)
    }

    /// **The block on a copy, swapped in; again on a collision**
    /// (ADR-281 D13, D14).
    pub fn update(&self, mut f: impl FnMut(&mut T)) {
        let mut seen = self.bits.load(Ordering::Acquire);
        loop {
            let mut value = T::from_bits(seen);
            f(&mut value);
            match self.bits.compare_exchange_weak(
                seen,
                value.to_bits(),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(now) => seen = now,
            }
        }
    }

    /// **The one door for a stamped value**
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D26), as one
    /// compare-and-swap: the value goes in where the word still holds what
    /// was seen. Compared as values, so a number equal to the one seen is the
    /// one seen, as the other shapes compare it.
    pub fn set_after(&self, value: T, seen: &T) -> Result<(), Overtaken>
    where
        T: PartialEq,
    {
        let mut now = self.bits.load(Ordering::Acquire);
        loop {
            if T::from_bits(now) != *seen {
                return Err(Overtaken);
            }
            match self.bits.compare_exchange_weak(
                now,
                value.to_bits(),
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(moved) => now = moved,
            }
        }
    }
}

impl<T: Bits + std::fmt::Debug> std::fmt::Debug for Word<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Word").field(&self.get()).finish()
    }
}

impl<T: Bits + Default> Default for Word<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// One lock, whichever shape it got — what a door over **several** of them needs
/// ([ADR-281](../../../docs/specification/adr/adr-281.md)).
///
/// A door over two locks cannot be written against `Local` or `Crossing` by name:
/// which shape a value gets is decided per value, so one program can hold both,
/// and the two values a transfer names need not have got the same answer. What
/// the door needs of each is the same three things whichever shape it is, and
/// this says which three.
pub trait Door {
    /// What the lock holds.
    type Held;

    /// **Where this lock stands in the one order everybody takes them in.**
    ///
    /// Its address. Two threads that both want A and B take them in the same
    /// order because they compute the same two numbers, which is what makes a
    /// cycle impossible rather than unlikely (Part II, 12.3).
    fn ordering(&self) -> usize;

    /// Read in place, the way `access` does.
    fn reading<R>(&self, f: impl FnOnce(&Self::Held) -> R) -> R;

    /// Change the value in place, the way `update` does
    /// ([ADR-281](../../../docs/specification/adr/adr-281.md) D12) — and hand a
    /// result outward, which is what lets two of these nest.
    ///
    /// It used to take the value **out** and put one back, which is what made
    /// an empty slot a state every other door had to report. D1 takes the cause
    /// away: the block is handed the address, so nothing is moved and no slot
    /// is ever empty.
    fn changing<R>(&self, f: impl FnOnce(&mut Self::Held) -> R) -> R;

    /// Open the lock with no poison mark (ADR-327 D11): only for a block that
    /// reads, or works on copies and puts them in place at its end, so a
    /// panic in it leaves the value as it was.
    fn unmarked<R>(&self, f: impl FnOnce(&mut Self::Held) -> R) -> R;
}

impl<T> Door for Local<T> {
    type Held = T;

    fn ordering(&self) -> usize {
        self as *const Self as usize
    }

    #[track_caller]
    fn reading<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.read(f)
    }

    #[track_caller]
    fn changing<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        self.held(f)
    }

    #[track_caller]
    fn unmarked<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        Local::unmarked(self, f)
    }
}

impl<T> Door for Crossing<T> {
    type Held = T;

    fn ordering(&self) -> usize {
        self as *const Self as usize
    }

    #[track_caller]
    fn reading<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.access(f)
    }

    #[track_caller]
    fn changing<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        self.hold(f)
    }

    #[track_caller]
    fn unmarked<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        Crossing::unmarked(self, f)
    }
}

/// **A handle on a lock is a lock**, so a door takes one without being told which
/// it was given ([ADR-281](../../../docs/specification/adr/adr-281.md) D17).
///
/// `SharedMut[T]` is a count around a lock and `Locked[T]` is the lock in a
/// field; the door is written once and both reach it. The **order** delegates to
/// the lock inside rather than to the handle, because two handles on one
/// allocation must compute the same number or the order they promise is not one.
impl<D: Door> Door for std::rc::Rc<D> {
    type Held = D::Held;

    fn ordering(&self) -> usize {
        (**self).ordering()
    }

    #[track_caller]
    fn reading<R>(&self, f: impl FnOnce(&Self::Held) -> R) -> R {
        (**self).reading(f)
    }

    #[track_caller]
    fn changing<R>(&self, f: impl FnOnce(&mut Self::Held) -> R) -> R {
        (**self).changing(f)
    }

    #[track_caller]
    fn unmarked<R>(&self, f: impl FnOnce(&mut Self::Held) -> R) -> R {
        (**self).unmarked(f)
    }
}

impl<D: Door> Door for std::sync::Arc<D> {
    type Held = D::Held;

    fn ordering(&self) -> usize {
        (**self).ordering()
    }

    #[track_caller]
    fn reading<R>(&self, f: impl FnOnce(&Self::Held) -> R) -> R {
        (**self).reading(f)
    }

    #[track_caller]
    fn changing<R>(&self, f: impl FnOnce(&mut Self::Held) -> R) -> R {
        (**self).changing(f)
    }

    #[track_caller]
    fn unmarked<R>(&self, f: impl FnOnce(&mut Self::Held) -> R) -> R {
        (**self).unmarked(f)
    }
}

/// **Both locks read, both held at once, and taken in one global order**
/// (Part II 12.3, [ADR-281](../../../docs/specification/adr/adr-281.md) D18).
///
/// The order is by address and not by the order they are written, which is the
/// whole point: `access_all(a, b)` in one task and `access_all(b, a)` in another
/// take them the same way round, so there is no cycle to deadlock in. Nesting
/// them by hand is what the language refuses (Part II, 12.3).
#[track_caller]
pub fn access_all<A, B, R>(a: &A, b: &B, f: impl FnOnce(&A::Held, &B::Held) -> R) -> R
where
    A: Door,
    B: Door,
{
    match a.ordering() <= b.ordering() {
        true => a.reading(|x| b.reading(|y| f(x, y))),
        false => b.reading(|y| a.reading(|x| f(x, y))),
    }
}

/// **Both locks written, changed in place**
/// ([ADR-281](../../../docs/specification/adr/adr-281.md) D18).
///
/// D12's one form, widened: **one `mut` per lock and nothing returned**. Both
/// are held for the whole of the block, so nothing sees either between the two
/// changes, and the same address order as above is what makes a cycle
/// impossible.
///
/// It used to take the old values by value and hand back a pair. That is what
/// made an empty slot a state — a block that panicked between the two left one
/// lock without a value — and D18 takes the cause away rather than the symptom.
///
/// Where one value does not copy, a panic in the block poisons both: values
/// changed in one door must agree (ADR-327 D11).
#[track_caller]
pub fn update_all<A, B>(a: &A, b: &B, f: impl FnOnce(&mut A::Held, &mut B::Held))
where
    A: Door,
    B: Door,
{
    match a.ordering() <= b.ordering() {
        true => a.changing(|x| b.changing(|y| f(x, y))),
        false => b.changing(|y| a.changing(|x| f(x, y))),
    }
}

/// **`update_all` where both values copy** (ADR-327 D11): the block works on
/// copies, and both are put in place together while both locks are still
/// held, so a panic in the block poisons neither. The compiler writes it
/// where both values are numbers, `bool`s or `char`s.
#[track_caller]
pub fn update_all_copied<A, B>(a: &A, b: &B, f: impl FnOnce(&mut A::Held, &mut B::Held))
where
    A: Door,
    B: Door,
    A::Held: Copy,
    B::Held: Copy,
{
    let both = |x: &mut A::Held, y: &mut B::Held| {
        let (mut cx, mut cy) = (*x, *y);
        f(&mut cx, &mut cy);
        *x = cx;
        *y = cy;
    };
    match a.ordering() <= b.ordering() {
        true => a.unmarked(|x| b.unmarked(|y| both(x, y))),
        false => b.unmarked(|y| a.unmarked(|x| both(x, y))),
    }
}
