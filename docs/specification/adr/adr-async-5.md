# ADR-async-5: Every Task Is Started by `async` in a Group It Does Not Outlive; `async e` Is Waited For Where Its Value Is First Needed, a Handle Is Asked For by an Option, and Fan-Out Over a Collection Is the `par_iter` Chain

**Status:** Accepted
**Date:** October 30, 2026 (owner's decisions on [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983))
**Answers:** what a program writes to run work beside the code that follows it - one value, a task it may wait for, cancel or let go, work over every element of a collection - without coloring functions and without `await`; and what rules every such task follows, whatever form started it
**Supersedes / Related:**
[ADR-292](adr-292.md) D2-D4, D9, D15, D16 and §6 (changed: the joining form is `async`, D5-D9; D1, D5, D6, D8, D10-D14 and D17-D20 stand and are read for `async`);
[ADR-346](adr-346.md) D1-D6 (every task error has a receiver; they hold for `async(detached)`, D13; D3, D4 changed by D17), D7 (changed by D21);
[ADR-235](adr-235.md) D1, D2 (changed: a `par_iter` lambda may pause, D24; a write to an outer name is still `NK2107`);
[ADR-327](adr-327.md) D5-D7, D9, D10 (a block per task; D6 changed by D19);
[ADR-326](adr-326.md) D2, D3 (admission by an exchangeable strategy and the ceilings the runtime enforces, taken for every task, D18);
[ADR-328](adr-328.md) D1, D7, D8, D10 (supervisors are a policy, D4; a crash is an error to the waiter; a pausing child task is polled by its group, D20);
[ADR-276](adr-276.md) D13 (no loop has a value, D30) and D16 (a task is a boundary no jump leaves, D10);
[ADR-277](adr-277.md) D16 (a task is a block, not a callable);
[ADR-lambdas-1](adr-lambdas-1.md) D2 (no scope lambda is a once-parameter any more);
[ADR-372](adr-372.md) D9 (`fold`'s `step` and `merge` may change their first argument);
[ADR-298](adr-298.md) D1, D7, D8 (`async` is reserved because a construct uses it; `spawn` is released once the move is built, D13);
[ADR-288](adr-288.md) D1, D32 (`sync`; a body holding an `async` form pauses; `NK2211`, of which `NK2119` is the shape for a `par_iter` lambda);
[ADR-055](adr-055.md) (pausing is inferred; `async` in the emitted Rust is the language below's and unrelated to the word here);
[ADR-280](adr-280.md) D21 (the `secondary` list);
[ADR-297](adr-297.md) D4, D5 (a cleanup's failure attaches; parked cleanups);
[ADR-005](adr-005.md) §1 Group B (the crossing check a child task does not meet);
[ADR-037](adr-037.md), [ADR-303](adr-303.md) (`user_parallelism`; I/O the runtime performs is not user code);
[ADR-258](adr-258.md) D1-D4 (the four weights);
Part I 3.3, 5.3, 5.4, 7.1, 8.1, 8.1.2, 8.2, 8.3, the reserved words; Part II 11.2, Chapter 12, 12.2, 12.4-12.8; Part III 13.3b, A.2, 17.1's `std::task`, the `@detached` row, C.4, C.5, the `NK21xx` and `NK22xx` rows, `--overlaps`

## 1. Question

**What does a program write to have work run beside the code that follows it,
and what rules does that work follow?**

The work comes in four shapes: one value computed while other code runs; a
task the program may wait for, cancel, start later or let outlive the code that
started it; the same work over every element of a collection; and a race, a
restart policy or one task per connection, which `select`, supervisors and
`serve` provide. Each needs to say where it is waited for, what it may borrow,
where its failure goes, what memory it uses and how it is cancelled. Answered
form by form, the same few facts get a rule each time, and the rules drift
apart: a body accepted in one form and refused in another, two cancel
spellings, a time limit that counts from two places.

## 2. How others do it

All rows are from memory, not checked against a current manual.

| Language / tool | Its answer | What it costs there |
| :--- | :--- | :--- |
| Swift | child tasks (`async let x = f()`, read with `await x`; `TaskGroup.addTask`) are bound to their scope and may borrow; `Task { }` and `Task.detached { }` outlive it and give a handle with `.value` and `.cancel()` | colored functions (`async`, `await`); an `async let` not awaited is cancelled at scope end and its error dropped; a group's results come in completion order |
| Kotlin | `async { f() }` is an expression yielding a `Deferred<T>`, `launch` a `Job`, both children of a `CoroutineScope`; `CoroutineStart.LAZY` starts on `start()` | `suspend` colors; a value is read through `await()`; the first failure cancels the siblings |
| Trio (Python) | only nurseries, `n.start_soon(f)`; all failures as one `ExceptionGroup`; to outlive a function a task is started in a nursery passed in | a nursery object in every signature that starts work; values by side effect |
| Go | `go f()`; `errgroup.Group` with `SetLimit(n)` and `Wait()` returning the first error | no values, no structure; races found at run time if at all |
| Rust | `std::thread::scope` borrows, `spawn` needs `'static`; tokio `JoinHandle::abort()`, `buffered(n)`; Rayon `par_iter` with adaptive splitting | the borrowing form and the free one are two APIs; Rayon and tokio are two pools the program mixes by hand |
| C# / .NET | one `Task` for everything; `Parallel.ForEachAsync` runs one body per element where it awaits and the sync `Parallel.ForEach` chunks | no lifetime guarantee; an unobserved task's error is lost; colored |
| Cilk | `cilk_for` is the spawn mechanism plus a grain the runtime picks; a fold is a reducer | no pausing on I/O, so it never has to choose the execution |
| Erlang / OTP | one kind of process; supervisors and links are policies over it | nothing is borrowed, everything is copied |
| Haskell `par` | sparks evaluated beside the expression that uses them | no handle, no cancel; pure values only |

**What transfers.** Kotlin's and Trio's point that a task's lifetime is *which
group it is started in*, not a second kind of task; Erlang's, that a supervisor
is a policy over that one kind. Kotlin's expression form and lazy start;
Swift's split of a child handle from a detached one; the spark's point that a
value started beside its use needs no handle. Go's and Rust's limit; Trio's
list of every failure. Cilk's and Rayon's point that a data-parallel loop is
the task mechanism plus a grain, and C#'s that the choice between pieces and
one task per element follows whether the body waits - which this language knows
without a keyword (the ledger, Part II 12.1). C#'s stitched traces across a
wait, and V8's rule that the success path pays nothing for them.

**What does not.** The coloring: pausing is inferred here (Part I 8.1,
ADR-055), and a caller does not change when a callee starts to pause. Swift's
cancel-and-drop at scope end and C#'s lost errors: every error has a receiver
(ADR-346 D1). Completion order: the written order is the only reproducible one
(ADR-292 D5). Go's and C#'s absence of structure, and Trio's nursery in every
signature.

## 3. Options, weighed

| Option | Why it lost, or won |
| :--- | :--- |
| **One model, one word, an expression prefix** (§4) | **won**: one rule set for every task; one word, `async`, standing where an expression stands, so a value needs no name and no handle; the rare cases (a handle, a late start, a task that outlives its group) are options; fan-out is the one chain that can also say `filter`, `fold` and pieces of `Bytes` |
| **A form per need**, each with its own rules | **lost**: the same facts written five times and already diverging (a body that prints accepted in one form, refused in another; two cancel spellings; `within:` counted from two places) |
| **The word before a statement** (a binding, a loop, a block) | **lost**: a binding becomes the only place a value can start, so `f(async g(), async h())` would need two names, and a handle form would stand before a `let` whose value is not what the `let` binds |
| **A wait marker at the use** (`await x`) | **lost**: the compiler already knows the wait point; the marker is noise and invites `await` on calls, the coloring this language refused |
| **Every form hands back a handle** | **lost**: a `join()` at every use is that same `await`, and the common case pays for the rare one |
| **The lifetime inferred from use** (a task becomes free when its value leaves the group) | **lost**: whether a line borrows would depend on a later line, a `return` added at the bottom would move captures at the top, and an unbound task could not say "detached" at all |
| **A loop form for fan-out beside `par_iter`** | **lost**: two spellings and two rule sets for one idea; the chain already says what a loop cannot (`filter`, `fold`, `par_pieces`) |
| **One loop form, `par_iter` removed** (a fold by a reducer) | **lost**: removes a built and tested form, and a reducer is a value the body writes, an exception to `NK2107` |

## 4. Decision

### The model

### D1 - Every task is started in a group and lives at most as long as the group.

A **task** is whatever runs beside other code: what an `async` form starts, a
`select` arm, an element of a `par_iter` chain that runs as one, a
supervisor's child, a connection under `serve`. It is started in a **group**
and ends before the group does. A **child task**'s group is a block
(`async { }`, the innermost block, `select`) or a call (`par_iter`,
`supervisor::run`, `serve`); a **free task**'s group is the process, which
nobody waits for but `main`'s drain (Part III A.2).

### D2 - One capture rule: a task borrows what outlives its group and takes what does not.

A child task borrows what was bound outside its group and takes what was bound
in a block that ends before its group does, an enclosing block's own names
among them. For a free task every local ends before the process does, so it
takes everything it uses: Part I 8.3's move, with `NK2101`, is this rule where
the group is the process. A child task may borrow because nothing outlives its
group, so the crossing check of a free task has nothing to do (ADR-292 D4).

### D3 - Every task has a memory block and ends in one of four ways, and its end goes to the group's receiver.

Each task allocates from a block of its own (ADR-327 D5). It ends with a value,
a failure, a crash (`task::Crashed`) or a full block (`task::OverBudget`, D19),
and that end reaches its group's receiver: the wait point or the join (D22),
the group's end (D21), the supervisor's policy, the protocol, standard error
for a free task nobody joins (ADR-346 D6). Only a crash cancels the siblings of
a child task.

### D4 - Each form states only its group's policy; the rules are written once.

| Form | Its group | Policy |
| :--- | :--- | :--- |
| `async` (Part I 8.1.2) | the enclosing `async { }` or block | wait for all, keep every failure |
| `par_iter` (Part II 12.6) | the call | per element: results in input order, every failure kept (D26) |
| `select` (Part II 12.4) | the block | the first wins, the rest are cancelled |
| `supervisor::run` (Part II 12.8) | the call | restart by strategy and policy |
| `serve` (ADR-326) | the server | one per connection, by admission |
| `async(detached)` (Part I 8.2) | the process | nobody waits but `main`'s drain; a bound handle is the receiver |

Part I 8.1.2 states the model - group, capture, block, four ends, receiver -
and every other section that starts tasks (8.2, Part II 12.4, 12.6, 12.8, the
`serve` text) states its policy and refers to it. `async` and `select` are the
pair of ADR-292 D15: both start every task; `async` keeps every result,
`select` the first.

### Starting a task and waiting for it

### D5 - `async` is a prefix of an expression, and it colors nothing.

`async` stands where an expression stands: on the right of a `=`, as an
argument, as an operand, or as an expression statement. It never stands before
a `let`, a loop or a function's declaration: whether a function pauses is
inferred, there is no `await`, and a caller does not change when a callee
starts to pause. `async fn name(…)` is refused (`NK2115`), and the message says
that pausing is inferred. `async` is a reserved word because this construct
uses it (ADR-298 D1, D8). Options are written in parentheses directly after the
word, without a positional zone: `async(limit: 4) { … }`,
`async(detached) fn { … }`.

### D6 - `async e` starts at once and hands back the value, waited for where the value is first needed.

```nika
let user = async db::load_user(id)
let rights = async db::load_rights(id)
let prefs = cache::load_prefs(id)      // runs while both loads are in flight
render(user, rights, prefs)            // waits for both, then runs
```

The task evaluates `e`. What runs between its start and its wait point is its
**window**. The wait point is where the value is first needed:

* **A binding** (`let x = async e`): before the first statement of the
  binding's block that names `x` anywhere in it, nested blocks, loops,
  branches and lambdas included; the wait is there whether or not the branch
  that names `x` is taken. A statement is never split by the wait, so the
  window of a binding is a list of whole statements. A `let` pattern (ADR-291)
  and a written type are taken as `let` takes them; the first statement naming
  any of the names waits. A binding nothing names is waited for at its group's
  end.
* **An argument**: before the call that receives it. `f(async g(), async h())`
  runs `g` and `h` beside each other and calls `f` when both are done;
  `f(async g(), h())` runs `g` while `h()` computes on the main line.
* **An operand**: before the operation. `let t = async a() + async b()` runs
  both and adds.
* **A tuple**: `let (a, b) = (async g(), async h())` runs both; each part is
  waited for where its name is first named.

The callee of `f(async g())` never sees a pending value: it gets a plain `T`,
so no function is colored and no signature changes. There is no handle, so the
value form cannot be cancelled (D14).

### D7 - A task with nothing beside it is refused, and what runs beside is checked.

An `async` form whose window is empty is `NK2110`: `f(async g())` with no other
argument computed beside it, `let x = async g()` named in the very next
statement, `async g()` as the last statement of its group. The help removes
the word.

Tasks start in the written order, left to right, and the main line goes on
after each start. ADR-292 D3's touch-set check runs, pair by pair, between what
runs beside: a bound task against every statement of its window, another
task's start included; the tasks of one call's arguments against the arguments
computed on the main line; the operands of one operation; the tasks of one
group against each other. A pair that meets on a resource is `NK2104`, and the
message names it. A task the compiler cannot account for is not refused
(Part III C.4). A binding the window changes and a task names is `NK2104` (D8).
A task that changes a name bound outside it is `NK2107`.

### D8 - A pending value named inside another task is waited for in that task.

```nika
let a = async load(id)
let b = async enrich(a)                // b's task waits for a; the main line does not
show(b)
```

Ownership follows the rules every other use follows, applied to the task and
its window:

* **Both read:** `b`'s task reads `a`, and the main line names `a` later. Two
  reads meet on nothing; the main line waits for `a` where it first names it,
  not for `b`. The value lives in `a`'s block and outlives both.
* **The window changes it:** a `mut` method, an assignment or a `mut` argument
  on `a` in the window of a task that names `a` is `NK2104`, naming `a`. After
  the main line has waited for that task, it is allowed.
* **The task moves it:** `b`'s task takes `a` by value. `a` is handed to `b`'s
  task where `b` starts; a later mention of `a`, on the main line or in another
  task, is a use after the hand-over (`NK2105`), and the main line never waits
  for `a`.
* **A diamond:** `let b = async f(a)` and `let c = async g(a)` both wait for
  `a` and read the one value. If either moves `a`, the other's mention is
  refused as above.
* **A failure:** where `a` fails, its error is received by the first mention of
  `a` in written order, as a callee's error is its caller's (D22); every other
  task that waits for `a` cannot go on and is cancelled. One error, one
  receiver (ADR-346 D1).

### D9 - `async e` as a statement starts a task without a value, waited for at its group's end; `async { … }` is the group.

```nika
async {
    async audit::log(event)
    async metrics::count(event)
    report(event)                       // runs beside both
}                                       // both have ended here
```

A value the statement's task hands back is dropped, as an expression
statement's is. The block's own statements run on the main line, in order;
`async { }` starts nothing by itself. A task started anywhere inside it - in an
`if`, in a nested block that is not itself a group - belongs to the innermost
`async { }` around it in the same function; where none encloses it, its group
is the innermost block that holds the `async` form. It is a block expression:
its value is its last expression's, available once every task has ended. An
`async { }` that starts no task is refused (`NK2111`): it waits for nothing.

### D10 - A lambda after `async` is run, not bound; no jump leaves a task.

`let x = async fn { … }` and `async fn { … }` run the lambda's body as the
task, and `return` in it ends the task with its value. A task of several steps
is written this way; `async { … }` stays the group (D9). A task whose value is
a function is written `let f = async fn { fn { … } }`. A lambda that names an
argument is `NK2103`: a task is handed nothing.

A task is a boundary no jump leaves (ADR-276 D16): a `break`, `continue` or
`return` that would leave the expression of an `async` form is `NK1132`.

### Handles

### D11 - `async(handle) fn { … }` starts at once and hands back a handle bound to the group.

```nika
let h = async(handle) fn { crawl(site) }
index_local()
let pages = h.join() catch { [] }
```

The handle is a `task::ChildHandle[T]`. It borrows, as every child task does
(D2): it needs no `Send` and no heap, because it points at the task's slot in
its group. It has `join()` and `cancel()`, both of which take it, and no
`detach()`: a child handle does not outlive its group. One that leaves its
group - returned past it, stored in a value that outlives it - is `NK2122`. A
child handle that is neither joined nor cancelled is waited for at its group's
end, and its failure goes to the group as an unread failure does (D21); it is
not `NK2109`.

### D12 - `async(deferred) fn { … }` hands back a handle that runs only from `h.start()`.

```nika
let backup = async(deferred) fn { copy_all(src, dst) }
if dst.free() > need { backup.start() } else { backup.cancel() }
backup.join()
```

`deferred` implies `handle`. The task is not started until `h.start()`, which
starts it and keeps the handle. A deferred handle that is not started on some
path, and is not cancelled there, is `NK2121`; a second `start()` on a path is
`NK2121` too. A deferred handle's window begins at its `start()`, and the
touch-set check is asked from there.

### D13 - `async(detached) fn { … }` hands back a free handle, which may outlive the group.

```nika
let w = async(detached) fn { watch(dir) }
w.detach()
async(detached) fn { log_forever() }     // unbound: detached at once
```

`detached` implies `handle`. The task is a free task (D1): its group is the
process, so it takes what it captures (D2, Part I 8.3, `NK2101`). The handle is
a `TaskHandle[T]` and has `join()`, `join(within:)`, `cancel()` and
`detach()`; ADR-346 D2-D6 hold for it: a bound handle not taken is `NK2109`, an
unbound one is detached and its error goes to standard error unless the
fallback (D22) handles it. `detached` and `deferred` combine:
`async(detached, deferred) fn { … }`.

`spawn fn { … }` is the earlier spelling of `async(detached) fn { … }`. `spawn`
stays a reserved word, and the form stays accepted with that meaning, until the
move is built (every `spawn` in the repository rewritten,
[#991](https://github.com/Nikaia-Foundation/Nikaia/issues/991)); the commit
that removes the form releases the word (ADR-298 D7) and takes it off Part I's
list.

### D14 - A task is cancelled only through its handle, by `h.cancel()`.

`h.cancel()` stops the task at its current pause, tears down its values and
drops its outcome, a failure included (ADR-292 D13, D14); it takes the handle.
The group's end waits until a cancelled child task has stopped, its cleanups
included, because it may borrow from the group; a pausing cleanup is not
adopted by the runtime as a `select` loser's is. The value form `async e` has
no handle and cannot be cancelled; a program that may want to cancel writes
`async(handle) fn { e }`. There is no cancel function beside the method.

### Options, time and memory

### D15 - Every form that starts tasks takes `limit:`, `within:` and `admission:`.

* **`limit: i64`** - at most `n` tasks of this form are in flight at once; the
  others wait for a place, unstarted, in written order. A task is under every
  `limit` around it. A value below `1` panics, naming the option. It stands on
  `async { … }` and on the `par_iter` chain (D27); on a form that starts one
  task it is refused (`NK2112`). Without it, no number applies: admission
  decides (D18).
* **`limit: 1`** written as the literal runs the tasks one at a time, in order,
  in the background. Tasks that never run at once cannot meet, so they are not
  checked against each other (D7); each is still checked against its window.
* **`within: time::Duration`** - a deadline (D16).
* **`admission: task::Admission`** - the strategy of D18 for this form.

### D16 - `within:` on `async(…)` is a deadline counted from the start, and running out of it is the failure `task::TimedOut`.

```nika
let page = async(within: 5.seconds()) http::get(url) catch { Page::empty() }
```

`async(within: d) …` counts `d` from where the form starts: on the value form,
on the handle forms and on `async { }` alike. When `d` runs out, the form's
tasks still running are cancelled as `h.cancel()` cancels them (D14), and each
ends with the failure `task::TimedOut`. It arrives as any failure does (D22):
where the value is first needed, or at the `join` of a handle; for a statement
form and an `async { }`, at the group's end; for a detached task nobody joins,
at its receiver. Above, the fallback's `Page::empty()` becomes `page`.

**Why.** An option of `async` sits where the task starts and says what holds
for the task; a deadline is such a fact, and running out of it is the task
failing at what it was asked to do. A program that writes a fallback expects it
to stand in for a value that did not come in time. A limit on one wait is not
the task's but the waiter's, and is written where the wait is (D17).

### D17 - `h.join(within: d)` is a limit on this wait; with `on_timeout: task::OnTimeout::Cancel` it cancels the task.

```nika
let h = async(detached) fn { fetch(url) }
let page = h.join(within: 2.seconds()) catch {
    match error {
        task::TimedOut => { println("slow"); h.join() }   // the task kept running
        else => throw error
    }
}
let quick = async(detached) fn { fetch(mirror) }
let other = quick.join(within: 2.seconds(), on_timeout: task::OnTimeout::Cancel) catch { "" }
```

`within:` on `join` is an option of the wait, not of the task: `d` counts from
the call. When it runs out:

* **By default** `join` throws `task::TimedOut`, the task keeps running, and the
  handle stays live: it can be joined again, cancelled, or, a `TaskHandle`,
  detached.
* **With `on_timeout: task::OnTimeout::Cancel`** the task is cancelled as
  `h.cancel()` cancels it, its cleanups run, `join` throws `task::TimedOut`,
  and the handle is used up. The option's type is `task::OnTimeout`,
  `Keep | Cancel`, `Keep` by default, qualified as every `std` option's case is
  (`order: par::Order::Any`).

Both handles take the option (`ChildHandle` and `TaskHandle`). Where the task's
own `within:` (D16) runs out first, `join` throws `task::TimedOut` as well; a
handle still live after it is joined again with the same error, or cancelled,
which does nothing more. The `task::TimedOut` a join's own limit throws is the
join's error, not the task's, which has not failed: it meets a `catch` around
the join and otherwise leaves from it, and no fallback runs for it.

**`NK2109`.** A `join(within:)` without `on_timeout: task::OnTimeout::Cancel`
takes a `TaskHandle` only on its success path, and on a failure of the task's
body. On the path where it throws `task::TimedOut` the handle is still live,
and must be taken on every path out of the handler: in a `match error` arm for
`task::TimedOut`, in an `else` or binding arm that can match it, and in the
whole handler where it does not match on the error. A `join(within:,
on_timeout: task::OnTimeout::Cancel)` takes it on both paths. A `ChildHandle`
left live is waited for at its group's end (D11) and is not `NK2109`.

**Why.** The waiter may want to do something else and come back, and the task,
which another part of the program may also wait for, is not ended by one
impatient waiter unless that waiter says so.

### D18 - Admission, not a number, decides when a task starts.

Without `limit:`, a strategy decides: a value implementing `task::Admission`,
with the shape of [ADR-326](adr-326.md) D2's `net::Admission`
(`decide(ref mut self, load: task::Load) -> task::Decision sync`, on the
runtime's beat). `std` provides the default; a package may provide another;
`admission:` names one for a form. A new task starts only where its block fits
the budget of the task that runs the group and a descriptor is free; otherwise
it waits, unstarted, until a sibling ends. `limit: n` caps on top of the
strategy. What no strategy can change, the runtime enforces (ADR-326 D3): never
more descriptors than the system allows, and at the ceiling a task waits; it
never fails for it.

**What this guarantees:** no crash from fan-out, and none from memory a task
allocates through Nikaia. **What it does not cover:** memory a Rust crate or a
C library keeps (ADR-327 D8), other processes, the kernel's buffers.

**Rejected:** a default number, unbounded or `task::cores()` - unbounded fails
at the system's ceiling at run time, and the cores are the wrong measure for
tasks that wait; a configured key - ADR-326 D2 refused one for connections for
the same reasons.

### D19 - A full block ends a task with `task::OverBudget`, and only it.

*Changes [ADR-327](adr-327.md) D6.*

A full block unwinds the task like a panic, from the allocation site, and its
cleanups run. At every boundary it arrives as its own error,
`task::OverBudget`, beside `task::Crashed`: a join throws it, an `async` value
hands it on as any failure of its task (D22), a supervisor counts it in its
child's failures, and `http` under `serve` answers `503` where the response has
not begun. In a group only a panic, a bug, cancels the siblings;
`task::OverBudget` fails only its task. The panic hook does not run for it; it
is reported through the boundary's error, and for an unbound detached task on
standard error (ADR-346 D6). Inside the task it cannot be caught: an
allocation declares no `throws`. A shared value whose block is full is still
poisoned (ADR-327 D6).

**Why its own error:** a full block is load, not a bug; a receiver that can
tell them apart answers `503` or tries later where a panic would be reported as
a defect, and a group that cancelled the siblings of a task that only ran out
of room would turn one refusal into many.

### D20 - How a group runs is ADR-292 D6 and Part II 12.7.

* A child task that never pauses (`sync`, Part II 12.1) runs on the pool where
  tasks run in parallel; at `user_parallelism = no` it runs when the main line
  next pauses or waits, so that every pausing task written before that point is
  started first (ADR-292 D6). The setting changes the duration, not the
  meaning, because D7 has shown the pair meets on nothing.
* A child task that may pause is polled by its group, on the group's thread,
  and never handed to the runtime (ADR-328 D10), so it cannot outlive what it
  borrows.
* A body holding an `async` form pauses and is not `sync` (ADR-292 D16): a
  written `sync` on it is `NK2202`.
* An `async` form inside an open lock is `NK2203`: its tasks run while the lock
  is held.

### Failures

### D21 - A failure nobody reads waits for the group's end, and every one is kept.

A task's failure is its outcome, received where its value is read or its
handle joined (D22). A failure nobody reads - a statement form, a binding
nothing names, a mention in a branch not taken, a child handle left live - is
received at the end of the group:

* **The group ends normally** (its last statement, a `return`, a `break`): the
  first unread failure in written order - start order - is thrown there, and
  the others travel as its `secondary`, in written order (ADR-292 D5, D9;
  ADR-280 D21). A `return` whose group has an unread failure throws instead of
  returning.
* **The group is left by a failure** (a `throw`, a `?`, a failed read): the
  group first waits for every task, then that failure leaves, and every unread
  failure is attached to it as its `secondary`, in written order. The error the
  code chose to throw leads, because it is the one its handlers are written
  for, and the order is still the source's.

Early exit waits; it does not cancel. A task that panics cancels its siblings,
and the group throws `task::Crashed` where it ends (ADR-328 D8); a task whose
block is full fails only itself (D19). A cleanup's failure attaches as
ADR-292 D10 says.

### D22 - A failure meets the same handlers in every form, in this order.

```nika
let page = async http::get(url) catch { Page::empty() }   // the fallback
let h = async(handle) fn { crawl(site) } catch { [] }     // the fallback of a handle
let n = h.join() catch { [] }                               // a catch where it arrives
```

1. **In the task.** A `catch` inside the task's expression or lambda is the
   task's own and runs at once, when the task fails.
2. **Where the failure arrives.** Otherwise the failure is held; no handler
   runs. It arrives at the mention that waits (the value form) or at the
   `join` (a handle), and a `catch` that encloses that point and lies inside
   the binding's block handles it.
3. **The fallback.** Otherwise the `catch` written directly after the `async`
   expression runs: for the value form at the wait point, its value becoming
   the value, and the statement goes on with it; for a handle inside the join,
   its value becoming the join's; for a detached handle that is never joined,
   in the task itself when the task fails, instead of the error going only to
   standard error.
4. **The receiver.** Otherwise the failure goes on: it leaves from the wait
   point or the join, or, unread, it is the group's at its end (D21); for a
   detached task nobody joins, standard error (ADR-346 D6).

A cancellation the program asks for (`h.cancel()`, the losing arm of a
`select`, a join's `on_timeout: task::OnTimeout::Cancel`) is not a failure: it
runs no fallback. `task::OverBudget` (D19) and a deadline's `task::TimedOut`
(D16) are failures and run it. Once a `catch` where the failure arrived has
taken it, a later mention of the value on that path is refused unless the form
has a fallback (`NK2118`), because the name has no value there. The `throws`
walk: the function throws what the fallback throws, or the task's whole set
where there is no fallback.

**What it costs, named:** a name can throw - `page.len()` fails without a call
that fails being written there, toward a `catch` around it; and which handler
runs depends on the path that reaches the mention, so a reader looks in up to
three places. The precedence is fixed and the places are lexical, which is
what keeps it readable.

### D23 - An error that crosses a task's boundary records where the task was started and where the failure arrived.

Beside its raise site, such an error records a **started** site (the `async`
form) and an **arrived** site (the wait point, or the `join` for a handle), as
text the compiler writes, like the raise site, so the success path costs
nothing. A chain of pending values gives one started and arrived pair per
crossing, in order; each `secondary` error carries its own. The long form shows
`raised`, `started` and `arrived` lines. Under `NIKAIA_TRACE=1` the task's
trace and the waiting function's are stitched, with a line
`── started at <site> ──` between them, per crossing.

### Fan-out over a collection

### D24 - Fan-out over a collection is `std`'s `par_iter` chain, and its lambdas may pause.

```nika
let bright = pixels.par_iter().map fn(p) { p.brightness * 1.5 }.collect()
let pages = urls.par_iter(limit: 8).map fn(u) { http::get(u) }.collect()
```

`map`, `filter` and `for_each` take a lambda that may pause; `NK2209` is not
asked of them. The tasks a chain runs belong to the call: the call is their
group, and it returns when every one has ended (D4).

### D25 - The compiler picks the execution from the lambda's pause property.

* **A lambda that never pauses** runs in pieces on the cores, claimed while the
  work runs (Part II 12.6), at most `task::cores()` at once.
* **A lambda that may pause** runs one task per element, under admission and
  `limit:` (D18), polled by the call's group (D20).

The meaning does not depend on which: D26's rules hold for both, and the
setting of `user_parallelism` changes the duration only.

### D26 - One rule set for every element, the same as for every task.

* Results come back in input order, whatever order the elements end in.
* Every failure is kept: the first in input order is thrown where the call
  returns, the others are its `secondary` list in input order (D21).
* A full block fails only its element, with `task::OverBudget` (D19); only a
  panic cancels the other elements, and the call throws `task::Crashed`.
* `NK2104` is asked of a `par_iter` lambda against itself, as of tasks that run
  beside each other: a lambda that writes stdout or another resource is
  refused, naming it. `limit: 1` written as the literal exempts it (D15).
* `NK2107`: a lambda that changes a name bound outside it.

### D27 - `limit:` replaces `workers:`; `within:` and `admission:` are taken by the chain.

`xs.par_iter(limit: n, within: d, admission: s, piece: k)`, and `limit:` on
`fold` and `par_fold` where `workers:` stood. `limit: i64` caps how many pieces
or tasks run at once; below `1` it panics, naming the option. `within:`
cancels the call's running tasks when it runs out, and the call throws
`task::TimedOut`. `admission:` names the strategy for a pausing chain; a
computing one ignores it.

### D28 - What the shared rules add is paid only where the compiler knows it is needed.

* A failure slot only for a lambda that can throw, and its list is allocated on
  the failure path; a lambda that cannot throw has no slot.
* A block reset per element only for a lambda that allocates and keeps memory
  past its element; one that does not shares its piece's block untouched.
* Every check (`NK2104`, `NK2107`, `NK2119`) is compile-time only.

Before the lowering is accepted, `par_iter().map(…).collect()` over 10^6
elements is counted against Rayon (callgrind, release builds on both sides,
LTO stated, the same output first), once with a lambda that can throw and once
with one that allocates.

### D29 - The lambdas of `fold`, `par_pieces` and `par_fold` never pause.

`init`, `step` and `merge` of `fold` and `par_fold`, and a `par_pieces`
lambda, are action code and may not pause (`NK2209`, [ADR-235](adr-235.md)).
A fold over values that need a pause is written as a pausing `map` followed by
the `fold`: `xs.par_iter().map fn(u) { http::get(u) }.fold(…)`. The help of
`NK2209` on a `fold` lambda names that chain.

**Why.** An accumulator is folded by one worker at a time without a lock
(Part II 12.6); a step that pauses would hold the accumulator across the pause,
or fold it by tasks that meet. The pausing part has a place of its own in the
chain already.

### D30 - `async` starts only tasks whose number stands in the source; no loop has a value.

An `async` form inside a loop's body - the same function, a nested block of it
included - is refused (`NK2120`), and the help names
`xs.par_iter().for_each fn(x) { … }`. A lambda's body is a function of its own
(Part I 5.3), so a lambda written inside a loop may hold an `async` form. So a
group's tasks are counted in the source, and a group of fixed size is laid out
without an allocation. There is no `async` loop, and ADR-276 D13 holds for
every loop: no loop has a value.

### D31 - `async xs.par_iter() … .collect()` is one task that fans out, beside the code after it.

```nika
let pages = async urls.par_iter().map fn(u) { http::get(u) }.collect()
log(f"{urls.len()} requests started")
for page in pages { index(page) }      // waits for every one
```

The prefix makes the whole chain one task of the enclosing group; the chain's
own elements are the tasks of the call's group (D24).

### Tools, codes and names

### D32 - `--overlaps` reports every task and every `par_iter` execution, and a lambda that starts to pause is a warning.

`--overlaps` lists each task an `async` form starts, its window (the first and
last line), where it is waited for, its group's end and which tasks are started
first (D20); and every `par_iter` call with the execution chosen (pieces or one
task per element) and why: the call that makes the lambda pause, or none. The
name says what it reports - what runs beside what - and no keyword.

Where the committed ledger recorded a `par_iter` site as running in pieces and
its lambda can now pause, the build warns (`NK2119`) at the call that made it
pause, as `NK2211` does for a function (ADR-288 D32); committing the ledger
acknowledges it. A lambda written to pause gets no warning, because nothing
changed.

### D33 - The codes.

`NK2110` a task with nothing beside it (D7); `NK2111` an `async { }` that
starts no task (D9); `NK2112` `limit:` on a form that starts one task (D15);
`NK2115` `async` before a function's declaration (D5); `NK2118` a mention after
a `catch` where the failure arrived took it, without a fallback (D22); `NK2119`
(warning) a `par_iter` lambda starts to pause (D32); `NK2120` `async` inside a
loop's body (D30); `NK2121` a deferred handle not started, or started twice, on
some path (D12); `NK2122` a child handle that leaves its group (D11). `NK2109`
is asked of a `join(within:)` as D17 says. `NK2113`, `NK2114`, `NK2116` and
`NK2117` are retired and not reused.

### D34 - The names are final.

The keywords `async` and `sync` stay. The options `handle`, `deferred`,
`detached` and `on_timeout`, the type `task::ChildHandle` and `h.start()` are
final, as are `par_iter`, `Par`, `piece:` and `--overlaps`.

The word *async* names the keyword and nothing else: a text about pausing says
*pause*. Part I 8.1's heading is "Pausing Without Marking Functions"; Part II
11.1's is "Implicit Pausing and the Scheduler". Part II 12.4 keeps "Racing
Tasks": a `select` arm is a task (D1). Part II 12.7 keeps "Work That Borrows:
`async`".

## 5. Consequences

* One kind of task and one word that starts it. A reader finds every task by
  `async`; whether it outlives its group is `detached`, written where it
  starts. The forms this replaces - the `overlap` block, the scope lambda with
  its handle, the `async` loop, a cancel function beside `h.cancel()`, and,
  once the move is built, `spawn` - have no place in the language.
* The common case, a value computed beside other work, has no handle, no name
  it does not want and no method to call.
* A function's signature still says nothing about pausing; that is unchanged
  on purpose.
* A `par_iter` lambda that prints or writes another shared resource is refused
  (`NK2104`, D26); the way out is the value, printed after the call, or
  `limit: 1`.
* A task's full block is not a panic: `task::OverBudget` reaches its receiver,
  and the panic hook stays for bugs.
* **What it costs:** the window, wait and group analysis in the checker; a
  value waited for before a call or an operation, not only before a statement;
  two handle types, `ChildHandle` and `TaskHandle`, because one borrows and one
  does not; a group runtime with a limit and admission; two sites more on an
  error that crosses; a name that can throw (D22); every `spawn` in `tools/`,
  tests and examples rewritten (mechanically).
* **A silent performance cliff, named.** One call in a `par_iter` lambda that
  may pause - a log line that flushes, a cache miss that reads a file - turns a
  chain over the cores into tasks that take turns on one thread (D20).
  `NK2119` and `--overlaps` make it visible; they are not a refusal.

## 6. What this does not decide

* A pending value or a handle as a `select` arm.
* Cancelling the other tasks of a group at the first failure (ADR-292 §6
  stands, as an option, if it comes).
* Naming any group other than the enclosing one and the process; `detached`
  names only the process.
* Whether a pausing child task may run on the pool instead of its group's
  thread (ADR-328 D10's limit); it would shrink the cliff of §5.
* `order: Any` on `map` and `collect`; results keep input order (D26).
* What a full block does in `main` beyond what an uncaught error does.

## 7. Where it is specified

Part I 5.3 (a lambda after `async`), 5.4, 7.1 (started and arrived sites), 8.1,
8.1.2 (the model, the forms, the group, the check, failures, admission), 8.2,
8.3, the reserved words; Part II 11.2, Chapter 12 (the memory budget,
`task::OverBudget`, admission), 12.2, 12.4, 12.5, 12.6, 12.7; Part III 13.3b,
17.1's `std::task`, the `@detached` row, C.5, the `NK2101`-`NK2122`, `NK2203`,
`NK2209` and `NK1132` rows, and `--overlaps`.
