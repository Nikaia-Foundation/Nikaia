# ADR-async-4: `async` Is an Expression Prefix; `async e` Is Waited Where Its Value Is First Needed, `async(handle)` Gives a Handle Bound to the Group, and Only a Handle Is Cancelled

**Status:** Accepted
**Date:** October 30, 2026 (owner's decisions in the third design round with the main session, [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983))
**Answers:** once one word starts every task ([ADR-async-3](adr-async-3.md)) and fan-out over a collection is `par_iter`'s ([ADR-async-2](adr-async-2.md)), what does a program write to start a task, where is it waited for, how is it cancelled, and which handler meets its failure?
**Supersedes / Related:**
[ADR-async-1](adr-async-1.md) D1, D2, D4, D6, D7, D10, D16, D18, D19 (changed: `async` is an expression prefix, there is no `async let`, `task::cancel` is removed, the failure levels hold for every form; D5, D8, D9, D11, D12, D14, D15, D17 stand and are read for the forms here);
[ADR-async-3](adr-async-3.md) D5 (one word starts every task; this record writes its options);
[ADR-async-2](adr-async-2.md) D7, D8 (no `async` loop; `async` starts only children whose number stands in the source);
[ADR-346](adr-346.md) D1-D6 (every error has a receiver; a free handle is taken; `join(within:)`; detached tasks; they hold for `async(detached)`; D3, D4 changed by D14: a timed-out `join` keeps the task and the handle unless `on_timeout: cancel`);
[ADR-292](adr-292.md) D3, D13, D14 (touch sets; one cancel mechanism; `cancel()` takes the handle);
[ADR-327](adr-327.md) D5 (a block per task);
[ADR-328](adr-328.md) D10 (a pausing child is polled by its group);
[ADR-298](adr-298.md) D1, D7, D8 (reserved words; `spawn` is released once the migration is built);
[ADR-055](adr-055.md) (pausing is inferred; nothing is colored);
Part I 5.3, 5.4, 7.1, 8.1, 8.1.2, 8.2, 8.3, the reserved words; Part II 11.2, Chapter 12, 12.4, 12.5, 12.7; Part III 17.1's `std::task`, C.5, the `NK21xx` rows, `--overlaps`

## 1. Question

ADR-async-1 put the word in front of a statement: `async let x = e`, `async for`,
`async e`, `async { }`. Two later answers moved the ground under it. ADR-async-2
removed the `async` loop, so the loop forms are gone; ADR-async-3 made `spawn`
an option of the one word, so a task with a handle, one that does not run yet
and one that outlives its group must be said with `async` too. And
`task::cancel(x)` existed only because a binding cannot carry a method: a
second cancel spelling beside `h.cancel()`.

**Where does `async` stand, what does each of its forms hand back, and where
does each meet its failure?**

## 2. How others do it

All rows are from memory, not checked against a current manual.

| Language / tool | Its answer | What it costs there |
| :--- | :--- | :--- |
| Swift | `async let x = f()`, read with `await x`; `Task { }` and `Task.detached { }` give a handle with `.value` and `.cancel()` | the binding form is a statement; colored functions; an `async let` not awaited is cancelled at scope end |
| Kotlin | `async { f() }` is an expression yielding a `Deferred<T>`; `launch` a `Job`; `CoroutineStart.LAZY` starts it on `start()` | a deferred value is a handle, read with `await()`; `suspend` colors |
| C# | `Task.Run(…)` is an expression; `await` at the use | every task is a heap object; colored |
| Rust | `tokio::join!(a, b)`; `spawn` → `JoinHandle` with `abort()`; `std::thread::scope` for borrowing | the borrowing form and the free one are two APIs |
| Futhark, Haskell `par` | `a \`par\` b \`pseq\` f a b`: sparks evaluated beside the expression that uses them | no handle, no cancel; only for pure values |

**What transfers:** Kotlin's expression form and its lazy start; Swift's split
of a child handle from a detached one; the spark's point that a value started
beside its use needs no handle at all. **What does not:** the coloring, and
Swift's cancel-at-scope-end, which drops an error (ADR-346 D1).

## 3. Options, weighed

| Option | What a program writes | Why it lost, or won |
| :--- | :--- | :--- |
| **A - keep the statement prefix** (`async let`, `async e`) and add `async(detached) let h = …` | the word before the statement | **lost**: a binding is the only place a value can be started, so `f(async g(), async h())` needs two names; the handle forms would sit before a `let` whose value is not what the `let` binds |
| **B - an expression prefix** | `let x = async e`, `f(async g(), async h())`, `let h = async(handle) fn { … }` | **won** (D1-D12): one position, the right of `=` or an argument or an operand; the value form needs no name; a handle is what the expression hands back, so the `let` binds what it says |
| **C - every form hands back a handle** (Kotlin's `Deferred`) | `let x = async g()` … `x.join()` | **lost**: a `join()` at every use is the `await` this language refused, and the common case pays for the rare one |

## 4. Decision

### D1 - `async` is a prefix of an expression, and there is no `async let`.

`async` stands where an expression stands: on the right of a `=`, as an
argument, as an operand, or as an expression statement. It never stands before
a `let`, a loop or a function's declaration (`NK2115`, ADR-async-1 D1). What it
starts is a **task** of its group (ADR-async-3 D1); the forms below differ in
what they hand back. Options are written in parentheses directly after the
word, without a positional zone: `async(limit: 4) { … }`, `async(detached) fn { … }`.

The names are final (D13).

### D2 - `async e` starts at once and hands back the value, waited for where the value is first needed.

```nika
let user = async db::load_user(id)
let rights = async db::load_rights(id)
let prefs = cache::load_prefs(id)      // runs while both loads are in flight
render(user, rights, prefs)            // waits for both, then runs
```

The wait point is where the value is first needed:

* **A binding** (`let x = async e`): before the first statement that names `x`,
  by ADR-async-1 D2's statement rule, nested blocks, branches and lambdas
  included. A binding nothing names is waited for at its group's end.
* **An argument**: before the call that receives it. `f(async g(), async h())`
  runs `g` and `h` beside each other and calls `f` when both are done;
  `f(async g(), h())` runs `g` while `h()` computes on the main line.
* **An operand**: before the operation. `let t = async a() + async b()` runs
  both and adds.
* **A tuple**: `let (a, b) = (async g(), async h())` is what `overlap { g();
  h() }` was; each part is waited for where its name is first named.

There is no handle, so the value form cannot be cancelled (D10). The callee of
`f(async g())` never sees a pending value: it gets a plain `T`, so no function
is colored and no signature changes.

### D3 - A task with nothing beside it is refused, and what runs beside is checked.

An `async e` whose window is empty is `NK2110`, ADR-async-1 D5's case made
general: `f(async g())` with no other argument computed beside it, `let x =
async g()` named in the very next statement, `async g()` as the last statement
of its group. The help removes the word.

Tasks start in the written order, left to right, and the main line goes on
after each start. The touch-set check (ADR-async-1 D8) runs between what runs
beside: between the tasks of one call's arguments and the arguments computed
on the main line, between the operands of one operation, and between a bound
task and every statement of its window. `NK2104` names the resource.

### D4 - `async(handle) fn { … }` starts at once and hands back a handle bound to the group.

```nika
let h = async(handle) fn { crawl(site) }
index_local()
let pages = h.join() catch { [] }
```

The handle is a `task::ChildHandle[T]`. It borrows, as every task of a block
group does (ADR-async-3 D2): it needs no `Send` and no heap, because it points
at the task's slot in its group. It has `join()` and `cancel()`, both of which
take it, and no `detach()`: a child handle does not outlive its group. One that
leaves its group - returned past it, stored in a value that outlives it - is
`NK2122`. A child handle that is neither joined nor cancelled is waited for at
its group's end, and its failure goes to the group as an unread failure does
(ADR-async-1 D9); it is not `NK2109`.

### D5 - `async(deferred) fn { … }` hands back a handle that runs only from `h.start()`.

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

### D6 - `async(detached) fn { … }` hands back a free handle, which may outlive the group.

```nika
let w = async(detached) fn { watch(dir) }
w.detach()
async(detached) fn { log_forever() }     // unbound: detached at once
```

`detached` implies `handle`. The task is a free task (ADR-async-3 D1): its
group is the process, so it takes what it captures (Part I 8.3, `NK2101`). The
handle is a `TaskHandle[T]` and has `join()`, `join(within:)`, `cancel()` and
`detach()`; ADR-346 D2-D6 hold for it as they held for `spawn`: a bound handle
not taken is `NK2109`, an unbound one is detached and its error goes to
standard error unless the fallback (D11) handles it. `detached` and `deferred`
combine: `async(detached, deferred) fn { … }`. This is `spawn` (ADR-async-3 D5);
`spawn fn { … }` is written `async(detached) fn { … }`.

### D7 - `async e` as a statement starts a task without a value, waited for at its group's end; `async { … }` stays the group.

```nika
async {
    async audit::log(event)
    async metrics::count(event)
    report(event)                       // runs beside both
}                                       // both have ended here
```

A value the task hands back is dropped. The group is ADR-async-1 D5's: the
innermost `async { }` in the same function, else the innermost block that holds
the form; an `async { }` that starts nothing is `NK2111`. Its options are
`limit:`, `within:` and `admission:` (ADR-async-1 D6); `limit:` on a form that
starts one task is `NK2112`.

### D8 - `async xs.par_iter() … .collect()` is one task that fans out, beside the code after it.

```nika
let pages = async urls.par_iter().map fn(u) { http::get(u) }.collect()
log(f"{urls.len()} requests started")
for page in pages { index(page) }      // waits for every one
```

The prefix makes the whole chain one task of the enclosing group; the chain's
own elements are the tasks of the call's group (ADR-async-2 D1). This replaces
`async let pages = for url in urls { … }`.

### D9 - A lambda after `async` is run, not bound.

`let x = async fn { … }` and `async fn { … }` run the lambda's body as the task,
and `return` in it ends the task with its value (ADR-async-1 D18, read for the
expression form). A task whose value is a function is written `let f = async fn
{ fn { … } }`. A lambda that names an argument is `NK2103`: a task is handed
nothing.

### D10 - A task is cancelled only through its handle, by `h.cancel()`.

`task::cancel(x)` is removed, and its codes `NK2113` and `NK2114` are retired,
not reused. `h.cancel()` stops the task at its current pause, tears down its
values and drops its outcome, a failure included (ADR-292 D13, D14); it takes
the handle. The group's end waits until a cancelled child task has stopped, its
cleanups included, because it may borrow from the group. The value form `async
e` has no handle and cannot be cancelled; a program that may want to cancel
writes `async(handle) fn { e }`.

`within:` on any form cancels by itself: when it runs out, the form's tasks
still running are cancelled and `task::TimedOut` arrives where the value or the
join is. `within:` on the form is a deadline counted from its start
([ADR-async-3](adr-async-3.md) D7); `join(within:)` is a limit on that one wait
and cancels only with `on_timeout: cancel` (D14).

### D11 - A failure meets the same handlers in every form, in this order.

```nika
let page = async http::get(url) catch { Page::empty() }   // the fallback
let h = async(handle) fn { crawl(site) } catch { [] }     // the fallback of a handle
let n = h.join() catch { [] }                               // a catch where it arrives
```

1. **In the task.** A `catch` inside the task's expression or lambda is the
   task's own and runs at once, when the task fails.
2. **Where the failure arrives.** Otherwise the failure is held. It arrives at
   the mention that waits (the value form) or at `h.join()` and `t.join()` (a
   handle), and a `catch` that encloses that point handles it (ADR-async-1
   D16's rule 2, read for every form).
3. **The fallback.** Otherwise the `catch` written directly after the `async`
   expression runs: for the value form at the wait point, its value becoming
   the value; for a handle inside the join, its value becoming the join's; for
   a detached handle that is never joined, in the task itself when the task
   fails, instead of the error going only to standard error. It is the last
   handler before the receiver.
4. **The receiver.** Otherwise the failure goes on to the receiver: it leaves
   from the wait point or the join, or, unread, it is the group's at its end
   (ADR-async-1 D9); for a detached task nobody joins, standard error
   (ADR-346 D6).

A cancellation is not a failure: it runs no fallback. `task::OverBudget` is a
failure and runs it (ADR-async-1 D15). `NK2118` stands: a later mention after a
`catch` where the failure arrived took it, without a fallback, is refused. The
started and arrived sites (ADR-async-1 D17) are recorded for every form; for a
handle the arrived site is the `join`.

### D12 - The codes.

`NK2110` widens to every task with nothing beside it (D3). `NK2111`, `NK2112`,
`NK2115` and `NK2118` stand. `NK2113` and `NK2114` are retired with
`task::cancel` (D10); `NK2116` and `NK2117` with the `async` loop (ADR-async-2
D7). New: `NK2119` (warning) a `par_iter` lambda starts to pause (ADR-async-2
D5); `NK2120` `async` inside a loop's body (ADR-async-2 D8); `NK2121` a deferred
handle not started, or started twice, on some path (D5); `NK2122` a child handle
that leaves its group (D4). None reuses a retired number. `NK2109` is asked of a `join(within:)` as D14
says.

### D13 - The names are final.

*Owner's decision, 2026-10-30: the naming round is closed.*

The keywords `async` and `sync` stay. The options `handle`, `deferred`,
`detached` and `on_timeout` (D14), the type `task::ChildHandle` and
`h.start()` are final, as are the other names these records use (`par_iter`,
`Par`, `piece:`, `--overlaps`). `spawn` stays reserved only until the move to
`async(detached)` is built ([#991](https://github.com/Nikaia-Foundation/Nikaia/issues/991)),
as ADR-async-3 D5 says.

The word *async* names the keyword and nothing else: a text about pausing says
*pause*. Part I 8.1's heading is "Pausing Without Marking Functions"; Part II
11.1's is "Implicit Pausing and the Scheduler". Part II 12.4 keeps "Racing
Tasks": a `select` arm is a task (ADR-async-3 D1). Part II 12.7 keeps "Work
That Borrows: `async`".

### D14 - `h.join(within: d)` is a limit on this wait; with `on_timeout: cancel` it cancels the task.

*Owner's decision, 2026-10-30. Changes [ADR-346](adr-346.md) D3, D4.*

```nika
let h = async(detached) fn { fetch(url) }
let page = h.join(within: 2.seconds()) catch {
    match error {
        task::TimedOut => { println("slow"); h.join() }   // the task kept running
        else => throw error
    }
}
let quick = async(detached) fn { fetch(mirror) }
let other = quick.join(within: 2.seconds(), on_timeout: cancel) catch { "" }
```

`within:` on `join` is an option of the wait, not of the task: `d` counts from
the call. When it runs out:

* **By default** `join` throws `task::TimedOut`, the task keeps running, and the
  handle stays live: it can be joined again, cancelled, or, a `TaskHandle`,
  detached.
* **With `on_timeout: cancel`** the task is cancelled as `h.cancel()` cancels it,
  its cleanups run, `join` throws `task::TimedOut`, and the handle is used up.
  `cancel` is the option's one value.

Both handles take the option (`ChildHandle` and `TaskHandle`). Where the task's
own `within:` (ADR-async-3 D7) runs out first, `join` throws `task::TimedOut`
as well; a handle still live after it is joined again with the same error, or
cancelled, which does nothing more.

**`NK2109`.** A `join(within:)` without `on_timeout: cancel` takes a
`TaskHandle` only on its success path, and on a failure of the task's body. On
the path where it throws `task::TimedOut` the handle is still live, and must be
taken on every path out of the handler: in a `match error` arm for
`task::TimedOut`, in an `else` or binding arm that can match it, and in the
whole handler where it does not match on the error. A `join(within:,
on_timeout: cancel)` takes it on both paths. A `ChildHandle` left live is
waited for at its group's end, as D4 says, and is not `NK2109`.

**Why.** A limit on a wait is the waiter's: the waiter may want to do something
else and come back, and the task, which another part of the program may also
wait for, is not ended by one impatient waiter unless that waiter says so.

## 5. Consequences

* One word, one position. `async let`, `async for`, `overlap`, `task::scope`,
  `task::cancel` and, after the migration, `spawn` are gone; `async` and its
  three options say everything they said.
* The common case - a value computed beside other work - has no handle, no
  name it does not want, no method to call.
* A reader finds every task by the word; whether it outlives its group is
  `detached`, written where it starts.
* **What it costs:** a value can now be waited for in the middle of an
  expression's evaluation, before a call or an operation, not only before a
  statement (ADR-async-1 option F's concern); the wait is still before
  something the reader sees - the call or the operator - and its operands
  started left to right. Every `spawn` in `tools/`, tests and examples is
  rewritten (mechanically). Two handle types, `ChildHandle` and `TaskHandle`,
  because one borrows and one does not.

## 6. What this does not decide

*Answered (owner, 2026-10-30): from where `within:` counts (D14 here,
ADR-async-3 D7), the names and the headings (D13).*

* A pending value or a handle as a `select` arm (ADR-async-1 §6).

## 7. Where it is specified

Part I 5.3, 5.4, 7.1, 8.1, 8.1.2, 8.2, 8.3, the reserved words; Part II 11.2,
Chapter 12, 12.4, 12.5, 12.6, 12.7; Part III 17.1's `std::task`, the `@detached`
row, C.5, the `NK2101`-`NK2122` rows, `--overlaps`.
