# ADR-async-1: `async` Starts a Child That Runs Beside the Code After It and Is Waited For Where Its Name Is First Used; `overlap` and `task::scope` Are Removed

**Status:** Accepted (D1-D8, D10, D12, D13, D16, D18, D19 changed or withdrawn by [ADR-async-2](adr-async-2.md), [ADR-async-3](adr-async-3.md) and [ADR-async-4](adr-async-4.md); the notes under each say what holds)
**Date:** October 30, 2026 (owner's decisions in two design rounds with the main session, [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983))
**Answers:** how a program asks for work to run beside the code that follows it, for one value, for a list of values and for work with no value, without coloring functions and without `await`; and what becomes of `overlap { … }` and `task::scope`, which ask for the same thing in two shapes
**Supersedes / Related:**
[ADR-292](adr-292.md) D2, D3, D4, D9, D15, D16 and §6 (changed: the joining form is `async`; D1, D5, D6, D8, D10-D14 and D17-D20 stand and are read for `async` where they said `overlap`);
[ADR-346](adr-346.md) D7 (changed: `task::scope` is gone; where a group is left by a failure, that failure leads, D9);
[ADR-327](adr-327.md) D5-D7 (D6 changed: a full block ends a task or a child with `task::OverBudget`, D15; D5 and D7 hold for a child as for a task);
[ADR-326](adr-326.md) D2, D3 (admission by an exchangeable strategy, and the ceilings the runtime enforces, taken for children, D14);
[ADR-328](adr-328.md) D8, D10 (changed: what they say of a scope's tasks holds for a group's children);
[ADR-276](adr-276.md) D13 (stands for every loop but one: `async let xs = for …` has a value, D3) and D16 (a child is one more boundary no jump leaves);
[ADR-277](adr-277.md) D16 (a child is a block, not a callable, as an `overlap` branch was);
[ADR-lambdas-1](adr-lambdas-1.md) D2 (`task::scope` leaves the list of once-parameters);
[ADR-298](adr-298.md) D1, D7, D8 (`async` is reserved because a construct uses it; `overlap` is released);
[ADR-288](adr-288.md) D1 (`sync`; a body holding an `async` form pauses);
[ADR-055](adr-055.md) (a pause is a suspension point; `async` in the emitted Rust is the language below's and unrelated to the word here);
[ADR-280](adr-280.md) D21 (the `secondary` list is a diagnostic);
[ADR-297](adr-297.md) D4, D5 (a cleanup's failure attaches; parked cleanups);
[ADR-005](adr-005.md) §1 Group B (the crossing check a child does not meet);
[ADR-037](adr-037.md), [ADR-303](adr-303.md) (`user_parallelism`; I/O the runtime performs is not user code);
[ADR-235](adr-235.md) (`par_iter`, the form for computation over data, apart);
Part I 3.3, 5.3, 7.1, 8.1, 8.1.2, 8.3; Part II Chapter 12, 12.2, 12.4, 12.6, 12.7; Part III 13.3b, 17.1's `std::task`, C.4, C.5, the `NK21xx` and `NK22xx` rows

## 1. Question

**What does a program write to run work beside the code that follows it - one
value, one value per element of a list, or work with no value - and wait for it
without an `await`?**

Today it writes one of two forms. `overlap { a(); b() }` runs two to eight
expressions and hands back their tuple; a branch may not bind a name, and the
number of branches is fixed in the source. `task::scope fn(s) { for u in us {
s.spawn fn { fetch(u) } } }` runs any number of tasks, but its tasks hand back
nothing, it needs a lambda and a handle to say "here", and its check is weaker
(`NK2107` only; two tasks that both print are accepted). A program that wants
the pages of a list of URLs writes neither well. Two forms for one idea is the
cost [ADR-276](adr-276.md) warns about, paid without the benefit.

## 2. How others do it

All rows are from memory, not checked against a current manual.

| Language / tool | Its answer | What it costs there | Here |
| :--- | :--- | :--- | :--- |
| Swift | `async let x = f()`, used with `try await x`; task groups `withThrowingTaskGroup { g in g.addTask { … }; for try await r in g { … } }` | functions are colored (`async`, `await` at every call); an `async let` not awaited is cancelled at scope end and its error dropped; a group's results come in completion order; no built-in limit | the binding form is taken, without the coloring, without dropping errors, results in input order, a limit as an option |
| Kotlin | `coroutineScope { val a = async { f() }; a.await() }` | `suspend` colors functions; the first failure cancels the siblings; a limit is a `Semaphore` or `limitedParallelism` | structured scope taken; cancelling at the first failure not taken (ADR-292 §6) |
| Trio (Python) | a nursery: `async with open_nursery() as n: n.start_soon(f)`; all failures as one `ExceptionGroup` | a handle to pass around; values come back by side effect | every failure kept, as here; the handle not needed |
| Go | `errgroup.Group`, `g.Go(func)`, `g.SetLimit(n)`, `g.Wait()` returns the first error | no values; captures unchecked, races found at run time if at all | `limit` taken |
| Rust | `std::thread::scope(|s| s.spawn(..))`; `futures::join!`; `stream.buffered(n)` (order kept), `buffer_unordered(n)` | borrows checked for memory only; `buffered(n)` is the closest to a valued loop with a limit | borrowing taken; the check extends past memory (touch sets) |
| Java | `StructuredTaskScope` (`fork`, `join`, `ShutdownOnFailure`) | a scope object and a policy class | one form, no policy object |
| JavaScript | `Promise.all([...])`, `Promise.allSettled` | colored (`async`/`await`); `all` keeps the first rejection and loses the others | |

**What transfers:** Swift's binding form (the value has a name, the wait is
where the name is used) and its group for a loop; Go's and Rust's limit;
Trio's every-failure list. **What does not:** the coloring (`async fn`,
`await`), because pausing is inferred here (Part I 8.1, ADR-055) and a
function's callers do not change when its body starts to pause; Swift's
cancel-and-drop at scope end, because every error has a receiver (ADR-346 D1);
completion order, because the written order is the only reproducible one
(ADR-292 D5).

**Where an error crossed** (D17), also from memory:

| Language / runtime | What a failure in a child shows | Cost |
| :--- | :--- | :--- |
| C# / .NET | the trace is stitched across each `await`, with a line `--- End of stack trace from previous location ---` | captured on every throw |
| Swift | async frames in the debugger and in crash reports; a thrown error carries no site | none on the success path, little in the error |
| Java | an exception from a forked subtask is wrapped (`ExecutionException`) with the subtask's trace as the cause; the fork site is not named | a trace per exception |
| tokio | no async trace by default; `async-backtrace`, `tokio-console` add one by instrumenting | instrumentation on every poll |
| V8 | "zero-cost" async stack traces: `at async f` frames, only across `await` | none until an error is thrown |

What transfers: C#'s stitching, behind `NIKAIA_TRACE=1` as every trace here is,
and V8's rule that the success path pays nothing. The two sites a child adds,
where it was started and where its failure arrived, are text the compiler
writes, as the raise site already is (Part I 7.1).

## 3. Options, weighed

| Option | What a program writes | Why it lost, or won |
| :--- | :--- | :--- |
| **A - keep `overlap` and `task::scope`** | the two forms of §1 | **lost**: two forms, neither with a value per element; the scope's check is weaker than the block's; a lambda and a handle to say where the wait is |
| **B - `async let`, `async for`, `async { }`** | a word in front of the binding, loop or block that runs beside what follows | **won** (D1-D13): one word for one idea, the value has a name, a loop is a group; the word makes asynchrony visible in the source without coloring any function |
| **C - widen `overlap`** (bindings and loops inside the block) | `overlap { let a = f(); for u in us { … } }` | **lost**: inside the block a statement would be a branch, not a step, so the same text would mean two things by where it stands |
| **D - Swift as it is, with `await`** | `async let x = f()` … `await x` | **lost**: a wait marker the inference already knows is noise, and it would invite `await` on calls, the coloring this language refused |
| **E - a library form** (`task::group`, a future per child) | `let g = task::group(); g.add fn { … }` | **lost**: a lambda per child, no name per value, and the checker sees calls instead of a construct it can check as a set |
| **F - wait at the first mention inside an expression** (Swift's `await x`) | the same text | **lost** to the statement rule (D2): a wait in the middle of a statement would run half the statement, then pause; with the wait before the statement, a statement is never split, and the window is a list of whole statements the touch-set check compares |
| **G - the loop's value through a lambda** (`async let xs = vs.map fn(v) { … }`) | a `map` the word sees into | **lost**: `async` would have to know which calls run their lambda once per element; a `for` is syntax the checker already reads as iterations |

*Changed by [ADR-async-2](adr-async-2.md) D7: the `async` loop is removed, so the exception below is gone.*

The cost B pays, named: **a loop with a value.** Part I 3.3 and ADR-276 D13
say a loop is a statement and hands back nothing. `async let xs = for …` is the
one exception. It is bounded: the value is not a `break`'s (D3 refuses every
jump out of a child), it is the list of the iterations' results, and the
exception is visible in the first word of the line. Every other loop keeps D13.

## 4. Decision

### D1 - `overlap` and `task::scope` are removed; `async` is the structured form, and it colors nothing.

*Changed by [ADR-async-4](adr-async-4.md) D1 and [ADR-async-3](adr-async-3.md) D1, D5: `async` is a prefix of an expression, never before a `let` or a loop; what it starts is a task of its group, and it starts every task, `spawn`'s included.*

`async` is a reserved word (ADR-298 D1, D8: a word the language uses is
reserved, and only for its construct); `overlap` is released (ADR-298 D7).
`async` stands in front of a binding, a loop, a block or an expression
statement or a lambda it runs (D18), never in front of a function's
declaration: whether a function pauses is still inferred, there is no `await`,
and a caller does not change when a callee starts to pause. `async fn name(…)`
is refused (`NK2115`), and the message says that pausing is inferred. What `async` starts is a **child**; the region whose end waits for
it is its **group**.

### D2 - `async let x = e` starts one child at the binding and waits for it before the first statement that names `x`.

*Changed by [ADR-async-4](adr-async-4.md) D1, D2: the form is `let x = async e`; the statement rule for a binding's wait point stands, and an argument or an operand is waited for before its call or operation.*

```nika
async let user = db::load_user(id)
async let rights = db::load_rights(id)
let prefs = cache::load_prefs(id)      // runs while both loads are in flight
render(user, rights, prefs)            // waits for both, then runs
```

The child evaluates `e`. The statements after the binding run beside it: they
are its **window**. The window ends before the first statement of the
binding's block that names `x` anywhere in it, nested blocks, loops, branches
and lambdas included; the wait is there, whether or not the branch that names
`x` is taken. A binding nothing names is waited for at the end of its group
(D5). A `let` pattern (ADR-291) and a written type are taken as `let` takes
them; the first statement naming any of the names waits. A mention inside
another child is not a wait of the main line (D7).

### D3 - `async let xs = for v in vs { body }` is a group with a value: each iteration is a child, and the value is the list of their results in input order.

*Withdrawn by [ADR-async-2](adr-async-2.md) D7: the `async` loop is removed; fan-out over a list is `let xs = async vs.par_iter().map fn(v) { … }.collect()`, and no loop has a value. The paragraph on jumps stands for every task: a jump that would leave one is `NK1132`.*

```nika
async(limit: 8) let pages = for url in urls { http::get(url) }
log(f"{urls.len()} requests started")
for page in pages { index(page) }      // waits for all of them
```

The value is a `Vec[T]`, where `T` is the type of the body's last expression,
in the order of the elements, whatever order the iterations end in. It is
waited for as D2 says. A body with no value is refused (the statement form,
D4, is the way out). Only `for … in` takes this form.

**A child is a boundary no jump leaves.** A `break`, `continue` or `return`
in a child that would leave it - an iteration of an `async` loop, the
expression of an `async let`, an `async` statement - is `NK1132`, as it is out
of an `overlap` branch today (ADR-276 D16).

### D4 - `async e` and `async for … { … }` as statements start children with no value, waited for at the end of their group.

*Changed by [ADR-async-2](adr-async-2.md) D7, D8: `async for` is removed; `async e` as a statement stands ([ADR-async-4](adr-async-4.md) D7), and an `async` inside a loop's body is `NK2120`.*

```nika
async for file in files { compress(file) }
async audit::log(event)
report()                               // runs beside every compress and the log
```

A value `e` hands back is dropped, as an expression statement's is.

### D5 - `async { … }` is a group, and its `}` waits for every child started inside it.

*Changed by [ADR-async-4](adr-async-4.md) D3 and [ADR-async-2](adr-async-2.md) D8: `NK2110` covers every task with nothing beside it, and its help removes the word; the loop case is `NK2120`. The capture rule is [ADR-async-3](adr-async-3.md) D2's.*

```nika
async {
    for dir in dirs {
        async for f in fs::list(dir) { scan(f) }
    }
}                                      // every scan has ended here
```

The block's own statements run on the main line, in order; it starts nothing by
itself. A child started anywhere inside it - in a loop, an `if`, a nested block
that is not itself a group - belongs to the innermost `async { }` around it.
It is a block expression: its value is its last expression's, available once
every child has ended. It replaces `task::scope fn(s) { s.spawn … }`.

Where no `async { }` encloses a child, its group is the innermost block that
holds the `async` form. A child whose window is empty - nothing runs between
its start and its wait point or its group's end, as `async fetch(u)` alone in a
loop's body, where each iteration would wait for its own child - is refused
(`NK2110`), and the help writes `async for`. An `async { }` that starts no
child is refused (`NK2111`): it waits for nothing. A child takes with it, as a
task does (Part I 8.3), what was bound inside its group in a block that ends
before the group does, an iteration's own names among them; what was bound
outside its group it borrows.

### D6 - Every form takes the options `limit:`, `within:` and `admission:`, written after the word.

*Changed by [ADR-async-4](adr-async-4.md) D7 and [ADR-async-2](adr-async-2.md) D4: the loop forms are gone; `limit:` stands on `async { … }` and on the `par_iter` chain, where it replaces `workers:`.*

`async(limit: n, within: d) let …`, `async(limit: n) for …`,
`async(within: d) { … }`: options without a positional zone, as
`join(within:)` (ADR-346 D4).

* **`limit: i64`** - at most `n` children of this form are in flight at once; the
  others wait for a place, unstarted, in written and input order. A child is
  under every `limit` around it. A value below `1` panics, naming the option,
  as `workers:` does (Part II 12.6). It is refused on a form that starts one
  child (`async let x = e`, `async e`; `NK2112`). Without it, no number
  applies: admission decides (D14).
* **`limit: 1`** written as the literal runs the children one at a time, in
  order, in the background. Children that never run at once cannot meet, so
  they are not checked against each other (D8); each is still checked against
  the window.
* **`within: time::Duration`** - counted from where the form starts. When it
  runs out, every child of the form still running is cancelled (D10's
  mechanism) and the wait throws `task::TimedOut`: at the first statement
  that names the binding, or at the group's end for a statement form and a
  block.
* **`admission: task::Admission`** - the strategy of D14 for this form.

### D7 - A pending binding named inside another child is waited for in that child.

*Changed by [ADR-async-4](adr-async-4.md) D2: read `let b = async enrich(a)` for `async let b = enrich(a)`; the rules stand.*

```nika
async let a = load(id)
async let b = enrich(a)                // b's child waits for a; the main line does not
show(b)
```

Ownership follows the rules every other use follows, applied to the child and
its window:

* **Both read:** `b`'s child reads `a`, and the main line names `a` later. Two
  reads meet on nothing; the main line waits for `a` where it first names it,
  not for `b`. The value lives in `a`'s block and outlives both.
* **The window changes it:** a `mut` method, an assignment or a `mut` argument
  on `a` in the window of a child that names `a` is `NK2104`, naming `a`. After
  the main line has waited for that child, it is allowed.
* **The child moves it:** `b`'s child takes `a` by value. `a` is handed to `b`'s
  child where `b` starts; a later mention of `a`, on the main line or in
  another child, is a use after the hand-over (`NK2105`), and the main line
  never waits for `a`.
* **A diamond:** `async let b = f(a)` and `async let c = g(a)` both wait for
  `a` and read the one value. If either moves `a`, the other's mention is
  refused as above.
* **A failure:** where `a` fails, its error is received by the first mention of
  `a` in written order, as a callee's error is its caller's (D9); every other
  child that waits for `a` cannot go on and is cancelled. One error, one
  receiver (ADR-346 D1).

### D8 - The touch-set check runs between a child and its window, and between the children of a group.

*Changed by [ADR-async-2](adr-async-2.md) D3 and [ADR-async-4](adr-async-4.md) D3: the iteration cases are asked of a `par_iter` lambda; the check runs between whatever runs beside, arguments and operands included.*

ADR-292 D3 carries over, pair by pair: a child against every statement of its
window, another child's start included; an iteration of an `async` loop
against the other iterations, which is the body against itself. A pair that
meets on a resource is `NK2104`, and the message names it: an `async for`
whose body writes stdout is refused, because two iterations would write it at
once. A child the compiler cannot account for is not refused (Part III C.4). A
child may borrow, because nothing outlives its group (ADR-292 D4); the
crossing check of a `spawn` has nothing to do. A binding the window changes
and a child names is `NK2104` (D7). An iteration that changes a name bound
outside it is `NK2107`, as a `par_iter` lambda is. Children of a `limit: 1`
form are not checked against each other (D6).

### D9 - A failure is received by whoever reads the child; an unread one waits for the group's end, and every one is kept.

A child's failure is its outcome, received where its binding is read (D16).
A failure nobody reads - a statement form, a binding
nothing names, a mention in a branch not taken - is received at the end of the
group:

* **The group ends normally** (its last statement, a `return`, a `break`): the
  first unread failure in written order - start order; for an `async` loop,
  input order - is thrown there, and the others travel as its `secondary`, in
  written order (ADR-292 D5, D9; ADR-280 D21). A `return` whose group has an
  unread failure throws instead of returning.
* **The group is left by a failure** (a `throw`, a `?`, a failed read): the
  group first waits for every child, then that failure leaves, and every
  unread failure is attached to it as its `secondary`, in written order. This
  places the leaving error first, where ADR-346 D7 put a scope's own error
  last: the error the code chose to throw is the one its handlers are written
  for, and the order is still the source's.

Early exit waits; it does not cancel. A child that panics cancels its siblings,
and the group throws `task::Crashed` where it ends (ADR-328 D8); a child whose
block is full fails only itself (D15). A binding's fallback (D16) runs for an
unread failure at the group's end. A cleanup's
failure attaches as ADR-292 D10 says.

### D10 - `task::cancel(x)` cancels a pending binding, and takes it.

*Withdrawn by [ADR-async-4](adr-async-4.md) D10: `task::cancel` is removed and `NK2113`, `NK2114` are retired; a task is cancelled only through its handle, `h.cancel()`; the value form cannot be cancelled.*

`std::task::cancel` takes a pending `async let` binding - one child, or a whole
`async let … = for` group - and is the one mention of it that does not wait.
The child stops at its current pause and its values are torn down (ADR-292
D13); its outcome is dropped, a failure included, as `h.cancel()` drops a
task's (ADR-292 D14). It takes the binding as `cancel()` takes a handle: a
mention after it, on any path, is refused (`NK2114`). The group's end waits until a
cancelled child has stopped, its cleanups included, because the child may
borrow from the group (D8); a pausing cleanup is not adopted by the runtime as
a `select` loser's is. An argument that is not a pending `async let` binding is
refused (`NK2113`). It is a function and not `x.cancel()`, because `x` names a value of
the result's type, which may have a `cancel()` of its own.

`select` stays a race (Part II 12.4). Whether a pending binding may be a
`select` arm is *not decided here*.

### D11 - How a group runs is ADR-292 D6 and Part II 12.7, unchanged.

* A child that never pauses (`sync`, Part II 12.1) runs on the pool where
  children run in parallel; at `user_parallelism = no` it runs when the main
  line next pauses or waits, so that every pausing child written before that
  point is started first (ADR-292 D6). The setting changes the duration, not
  the meaning, because D8 has shown the pair meets on nothing.
* A child that may pause is polled by its group, on the group's thread, and
  never handed to the runtime (ADR-328 D10), so it cannot outlive what it
  borrows.
* A body holding an `async` form pauses and is not `sync` (ADR-292 D16): a
  written `sync` on it is `NK2202`, and a `par_iter` lambda holding one is
  `NK2209`.
* An `async` form inside an open lock is `NK2203`, as a scope was: its
  children run while the lock is held.

### D12 - `--overlaps` keeps its name and reports children.

*Changed by [ADR-async-2](adr-async-2.md) D5: it also reports, per `par_iter` call, the execution chosen and why.*

It lists each child, its window (the first and last line), where it is waited
for and its group's end, and which children are started first (D11). The name
says what it reports - what runs beside what - and no keyword.

### D13 - The concurrency forms are five, each for one need.

*Replaced by [ADR-async-3](adr-async-3.md) D4: every form is a task's group with a policy; `spawn` is `async(detached)`.*

| Form | For | Outlives the code that starts it | Borrows |
| :--- | :--- | :--- | :--- |
| `async` | work beside the code that follows, every result and failure kept | no | yes |
| `spawn` | a task with a handle, or detached | yes | no, it moves (Part I 8.3) |
| `select` | the first of several to finish | no | yes |
| `par_iter` | computation over data, on the cores | no | yes, reads only |
| channels | values between tasks | - | - |

`async` and `select` are the pair ADR-292 D15 named: both start every child;
`async` keeps every result, `select` the first.

### D14 - Admission, not a number, decides when a child starts.

Each child allocates from a memory block of its own, as a task does under
[ADR-327](adr-327.md) D5: for ownership and borrowing it is not a task (D8),
for memory it is. Without `limit:`, a strategy decides: a value implementing
`task::Admission`, with the shape of [ADR-326](adr-326.md) D2's
`net::Admission` (`decide(ref mut self, load: task::Load) -> task::Decision
sync`, on the runtime's beat). `std` provides the default; a package may
provide another; `admission:` names one for a form. A new child starts only
where its block fits the budget of the task that runs the group and a
descriptor is free; otherwise it waits, unstarted, until a sibling ends.
`limit: n` caps on top of the strategy. What no strategy can change, the
runtime enforces (ADR-326 D3): never more descriptors than the system allows,
and at the ceiling a child waits; it never fails for it.

**What this guarantees:** no crash from fan-out, and none from memory a child
allocates through Nikaia. **What it does not cover:** memory a Rust crate or a
C library keeps (ADR-327 D8), other processes, the kernel's buffers.

**Rejected:** a default number, unbounded or `task::cores()` - unbounded fails
at the system's ceiling at run time, and the cores are the wrong measure for
children that wait; a configured key - ADR-326 D2 refused one for connections
for the same reasons.

### D15 - A full block ends a task or a child with `task::OverBudget`, and only it.

*Changes [ADR-327](adr-327.md) D6 for tasks.*

A full block unwinds the task or the child like a panic, from the allocation
site, and its cleanups run. At every boundary it arrives as its own error,
`task::OverBudget`, beside `task::Crashed`: `h.join()` throws it, an `async
let` hands it on as any failure of its child (D16), a supervisor counts it in
its child's failures, and `http` under `serve` answers `503` where the
response has not begun. In a group only a panic, a bug, cancels the siblings;
`task::OverBudget` fails only its child. The panic hook does not run for it; it
is reported through the boundary's error, and for an unbound `spawn` on
standard error (ADR-346 D6). Inside the task or the child it cannot be caught:
an allocation declares no `throws`. A shared value whose block is full is
still poisoned (ADR-327 D6).

**Why its own error:** a full block is load, not a bug; a receiver that can
tell them apart answers `503` or tries later where a panic would be reported
as a defect, and a group that cancelled the siblings of a child that only ran
out of room would turn one refusal into many.

### D16 - A failure is handled in the child, at the use, or at the declaration, in that order.

*Changed by [ADR-async-4](adr-async-4.md) D11: the levels hold for every form - in the task, where the failure arrives (a mention or a `join`), the fallback written after the `async` expression, the receiver; a cancellation runs no fallback.*

```nika
async let page = http::get(url) catch { Page::empty() }   // fallback, at the declaration
let size = page.len() catch { 0 }                          // a catch at the use
```

1. **In the child.** A `catch` inside the child's expression or lambda is the
   child's own and runs at once, when the child fails.
2. **At the use.** Otherwise the failure is held; no handler runs. At the wait
   point it arrives where the mention of `x` that runs is evaluated, and a
   `catch` that encloses that mention and lies inside the binding's block
   handles it.
3. **At the declaration.** Otherwise the `catch` written directly after the
   initializer of the `async let` - the binding's **fallback** - runs, late,
   where the failure arrived; its value becomes `x`'s, and the statement goes
   on with it.

With none of them the failure leaves from the wait point. Once a `catch` at a
use has taken the failure, a later mention of `x` on that path is refused
unless the binding has a fallback (`NK2118`), because `x` has no value there.
The `throws` walk: the function throws what the fallback throws, or the
child's whole set where there is no fallback.

**What it costs, named:** a name can throw - `page.len()` above fails without a
call that fails being written there, toward a `catch` at the use; and which
handler runs depends on the path that reaches the mention, so a reader looks
in three places. The precedence is fixed and the three places are lexical,
which is what keeps it readable.

### D17 - An error that crosses an `async` boundary records where the child was started and where the failure arrived.

Beside its raise site, such an error records a **started** site (the `async`
form) and an **arrived** site (the wait point), as text the compiler writes,
like the raise site, so the success path costs nothing. A chain of bindings
gives one started and arrived pair per crossing, in order; each `secondary`
error carries its own. The long form shows `raised`, `started` and `arrived`
lines. Under `NIKAIA_TRACE=1` the child's trace and the waiting function's are
stitched, with a line `── started at <site> ──` between them, per crossing.

### D18 - A child of several steps is a lambda, run where it stands.

*Changed by [ADR-async-4](adr-async-4.md) D9: written `let x = async fn { … }`.*

`async let x = fn { … }` and `async fn { … }` run the lambda's body as the
child; `return` in it ends the child with its value, as in `spawn fn { … }`.
`async { … }` stays the group (D5). A bare block would leave `return` and
`break` without a meaning; the lambda body settles them. **The cost:** a lambda
literal directly after `async` is run, not bound; a child whose value is a
function is written `async let f = fn { fn { … } }`.

### D19 - The refusals have codes of their own.

*Changed by [ADR-async-4](adr-async-4.md) D12: `NK2113`, `NK2114`, `NK2116`, `NK2117` are retired; `NK2119`-`NK2122` are added.*

`NK2110` a child with an empty window; `NK2111` an `async { }` that starts no
child; `NK2112` `limit:` on a form that starts one child; `NK2113`
`task::cancel` of what is not a pending binding; `NK2114` a mention after
`task::cancel`; `NK2115` `async` before a function's declaration; `NK2116`
`async let xs = for` whose body hands back nothing; `NK2117` `async` before a
loop that is not `for … in`; `NK2118` a mention after a use-site `catch` took
the failure, without a fallback. `NK2104` loses its case for a branch that
binds; the `overlap` arity refusal goes with `overlap`.

## 5. Consequences

* Five fewer things to learn: no `overlap` block, no tuple of two to eight, no
  scope lambda, no scope handle, no `s.spawn`. One word, four positions.
* The word `async` appears where work runs beside the code after it, and
  nowhere else; reading a function's signature still says nothing about
  pausing, and that is unchanged on purpose.
* A program that printed from several `task::scope` tasks no longer compiles
  as an `async for`: two iterations meet on stdout. The way out is the value
  (`async let lines = for …`, then print), or `limit: 1`.
* An `async { }` whose statements a reader takes for background work and that
  starts no child is refused (`NK2111`); a child of several steps is a lambda
  (D18).
* A task's full block is no longer a panic: `task::OverBudget` reaches its
  receiver, and the panic hook stays for bugs.
* **What it costs:** one reserved word for one released; the window, wait and
  group analysis in the checker; a loop with a value; `task::cancel` as a
  function the checker knows by name; `std`'s vehicles per arity
  (`overlap<n>`) replaced by a group runtime with a limit and admission; two
  sites more on an error that crosses; a name that can throw (D16).

## 6. What this does not decide

* A pending binding as a `select` arm.
* Cancelling the other children at the first failure (ADR-292 §6 stands, as an
  option, if it comes).
* A group whose children are added while it runs from a value that arrives
  later (a stream); `async for` takes a list known where it starts.
* `async` in front of `while` (refused, `NK2117`).
* An option on `async` that puts a child in another group than D5's.
* What a full block does in `main` beyond what an uncaught error does.

## 7. Answered (owner, 2026-10-30)

The first round left five details open; the owner answered them in a second:
the default of `limit` (no number: admission, D14, with D15 added); the group
of a bare `async` (the innermost block, with `NK2110`, D5); where a failure is
thrown (three levels, D16, with D17); which error leads (the leaving one, D9);
a child of several steps (a lambda, D18).

## 8. Where it is specified

Part I 5.3 (a lambda after `async`), 7.1 (started and arrived sites), 8.1.2
(the forms, the group, the check, failures, `task::cancel`, admission), the
reserved words; Part II Chapter 12 (the memory budget, `task::OverBudget`,
admission), 12.2, 12.4, 12.7; Part III 13.3b, 17.1's `std::task`, C.5, the
`NK2104`, `NK2107`, `NK2110`-`NK2118`, `NK2203`, `NK2209` and `NK1132` rows, and
`--overlaps`.
