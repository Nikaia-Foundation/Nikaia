# ADR-async-2: Open - Whether `par_iter` and the `async` Loop Become One Mechanism

**Status:** Open
**Date:** October 30, 2026 (decision paper for the owner, [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983))
**Answers:** can a loop over data that runs on the cores (`xs.par_iter() …`) and a loop that starts one child per element (`async for`, `async let xs = for …`) be one mechanism, with the compiler choosing how it runs, instead of two forms with two rule sets and a warning between them?
**Supersedes / Related:**
[ADR-async-1](adr-async-1.md) D3, D4, D6, D8, D11, D14, D15 (the `async` loop: its value, its options, its check, how it runs, its memory);
[ADR-235](adr-235.md) D1, D2 (`par_iter`: a `Par[ref T]`, no pause, no write to an outer name);
[ADR-372](adr-372.md) D9 (`fold`'s `step` and `merge` may change their first argument);
[ADR-327](adr-327.md) D5, D9, D10 (a block per task; what outlives a task is not in it; a budget counts what is held);
[ADR-328](adr-328.md) D10 (a pausing child is polled by its group, on the group's thread);
[ADR-288](adr-288.md) D32 (`NK2211`, the warning when a function starts to pause);
[ADR-258](adr-258.md) D1-D4 (the four weights);
Part I 8.1.2; Part II 12.6, 12.7

*This record decides nothing yet. It is the decision basis for the owner: one
question, answerable on its own. The companion question - whether the task
concepts become one - is [ADR-async-3](adr-async-3.md), and the names come after
both.*

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

## 4. Recommendation

**Option B.** It answers the owner's first question with yes for the
mechanism: one set of rules, one runtime, and the execution read off what the
compiler already knows. It keeps two spellings only where they say different
things - a loop with a window and a value per element, a chain with `filter`,
`fold` and pieces of bytes - and adds no syntax. C is the step after B, if a
reducer proves natural; it can be taken later without undoing B. D undoes a
decision a day old for the shape that decision rejected.

The rules B would write, for the owner to accept or change (not yet `Dn`):

1. An `async` loop whose body never pauses runs in pieces on the pool; one whose
   body may pause runs one child per element. The meaning does not depend on
   which, and `--overlaps` names it.
2. Per element: a result in input order, every failure kept, a full block fails
   that element; a piece's block is emptied between its elements.
3. A `par_iter` lambda is checked as an iteration of an `async` loop
   (`NK2104` is asked of it), and its failures follow the same rule.
4. `limit:` is the one option for how many at once, on both; `workers:` goes.
   Without it, a computing loop runs at most `task::cores()` pieces, and a
   pausing one is under admission.
5. `async let xs = for …` named in the very next statement is allowed
   (`NK2110` is about one child, not a loop).
6. A loop whose execution changes because its body started to pause is a
   warning naming the call that made it pause.

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

* Whether a pausing child may run on the pool instead of its group's thread
  (ADR-328 D10's limit); it would shrink §5's cliff, and is a runtime question
  of its own.
* The names (`par_iter`, `Par`, `workers:`, `piece:`): after ADR-async-3.
* `order: Any` on a loop with a value; under B a loop's results keep input
  order.
