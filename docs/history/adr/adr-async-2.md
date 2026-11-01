# ADR-async-2: Fan-Out Over a Collection Is the `par_iter` Chain, Whose Lambdas May Pause; the Compiler Picks Pieces or One Task per Element, Under One Rule Set; the `async` Loop Is Removed

**Status:** Accepted
**Date:** October 30, 2026 (owner's decision, option D, in the third design round, [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983))
**Answers:** can a loop over data that runs on the cores (`xs.par_iter() …`) and a loop that starts one child per element (`async for`, `async let xs = for …`) be one mechanism, with the compiler choosing how it runs, instead of two forms with two rule sets and a warning between them?
**Supersedes / Related:**
[ADR-async-1](adr-async-1.md) D3, D4, D6, D8, D11, D13 (changed: the `async` loop is removed; fan-out is `par_iter`'s; D14, D15 hold for its tasks);
[ADR-235](adr-235.md) D1, D2 (changed: a `par_iter` lambda may pause; a write to an outer name is still `NK2107`);
[ADR-276](adr-276.md) D13 (stands again for every loop: no loop has a value);
[ADR-372](adr-372.md) D9 (`fold`'s `step` and `merge` may change their first argument);
[ADR-327](adr-327.md) D5, D9, D10 (a block per task; what outlives a task is not in it; a budget counts what is held);
[ADR-328](adr-328.md) D10 (a pausing child is polled by its group, on the group's thread);
[ADR-288](adr-288.md) D32 (`NK2211`, the warning when a function starts to pause; `NK2119` is its shape for a `par_iter` lambda);
[ADR-async-4](adr-async-4.md) D8 (`async xs.par_iter() …` is one task that fans out);
[ADR-258](adr-258.md) D1-D4 (the four weights);
Part I 8.1.2; Part II 12.6, 12.7; Part III the `NK2104`, `NK2107`, `NK2119`, `NK2120`, `NK2209` rows, `--overlaps`

*The owner chose option D (§3) over the recommended B: fewer forms, and the
chain already says what a loop cannot (`filter`, `fold`, pieces of `Bytes`).
Sections 1-3 are the decision paper as it was put; §4 is the decision.*

## 1. Question

### 1.1 From zero

A program that has a list and wants the work per element to run at the same
time has two forms today.

```nika
// computation over data, on every core (Part II 12.6)
let bright = pixels.par_iter().map fn(p) { p.brightness * 1.5 }.collect()

// work beside the code after it, one child per element (Part I 8.1.2)
async(limit: 8) let pages = for url in urls { http::get(url) }
```

Both hand back a list in input order, both refuse a body that writes a name
bound outside it (`NK2107`), both run on the pool where the body never pauses.
They differ in almost everything else:

| | `par_iter` (Part II 12.6, ADR-235) | `async` loop (Part I 8.1.2, ADR-async-1) |
| :--- | :--- | :--- |
| body may pause | no, `NK2209` | yes |
| unit of work | a piece of elements, claimed while the work runs | one child per element |
| memory | not specified: the caller's block | a block per child (ADR-async-1 D14) |
| check between iterations | `NK2107` only | `NK2104` touch sets and `NK2107` |
| runs beside the code after it | no: the call returns when it is done | yes: a window, waited at the first mention |
| failure | not specified for a throwing lambda | every failure kept, the first in input order thrown, the rest `secondary` |
| a full block | not specified | `task::OverBudget`, fails that child only |
| how many at once | `workers:` (`fold` only), default `task::cores()` | `limit:`, default admission (D14) |
| order | `order: Kept \| Any` on `fold` | input order, always |
| fold, filter, pieces of `Bytes` | `fold(init, step, merge)`, `filter`, `par_pieces` | none: a list of results |
| time limit, cancel | none | `within:`, `task::cancel` |
| built | yes | no (#984, blocked on #983) |

**One verified inconsistency.** The same body is refused in one form and
accepted in the other. With the current `target/fast/nikaia`:

```nika
xs.par_iter().for_each fn(x) { println(f"{x}") }   // compiles and runs
async for x in xs { println(f"{x}") }              // NK2104 by Part I 8.1.2 (async is not built yet)
```

Two iterations writing stdout at once is the same fact in both; only one form
asks.

### 1.2 Where the question comes from

The owner, on #983 (2026-10-30), after ADR-async-1 was accepted: *fewer forms
are more*; a difference in meaning should be what the compiler infers or what
an option says, not a separate construct. The main session's inventory on #983
(point 4) names this overlap: the boundary is clear in the head of whoever
wrote it, and drawn nowhere. Nothing is built on the `async` loop yet (#984 is
`blocked` on #983), so a change now costs specification text, not code.

### 1.3 The question

**Should the `async` loop and `par_iter` become one mechanism - one set of
rules (check, failure, memory, order, limit) and one runtime - with the
compiler choosing per loop whether it runs in pieces on the cores or as one
child per element, read off whether the body can pause?** And, separately:
how many spellings does that mechanism keep (§3)?

## 2. How others do it

All rows are from memory, not checked against a current manual.

| Language / tool | Its answer | What it costs there |
| :--- | :--- | :--- |
| **Cilk** (Cilk Plus, OpenCilk) | `cilk_for` is compiled into `cilk_spawn` / `cilk_sync` by recursive halving down to a grain size; fold is a *reducer* (hyperobject), a value each worker sees its own view of, merged at the sync | one mechanism for data and tasks; a spawn costs a few times a call because a child is a stack frame and the thief steals the continuation; no I/O pausing at all |
| **OpenMP** | `parallel for` with `schedule(static \| dynamic \| guided, chunk)` and `reduction(+:x)`; `task` / `taskwait` / `taskgroup`; since 4.5 `taskloop` with `grainsize` / `num_tasks` turns a loop into tasks | the loop and the task are two mechanisms, `taskloop` the bridge; chunking is an option the programmer tunes |
| **Rayon** (Rust) | `par_iter()` with adaptive splitting (a piece is split again only when another worker steals), built on `join`; `scope(|s| s.spawn(…))` on the same pool | one work-stealing pool for both; a body that blocks holds a worker; no I/O |
| **tokio** (Rust) | `JoinSet`, `stream::iter(xs).map(f).buffered(n)` (order kept) or `buffer_unordered(n)`; `spawn_blocking` for computation | a second runtime beside Rayon; mixing the two is the program's work, and blocking a tokio worker is the classic mistake |
| **C# / .NET** | `Parallel.For` / `ForEach` (a partitioner chunks), PLINQ `AsParallel()`; `Parallel.ForEachAsync(xs, new ParallelOptions { MaxDegreeOfParallelism = n }, async (x, ct) => …)` since .NET 6 for a body that awaits | two entry points, but one option type and one `Task` underneath; the async one runs one body per element, the sync one chunks |
| **Java** | parallel streams on the common ForkJoinPool (`spliterator` splitting); for blocking work, one virtual thread per element under `StructuredTaskScope` | two mechanisms; a parallel stream with a blocking body starves the common pool |
| **Swift** | `withTaskGroup` + `addTask` per element; `DispatchQueue.concurrentPerform(iterations:)` for computation | a child task per element, never chunked; computation is a separate API |
| **Kotlin** | `xs.map { async { f(it) } }.awaitAll()`; `Dispatchers.Default` for computation, `limitedParallelism(n)` | one coroutine per element; chunking is the program's |
| **Go** | a goroutine per element, `errgroup.SetLimit(n)` | one mechanism; cheap goroutines (a small growable stack) make one per element acceptable for many cases; chunking is the program's |

**What transfers.** Cilk's and Rayon's point that a data-parallel loop is the
task mechanism plus a grain the runtime chooses: one mechanism is possible
where children are cheap and the runtime splits lazily. C#'s
`Parallel.ForEachAsync`: the split between "chunk" and "one per element"
follows whether the body awaits, which is what this language already knows
without a keyword (the ledger, Part II 12.1). Cilk's reducer as a way to say a
fold inside a loop.

**What does not.** Cilk has no pausing: its children never wait on I/O, so it
never has to choose. Go's and Kotlin's "one per element, chunk by hand" puts
the work on the developer (ADR-258 D2). tokio's and Java's two pools are what
this language set out to avoid.

## 3. Options, weighed

Common to every option except A: **the rules of the one mechanism are the
`async` loop's** (ADR-async-1 D3, D8, D9, D14, D15): per element a result in
input order, a check between iterations (`NK2104`, `NK2107`), every failure
kept, a full block failing one element. What differs is how many spellings
remain and who chooses the execution.

### Option A - two forms, the boundary written down

Both stay as they are. The specification states when to take which, and a
warning on an `async` loop whose body never pauses over a sized list names
`par_iter`. The `NK2104` gap (§1.1) is closed one way or the other.

```nika
let bright = pixels.par_iter().map fn(p) { p.brightness * 1.5 }.collect()
async(limit: 8) let pages = for url in urls { http::get(url) }
```

### Option B - one mechanism, the body decides how it runs; `par_iter` stays as the chain over it

The compiler reads the body's pause property, which it knows already (Part II
12.1, the ledger):

* **A body that never pauses** runs in pieces, claimed while the work runs
  (Part II 12.6's execution): one block per piece, emptied between elements, so
  the budget is still per element (ADR-327 D10 counts what is held; a result
  leaves the block, D9) and a full block fails that element, not the piece.
* **A body that may pause** runs one child per element, under admission and
  `limit:` (ADR-async-1 D14), polled by its group (ADR-328 D10), as today.

`par_iter` stays as a surface for what a loop cannot say - `filter` chains,
`fold(init, step, merge)` with `order:`, `par_pieces` over `Bytes` - and
obeys the same rules: its lambdas are checked as an iteration is (`NK2104`
included), a throwing lambda's failures are kept as an `async` loop's are,
`limit:` replaces `workers:`, `piece:` is accepted on both. An `async let xs =
for …` named directly in the next statement is allowed: its iterations are each
other's window, so `NK2110` does not apply to a loop. The two lines below then
run the same way:

```nika
let bright = pixels.par_iter().map fn(p) { p.brightness * 1.5 }.collect()
async let bright = for p in pixels { p.brightness * 1.5 }
use(bright)
```

`--overlaps` reports, per loop, which execution it got and why. Where a loop's
execution changes because a body started to pause, a warning says so, as
`NK2211` does for a function that starts to pause (ADR-288 D32).

### Option C - one spelling: the `async` loop; `par_iter` is removed

As B, and `par_iter` goes. A fold is a **reducer** (Cilk): a `std` value
declared before the loop with its `init` and `merge`, which each piece sees its
own copy of and which the check exempts from `NK2107`. `filter` is a body that
hands back `T?`, collected without the `null`s; pieces of `Bytes` are a
source (`b.pieces(sep)`) the loop splits.

```nika
let summary = par::Reducer(Summary, Summary::merge)
async for r in rows { summary.view().record(r) }
let s = summary.take()                          // after the group's end
```

### Option D - one spelling the other way: `par_iter` takes pausing lambdas; the `async` loop is removed

C#'s `ForEachAsync`: the chain runs one call per element where the lambda
pauses and in pieces where it does not; a loop beside the code after it is
`async let pages = urls.par_iter().map fn(u) { http::get(u) }.collect()`, one
child that fans out.

### Weighed

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent with itself | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | the developer pays for choosing, every time | no: the developer picks the execution | none added | no: two rule sets for one idea; a body is refused in one and accepted in the other unless the gap is closed | cheap to undo; nobody notices the duplicate until a reader asks which to take |
| **B** | a pausing body pays for children; a computing one pays only a block reset per element | yes: the pause property, already known, picks the execution | to be measured: a per-element failure slot and block reset on `par_iter` (§5) | yes for the rules; two spellings remain, each for what the other cannot say | moderate: rules can be loosened again; the warning makes the cliff visible |
| **C** | a fold pays a new `std` type the checker knows by name | yes | as B, plus a reducer's view per piece | most: one form; but a reducer is a value the body writes, an exception to `NK2107` | high: removes a built, tested form (`par_iter`, `fold`, `par_pieces`, ADR-372 D9) and every program on it |
| **D** | a loop beside the window pays a lambda and a chain | yes | as B | one form, but the window becomes one child around a chain; ADR-async-1 option G was rejected for this shape | high: undoes the `async` loop just accepted; the checker sees calls, not iterations |

## 4. Decision

### D1 - Fan-out over a collection is `std`'s `par_iter` chain, and its lambdas may pause.

```nika
let bright = pixels.par_iter().map fn(p) { p.brightness * 1.5 }.collect()
let pages = urls.par_iter(limit: 8).map fn(u) { http::get(u) }.collect()
```

`map`, `filter` and `for_each` take a lambda that may pause; `NK2209` is no
longer asked of them. The tasks a chain runs belong to the call: the call is
their group, and it returns when every one has ended (ADR-async-3 D4). A
lambda of `fold` (`init`, `step`, `merge`), `par_pieces` and `par_fold` is
still action code and may not pause (`NK2209`, D9).

### D2 - The compiler picks the execution from the lambda's pause property.

* **A lambda that never pauses** runs in pieces on the cores, claimed while the
  work runs (Part II 12.6), at most `task::cores()` at once.
* **A lambda that may pause** runs one task per element, under admission and
  `limit:` (ADR-async-1 D14), polled by the call's group (ADR-328 D10).

The meaning does not depend on which: D3's rules hold for both, and the
setting of `user_parallelism` changes the duration only.

### D3 - One rule set, the same as for `async` tasks.

* Results come back in input order, whatever order the elements end in.
* Every failure is kept: the first in input order is thrown where the call
  returns, the others are its `secondary` list in input order (ADR-async-1 D9).
* A full block fails only its element, with `task::OverBudget` (ADR-async-1
  D15); only a panic cancels the other elements, and the call throws
  `task::Crashed`.
* `NK2104` is asked of a `par_iter` lambda as of an iteration: the lambda
  against itself, so a lambda that writes stdout or another resource is
  refused, naming it. `limit: 1` written as the literal exempts it, as it
  exempted an `async` loop's iterations.
* `NK2107` as before: a lambda that changes a name bound outside it.

### D4 - `limit:` replaces `workers:`; `within:` and `admission:` are taken by the chain.

`xs.par_iter(limit: n, within: d, admission: s, piece: k)`, and `limit:` on
`fold` and `par_fold` where `workers:` stood. `limit: i64` caps how many pieces
or tasks run at once; below `1` it panics, naming the option. `within:` cancels
the call's running tasks when it runs out and the call throws `task::TimedOut`.
`admission:` names the strategy for a pausing chain; a computing one ignores it.

### D5 - `--overlaps` reports the execution per site, and a lambda that starts to pause is a warning.

`--overlaps` lists every `par_iter` call with the execution chosen (pieces or
one task per element) and why: the call that makes the lambda pause, or none.
Where the committed ledger recorded a site as running in pieces and its lambda
can now pause, the build warns (`NK2119`) at the call that made it pause, as
`NK2211` does for a function (ADR-288 D32); committing the ledger acknowledges
it. It is not a refusal: a lambda written to pause, `http::get` in it, gets no
warning, because nothing changed.

### D6 - What the shared rules add is paid only where the compiler knows it is needed.

* A failure slot only for a lambda that can throw, and its list is allocated on
  the failure path; a lambda that cannot throw has no slot.
* A block reset per element only for a lambda that allocates and keeps memory
  past its element; one that does not shares its piece's block untouched.
* Every check (`NK2104`, `NK2107`, `NK2119`) is compile-time only.

**Measurements owed** before the lowering is accepted (callgrind counts only,
release builds on both sides, LTO stated, the same output first):
`par_iter().map(…).collect()` over 10^6 elements against Rayon, once with a
lambda that can throw and once with one that allocates; and §6's rows 2-4.

### D7 - The `async` loop is removed.

`async let xs = for …` and `async for` are gone, and with them the one loop
that had a value: ADR-276 D13 holds for every loop again. `NK2116` and
`NK2117` are retired, not reused. A list of results computed beside the code
after it is `let xs = async vs.par_iter().map fn(v) { … }.collect()`
(ADR-async-4 D8).

### D8 - `async` starts only tasks whose number stands in the source.

An `async` form inside a loop's body - the same function, a nested block of it
included - is refused (`NK2120`), and the help names
`xs.par_iter().for_each fn(x) { … }`. A lambda's body is a function of its own
(Part I 5.3), so a lambda written inside a loop may hold an `async` form. So a
group's tasks are counted in the source, and a group of fixed size is laid out
without an allocation.

*The paper recommended option B: two spellings (the `async` loop and the chain) over one mechanism. The owner took D: one spelling, the chain.*

### D9 - The lambdas of `fold`, `par_pieces` and `par_fold` never pause.

*Owner's decision, 2026-10-30, closing §7's first point.*

`init`, `step` and `merge` of `fold` and `par_fold`, and a `par_pieces`
lambda, are action code and may not pause (`NK2209`, [ADR-235](adr-235.md)).
A fold over values that need a pause is written as a pausing `map` followed by
the `fold`: `xs.par_iter().map fn(u) { http::get(u) }.fold(…)`. The help of
`NK2209` on a `fold` lambda names that chain.

**Why.** An accumulator is folded by one worker at a time without a lock
(Part II 12.6); a step that pauses would hold the accumulator across the pause,
or fold it by tasks that meet. The pausing part has a place of its own in the
chain already.

## 5. Risks, named

* **A silent performance cliff.** One call in the body that may pause - a log
  line that flushes, a cache miss that reads a file - turns a loop over the
  cores into children that take turns on one thread (Part II 12.7, ADR-328
  D10). Today `NK2209` refuses that body, so the developer sees it; under B it
  compiles and runs slower by up to the number of cores. Rule 6's warning and
  `--overlaps` are the mitigation; they are not a refusal.
* **Programs that compile today are refused.** `par_iter` lambdas that print
  or write another shared resource become `NK2104` (rule 3). Any `examples/`,
  `tools/*.nika` or test with such a lambda moves to the value form.
* **`par_iter`'s per-element cost may grow.** Kept failures need a slot per
  element or a side list; a block reset per element is a store, but on a body of
  a few instructions it is not nothing. Rayon's map over a million elements
  costs a few instructions an element; anything per element shows.
* **Per-element budget versus per-piece cost.** Emptying a block between
  elements keeps ADR-async-1 D15's "fails one child" true only if nothing of an element
  stays in the block; ADR-327 D9 says what outlives is not in it, but this is
  not built (ADR-327 is built for D1-D3 only).
* **Option B keeps two spellings.** It unifies the mechanism, not the surface;
  the owner's "fewer forms" is met for rules and runtime, half for syntax.

## 6. Measurements owed

Nothing below is measured. Each is an instruction and mispredicted-branch count
from callgrind, release builds on both sides, LTO stated, the same output byte
for byte first (CLAUDE.md, *Measuring performance*):

1. `par_iter().map(…).collect()` over 10^6 small elements, today's lowering
   against the lowering with rule 2 (failure slot, block reset per element),
   at `user_parallelism = yes` and `no`; against tuned Rust (`rayon`, and a
   hand-chunked loop).
2. The same body as an `async let xs = for …` lowered one child per element
   (block, admission, scheduling) against pieces: what the inference saves.
   Needs the `async` loop built (#984).
3. The block cost alone: creating and emptying an empty 64 KiB block, which
   `benches/taskblock` does not separate (its README: 10 188 instructions a
   task with 22 allocations, against 17 410 with `malloc`; block set-up not
   isolated).
4. A body that pauses once in a thousand elements, under B, against the same
   loop split by hand: the size of the cliff of §5.

## 7. What this does not decide

*Answered (owner, 2026-10-30): a lambda of `fold`, `par_pieces` or `par_fold`
never pauses (D9); the names are final ([ADR-async-4](adr-async-4.md) D13).*

* Whether a pausing child may run on the pool instead of its group's thread
  (ADR-328 D10's limit); it would shrink §5's cliff, and is a runtime question
  of its own.
* `order: Any` on `map` and `collect`; results keep input order (D3).
