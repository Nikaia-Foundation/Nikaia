# ADR-async-1: `async` Starts a Child That Runs Beside the Code After It and Is Waited For Where Its Name Is First Used; `overlap` and `task::scope` Are Removed

**Status:** Accepted
**Date:** October 30, 2026 (owner's decision in a design round with the main session; the details in §7 are open, [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983))
**Answers:** how a program asks for work to run beside the code that follows it, for one value, for a list of values and for work with no value, without coloring functions and without `await`; and what becomes of `overlap { … }` and `task::scope`, which ask for the same thing in two shapes
**Supersedes / Related:**
[ADR-292](adr-292.md) D2, D3, D4, D9, D15, D16 and §6 (changed: the joining form is `async`; D1, D5, D6, D8, D10-D14 and D17-D20 stand and are read for `async` where they said `overlap`);
[ADR-346](adr-346.md) D7 (changed: `task::scope` is gone; which error leads when a group is left by a failure is D9 here);
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
Part I 3.3, 8.1, 8.1.2, 8.3; Part II 12.4, 12.6, 12.7; Part III C.4, the `NK21xx` and `NK22xx` rows

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

The cost B pays, named: **a loop with a value.** Part I 3.3 and ADR-276 D13
say a loop is a statement and hands back nothing. `async let xs = for …` is the
one exception. It is bounded: the value is not a `break`'s (D3 refuses every
jump out of a child), it is the list of the iterations' results, and the
exception is visible in the first word of the line. Every other loop keeps D13.

## 4. Decision

### D1 - `overlap` and `task::scope` are removed; `async` is the structured form, and it colors nothing.

`async` is a reserved word (ADR-298 D1, D8: a word the language uses is
reserved, and only for its construct); `overlap` is released (ADR-298 D7).
`async` stands in front of a binding, a loop, a block or an expression
statement, never in front of a function: whether a function pauses is still
inferred, there is no `await`, and a caller does not change when a callee
starts to pause. `async fn` is refused, and the message says that pausing is
inferred. What `async` starts is a **child**; the region whose end waits for
it is its **group**.

### D2 - `async let x = e` starts one child at the binding and waits for it before the first statement that names `x`.

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

```nika
async for file in files { compress(file) }
async audit::log(event)
report()                               // runs beside every compress and the log
```

A value `e` hands back is dropped, as an expression statement's is.

### D5 - `async { … }` is a group, and its `}` waits for every child started inside it.

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

Where no `async { }` encloses a child, its group is the block that holds the
`async` form - *open: whether that is the innermost block or the function's
body, §7 Q2*.

### D6 - Every form takes the options `limit:` and `within:`, written after the word.

`async(limit: n, within: d) let …`, `async(limit: n) for …`,
`async(within: d) { … }`: options without a positional zone, as
`join(within:)` (ADR-346 D4).

* **`limit: i64`** - at most `n` children of this form are in flight at once; the
  others wait for a place, unstarted, in written and input order. A child is
  under every `limit` around it. A value below `1` panics, naming the option,
  as `workers:` does (Part II 12.6). It is refused on a form that starts one
  child (`async let x = e`, `async e`). The default is *open, §7 Q1*.
* **`limit: 1`** written as the literal runs the children one at a time, in
  order, in the background. Children that never run at once cannot meet, so
  they are not checked against each other (D8); each is still checked against
  the window.
* **`within: time::Duration`** - counted from where the form starts. When it
  runs out, every child of the form still running is cancelled (D10's
  mechanism) and the wait throws `task::TimedOut`: at the first statement
  that names the binding, or at the group's end for a statement form and a
  block.

### D7 - A pending binding named inside another child is waited for in that child.

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

A child's failure is its outcome, received where its binding is read (the
place is *open, §7 Q3*). A failure nobody reads - a statement form, a binding
nothing names, a mention in a branch not taken - is received at the end of the
group:

* **The group ends normally** (its last statement, a `return`, a `break`): the
  first unread failure in written order - start order; for an `async` loop,
  input order - is thrown there, and the others travel as its `secondary`, in
  written order (ADR-292 D5, D9; ADR-280 D21). A `return` whose group has an
  unread failure throws instead of returning.
* **The group is left by a failure** (a `throw`, a `?`, a failed read): the
  group first waits for every child, then that failure leaves, and every
  unread failure is attached to it as its `secondary`, in written order. *This
  places the leaving error first, where ADR-346 D7 put a scope's own error
  last; §7 Q4.*

Early exit waits; it does not cancel. A child that panics cancels its siblings,
and the group throws `task::Crashed` where it ends (ADR-328 D8). A cleanup's
failure attaches as ADR-292 D10 says.

### D10 - `task::cancel(x)` cancels a pending binding, and takes it.

`std::task::cancel` takes a pending `async let` binding - one child, or a whole
`async let … = for` group - and is the one mention of it that does not wait.
The child stops at its current pause and its values are torn down (ADR-292
D13); its outcome is dropped, a failure included, as `h.cancel()` drops a
task's (ADR-292 D14). It takes the binding as `cancel()` takes a handle: a
mention after it, on any path, is refused. The group's end waits until a
cancelled child has stopped, its cleanups included, because the child may
borrow from the group (D8); a pausing cleanup is not adopted by the runtime as
a `select` loser's is. An argument that is not a pending `async let` binding is
refused. It is a function and not `x.cancel()`, because `x` names a value of
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

It lists each child, its window (the first and last line), where it is waited
for and its group's end, and which children are started first (D11). The name
says what it reports - what runs beside what - and no keyword.

### D13 - The concurrency forms are five, each for one need.

| Form | For | Outlives the code that starts it | Borrows |
| :--- | :--- | :--- | :--- |
| `async` | work beside the code that follows, every result and failure kept | no | yes |
| `spawn` | a task with a handle, or detached | yes | no, it moves (Part I 8.3) |
| `select` | the first of several to finish | no | yes |
| `par_iter` | computation over data, on the cores | no | yes, reads only |
| channels | values between tasks | - | - |

`async` and `select` are the pair ADR-292 D15 named: both start every child;
`async` keeps every result, `select` the first.

## 5. Consequences

* Five fewer things to learn: no `overlap` block, no tuple of two to eight, no
  scope lambda, no scope handle, no `s.spawn`. One word, four positions.
* The word `async` appears where work runs beside the code after it, and
  nowhere else; reading a function's signature still says nothing about
  pausing, and that is unchanged on purpose.
* A program that printed from several `task::scope` tasks no longer compiles
  as an `async for`: two iterations meet on stdout. The way out is the value
  (`async let lines = for …`, then print), or `limit: 1`.
* An `async { }` whose statements a reader takes for background work runs them
  in order; the shape of a child of several steps is *open, §7 Q5*.
* **What it costs:** one reserved word for one released; the window, wait and
  group analysis in the checker; a loop with a value; `task::cancel` as a
  function the checker knows by name; `std`'s vehicles per arity
  (`overlap<n>`) replaced by a group runtime with a limit.

## 6. What this does not decide

* A pending binding as a `select` arm.
* Cancelling the other children at the first failure (ADR-292 §6 stands, as an
  option, if it comes).
* A group whose children are added while it runs from a value that arrives
  later (a stream); `async for` takes a list known where it starts.
* `async` in front of `while`.

## 7. Open questions (the owner's; [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983))

* **Q1 - The default of `limit`.** Unbounded (the main session's lean, and
  Swift's, Go's, `Promise.all`'s), or `task::cores()`. *Recommendation:*
  unbounded: a child that computes already runs on the pool, so computation is
  bounded by the cores without a limit, and a child that waits costs a slot of
  memory, not a core; `cores()` would hold eight downloads in flight on an
  eight-core machine, the wrong measure. The risk: a loop over a million URLs
  opens a million connections and fails with the system's limit on open
  files, at run time.
* **Q2 - The group of a bare `async`.** (a) The innermost block; (b) the
  function's body; (c) the innermost block, but a loop body does not count, so
  a loop's children wait at the block around the loop. *Recommendation:* (a),
  with a refusal for a child whose window is empty (`for u in us { async
  fetch(u) }`, where each iteration's group ends right after the child, so the
  iterations run one after the other), whose help writes `async for`.
* **Q3 - Where a failure of a pending binding is thrown.** (a) At the mention
  of `x`, though the wait was before the statement, so a `catch` around the
  mention takes it; (b) at the start of the statement, so only a `catch` around
  the whole statement takes it. *Recommendation:* (a).
* **Q4 - Which error leads when the group is left by a failure.** (a) The
  leaving error, the children's as its `secondary` (the owner's words in the
  round; changes ADR-346 D7); (b) written order, the earliest child's first
  (ADR-346 D7, ADR-292 D5). *Recommendation:* (a).
* **Q5 - A child of several steps.** (a) `async let _ = { … }`, and an
  `async { }` that starts no child is refused, because it would only wait for
  nothing; (b) another spelling for the group, so that `async { … }` is a child.
  *Recommendation:* (a).

## 8. Draft of the specification text

The text below goes into Part I 8.1.2 (replacing *Asking for Overlap*), Part
II 12.4 and 12.7, and Part III's rows once §7 is answered; the step is [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983)'s
first. It is kept here and not in Part I-III because Q2-Q5 change its central
sentences, and until it is built `overlap` is what the compiler accepts.

> ### 8.1.2. Running Work Beside the Code After It: `async`
>
> **`async` starts a child.** `async let x = e` starts `e` and goes on; the
> statements after it run beside it, up to the first statement of the block
> that names `x`, where the program waits for it. `async e` and `async for v
> in vs { … }` start children whose value nobody reads. `async let xs = for v
> in vs { body }` starts one child per element, and `xs` is the list of their
> results in the order of the elements. `async { … }` waits at its `}` for
> every child started inside it. A child nobody waits for by name is waited
> for at the end of its group. There is no `await`, and no function is
> `async`.
>
> **A child is not a task.** Nothing outlives its group: a child borrows, and
> the crossing rules a `spawn` meets (Part II, 11.2) do not apply.
>
> **A child and the code beside it meet on nothing, and the compiler checks
> it.** A child and a statement of its window, or two iterations of an `async`
> loop, that meet on a resource are refused with `NK2104`, naming it; so is a
> binding the window changes while a child names it. Two reads meet on
> nothing. `limit: 1` runs a form's children one at a time, and they are not
> checked against each other. `--overlaps` reports the children and their
> windows.
>
> **Options.** `async(limit: n, within: d)`: at most `n` children of the form at
> once; after `d`, the children still running are cancelled and the wait
> throws `task::TimedOut`.
>
> **A failure is the reader's.** A child's failure is thrown where its binding
> is read. One nobody reads is thrown at the end of the group, the first in
> written order, the others as its `secondary` list; where the group is left
> by a failure, the group waits for its children and their failures are that
> failure's `secondary`. `task::cancel(x)` stops a pending child and drops its
> outcome; `x` may not be named after it.
>
> **The meaning is the same at both values of `user_parallelism`; the duration
> differs.**
