# ADR-async-3: Open - Whether Every Task Is a Child of a Group, With Its Lifetime and Captures Following From the Group

**Status:** Open
**Date:** October 30, 2026 (decision paper for the owner, [#983](https://github.com/Nikaia-Foundation/Nikaia/issues/983))
**Answers:** can the things that run beside other code - an `async` child, a `spawn` task with its `TaskHandle`, a `select` arm, a supervisor's child, a connection task under `serve` - be one kind of task, whose lifetime, captures and wait point follow from where it is started or from an option, instead of five concepts with their own rules?
**Supersedes / Related:**
[ADR-async-1](adr-async-1.md) D1, D5, D8, D10, D13, D14 (children, groups, what a child takes and borrows, `task::cancel`, the five forms, admission);
[ADR-346](adr-346.md) D1-D6 (every task error has a receiver; the handle must be taken; `join(within:)`; detached tasks);
[ADR-292](adr-292.md) D12-D15 (`select`; one cancel mechanism; `h.cancel()`; the pair);
[ADR-328](adr-328.md) D1, D7, D8, D10 (supervisors; a child keeps no state; a crash is an error to the waiter; how a group runs its tasks);
[ADR-326](adr-326.md) D2, D3 (admission for connections, the shape ADR-async-1 D14 took);
[ADR-327](adr-327.md) D5 (a block per task);
[ADR-258](adr-258.md) D1-D4 (the four weights);
Part I 5.4, 8.1, 8.1.2, 8.2, 8.3; Part II 12.4, 12.8; Part III A.2

*This record decides nothing yet. It is the decision basis for the owner, and
it stands on its own: its answer does not depend on
[ADR-async-2](adr-async-2.md)'s. The names come after both.*

## 1. Question

### 1.1 From zero

A program has several ways to have code run while other code runs:

```nika
async let user = db::load_user(id)          // a child: borrows, waited at `user`
let h = spawn fn { refresh(cache) }         // a task: moves, may outlive, a handle
select {                                    // arms: borrow, the first wins
    r = fetch(u) => r,
    _ = time::sleep(5.seconds()) => throw TooSlow,
}
supervisor::run([supervisor::child(fn { worker(config) })])  // restarted children
http::serve(addr) fn(req) { … }             // one task per connection
```

Each has its own section and its own words for the same few facts:

| | lifetime | captures | waited at | its failure goes to | cancelled by | memory |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `async` child (Part I 8.1.2) | its group | borrows what outlives the group, takes what does not (ADR-async-1 D5) | first mention, or the group's end | the mention, the fallback, the group's end | `task::cancel(x)`, a sibling's panic, `within:` | a block of its own, "as a task" |
| `spawn` task (Part I 8.2, 8.3) | may outlive the function | moves (`@detached`, Part I 5.4) | `h.join()`, or nobody | the joiner, or standard error | `h.cancel()`, `join(within:)` | a block of its own |
| `select` arm (Part II 12.4) | the block | borrows | the block, first one | the block | losing | not stated |
| supervisor child (Part II 12.8) | until `run` gives up | lent to every attempt; no `mut` | `run`'s end | the policy, then `Escalated` | the strategy | a block per task |
| connection task (`serve`, ADR-326) | one connection | the handler's | `serve` | the protocol, else as a detached task (Part III) | the server's shutdown | a block per task |

And the specification says, in Part I 8.1.2 and Part III's `NK2104` row, **"A
child is not a task"** - yet a child's tools live in `task::`, it has a task's
block, it ends with `task::OverBudget` and `task::Crashed`. Part I 8.1 still says
"a task exists only where the program writes one (`spawn`, `par_iter`)" and "a
pause never forks one", which `async` now contradicts. Part II 12.4 is titled
"Racing Tasks" though an arm is not called one.

Three smaller seams:

* **Two cancel spellings for one mechanism**: `h.cancel()` on a handle,
  `task::cancel(x)` on a pending binding (ADR-async-1 D10 explains why `x` cannot
  have the method: it names a value of the result's type).
* **`within:` counts from two places**: on `async` from where the form starts
  (ADR-async-1 D6), on `join` from the call to `join` (ADR-346 D4). The same word
  is a deadline in one place and a wait limit in the other.
* **`spawn` + `join` in one scope is nearly `async let`**, with a move where a
  borrow would do and a handle where a name would do (#983, point 5).

### 1.2 Where the question comes from

The owner, on #983 (2026-10-30): `async` beside `spawn` is confusing; unify the
concepts first, names last. Nothing is built yet of `async` (#984 `blocked`),
of supervisors (ADR-328: no) or of admission (ADR-326: no); `spawn` with
`TaskHandle` is built in part (ADR-346, #605). A change now costs specification
text and the shape of a runtime not yet written.

### 1.3 What the rules already share

Read side by side, the five already agree on more than the table shows:

* **The capture rule is one rule written twice.** ADR-async-1 D5: a child borrows
  what was bound outside its group and takes what was bound in a block that ends
  before the group does. Part I 8.3 / 5.4: a `spawn` moves everything it uses.
  The second is the first where the group is the process: every local ends
  before it, so everything is taken.
* **The failure rule is one rule.** Every error has a receiver (ADR-346 D1):
  the mention, the joiner, the group's end, the supervisor's policy, the
  protocol, standard error for nobody.
* **The memory rule is one rule**: a block per task (ADR-327 D5), the same for a
  child (ADR-async-1 D14).
* **Admission is one strategy shape**: ADR-326 D2 for connections, copied by
  ADR-async-1 D14 for children.
* **A panic is a crash to the waiter** everywhere (ADR-328 D8).

What really differs is the **group** each one is started in, and that group's
**policy**: wait for all (`async { }`), keep the first (`select`), restart
(`supervisor::run`), one per connection with admission (`serve`), and the
process itself, which nobody waits for except `main`'s drain (Part III A.2).

### 1.4 The question

**Should the specification say that every task is a child of a group - its
lifetime the group's, its captures by the one rule of §1.3, its failure the
group's receiver's - with the group's policy as the only difference, and
`spawn` the start of a child of the process?** And, separately: does that
model keep its five spellings, or does the start of a task become one word with
an option (§3, C)?

## 2. How others do it

All rows are from memory, not checked against a current manual.

| Language / tool | Its answer | What it costs there |
| :--- | :--- | :--- |
| **Swift** | every unit is a `Task`: *child tasks* (`async let`, `TaskGroup.addTask`) are bound to their scope and may borrow; *unstructured* `Task { }` (inherits priority and actor) and `Task.detached { }` outlive it, capture by escaping closure, and give a `Task<T, E>` with `.value` and `.cancel()` | one word "task", three ways to start one; cancellation is cooperative and one mechanism; the colored `async`/`await` |
| **Kotlin** | `launch` (a `Job`) and `async` (a `Deferred<T>`, a `Job` with a value) are both children of a `CoroutineScope`; the lifetime is the scope the builder is called on - `coroutineScope { }`, an object's own scope, or `GlobalScope` | one kind of coroutine; lifetime by *which scope*; `GlobalScope` is discouraged; `suspend` colors |
| **Trio** | only nurseries: `nursery.start_soon(f)`. To outlive a function, a task is started in a nursery passed in from further out | one concept; lifetime = the nursery chosen; no handle, values by side effect |
| **Java** | virtual threads are one kind of thread; `StructuredTaskScope.fork()` gives a `Subtask` bound to the scope, an executor or `Thread.ofVirtual().start()` one that outlives | one kind of thread, two ownership models, the structured one a library object |
| **C# / .NET** | one `Task` for everything: `Task.Run`, `async` methods, `WhenAll`, `Parallel.ForEachAsync`; no structure; cancellation is a `CancellationToken` passed by hand | maximal unification, no lifetime guarantee, errors of an unobserved task lost |
| **Go** | one form, `go f()`: no handle, no value, no structure; `errgroup`, `context` for cancellation | one concept; every guarantee is a library's or the program's |
| **Erlang / OTP** | one kind of process (`spawn`, `spawn_link`); supervisors and links are policies over it | uniform processes; nothing is borrowed, everything is copied |
| **Rust** | `std::thread::scope` borrows, `std::thread::spawn` needs `'static`; tokio `spawn` / `JoinSet` / `JoinHandle::abort()`; Rayon `scope` | a scoped and an unscoped spawn, written differently, the difference in the type (`'static`) |
| **Cilk / OpenMP** | `cilk_spawn` is always a child, implicitly synced at the function's end; OpenMP `task` belongs to the enclosing `taskgroup` | no unstructured task at all |

**What transfers.** Kotlin's and Trio's point: lifetime is *which group the task
is started in*, not a second kind of task. Swift's vocabulary, *child task*
against *unstructured (free) task*, under one word. Erlang's: a supervisor is a
policy over the one kind, not a kind of its own. **What does not:** C#'s and
Go's absence of structure (every error here has a receiver, ADR-346 D1);
Trio's passing of a nursery object to every function that starts work, which
puts a handle back into every signature; the coloring of Swift and Kotlin.

## 3. Options, weighed

### Option A - five forms, the vocabulary straightened

Nothing changes in what a program writes. The text says "child task" and "free
task" instead of "child" and "task", drops "a child is not a task", titles 12.4
for arms, and picks one cancel spelling.

### Option B - one model: every task is a child of a group; the forms keep their spelling

The specification states once, in one section, what a task is: started in a
group; lives as long as the group lets it; borrows what outlives the group and
takes what does not (the one rule of §1.3, which yields both today's borrow and
today's move); allocates from its block; ends with a value, a failure, a crash
or `task::OverBudget`, which the group's receiver gets. Each form then states
only its group's policy:

| Form | Its group | Policy |
| :--- | :--- | :--- |
| `async …` | the enclosing `async { }` or block | wait for all, keep every failure |
| `select { }` | the block | first wins, the rest are cancelled |
| `supervisor::run` | the call | restart by strategy and policy |
| `serve` | the server | one per connection, by admission |
| `spawn` | the process | nobody waits but `main`'s drain; a handle, if bound, is the receiver |

Two seams close inside the model: **one cancel spelling**, `task::cancel(x)`,
for a handle and a pending binding alike (`h.cancel()` goes); and **`within:`
means one thing**: a limit counted from where the wait starts, on `join` and at
an `async` binding's wait point alike - or, the other way, a deadline from the
start on both (the owner picks; §4 recommends one).

```nika
async let page = http::get(url)             // a child task of this block
let h = spawn fn { refresh(cache) }         // a child task of the process
task::cancel(h)                             // one spelling for both
```

### Option C - B, and one word starts every task

As B, and `spawn` becomes an option of the one word naming the group:
`async(detached) fn { … }` (unbound: nobody waits) and `let h =
async(detached) fn { … }` (bound: a `TaskHandle[T]`, taken as ADR-346 D3 says).
ADR-async-1 §6 left "an option on `async` that puts a child in another group"
open; this is that option with one value. The word chosen is the naming round's.

```nika
async let page = http::get(url)
let h = async(detached) fn { refresh(cache) }
```

### Option D - one word, the lifetime inferred from use

`async` everywhere and no option: a child whose binding leaves its group -
returned, stored, passed to a function that keeps it - becomes a free task and
takes its captures; one that does not stays a child and borrows. Part I 5.4
infers a lambda's captures from where it is handed in the same way.

```nika
fn start(cache: Shared[Cache]) -> TaskHandle[()] {
    async let h = fn { refresh(cache) }     // free, because `h` is returned
    h
}
```

### Weighed

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent with itself | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | nobody pays more | as today | none | partly: words agree, rules still written five times | nothing to undo |
| **B** | nothing new is written; `h.cancel()` users write `task::cancel(h)` | as today; one rule set to check, not five | none if a child keeps its lowering (polled by its group, no `Send`, no handle) | yes for rules; five spellings for five policies | low: it is text and one runtime shape; a rule can be split again |
| **C** | a free task pays an option, the common child pays nothing | as B | as B | most: one word starts a task, the option names the group | moderate: `spawn` is built in part and used in `tools/` and tests; the move is mechanical |
| **D** | nobody writes anything | the most: an escape analysis on the binding | a child that turns free pays `Send`, a heap and the executor, without a word in the source saying so | one word, but whether a line borrows depends on a later line | high: a `return h` added at the bottom moves captures at the top and refuses uses (`NK2101`) far away; an unbound `async fn { … }` cannot say "detached", so D still needs an option for that case |

## 4. Recommendation

**Option B now; C as the naming round's question.** B is what the five forms
already mean, written once: the capture, failure, memory and admission rules
are already one rule each (§1.3), and only the policy differs. It changes no
program but `h.cancel()`. Whether `spawn` survives as a word or becomes
`async(detached)` (C) is a matter of names, which the owner puts last, and B
leaves it open without a cost. **D is not recommended**: the unbound case cannot
be inferred at all (detached and group-bound both have no use), and for the
bound case the capture mode and the run-time cost would hang on a line far from
the one that starts the task. Part I 5.4's inference is decided at the call, by
the callee's ledger fact; D's would be decided by every later use.

The rules B would write, for the owner to accept or change (not yet `Dn`):

1. A **task** is started in a group and does not outlive it. A **child task**'s
   group is a block (`async`, `select`) or a call (`supervisor::run`, `serve`);
   a **free task**'s is the process (`spawn`).
2. A task borrows what outlives its group and takes what does not; Part I 8.3's
   move and ADR-async-1 D5's borrow are this rule's two cases.
3. Every task has a block, ends in one of four ways, and its end reaches the
   group's receiver; each form names its receiver.
4. `task::cancel(x)` cancels a pending binding or a handle and takes it;
   `h.cancel()` is removed.
5. `within:` is a limit on the wait, counted from where the wait starts, on
   `join` and on an `async` binding alike. (The alternative, a deadline from the
   start on both, is what ADR-async-1 D6 says today; it would change ADR-346 D4.
   Recommended is the wait, because a reader sees `within:` where the program
   waits on a handle, and an `async` form's options sit at its start only
   because its wait has no syntax of its own. The owner may prefer to keep the
   start for `async`, and then the word should differ: a naming point.)
6. "A child is not a task" becomes "a child task borrows; a free task takes".

## 5. Risks, named

* **Performance, if "one model" is read as "one runtime object".** A child
  polled by its group needs no `Send`, no reference count, no executor hand-off;
  a free task needs all three. B unifies the rules, not the lowering: the
  compiler keeps two lowerings and picks by the group, which it sees in the
  source. If a later step merges the runtime (one group type with a policy for
  `select`, supervisors and `serve`), each policy's per-task overhead must be
  counted before and after.
* **`within:` (rule 5) moves an accepted decision.** Either ADR-async-1 D6 or
  ADR-346 D4 changes; both are a day to a few weeks old, and nothing of `async`
  is built.
* **"Task" for an `async` child invites `spawn` expectations** (a handle, a
  thread). The vocabulary *child task* must be taught with its borrow, as Swift
  does.
* **`task::cancel(h)` beside `h.join()` and `h.detach()`** reads unevenly: one
  verb a function, two methods. The alternative keeps `h.cancel()` and accepts
  two spellings because the receivers differ (a handle has methods, a pending
  binding cannot).

## 6. Measurements owed

Nothing below is measured; callgrind counts, release builds on both sides, LTO
stated, the same output first:

1. `spawn` + `join` of a trivial body against `async let` + mention of the same
   body, at both settings of `user_parallelism`, once `async` is built (#984):
   what a child saves over a free task. Today only the `overlap` lowering is
   counted (`benches/overlap`).
2. If the runtime is merged (§5): per-task overhead of `select`, a supervisor
   child and a `serve` connection before and after.
3. A block per task: `benches/taskblock` counts 10 188 instructions a task (22
   allocations) with a block against 17 410 with `malloc`; whether `select` arms
   get blocks too (rule 3) adds that cost per arm, not yet counted.

## 7. What this does not decide

* A pending binding as a `select` arm (ADR-async-1 §6); under B it is natural,
  but it is its own question.
* Cancelling siblings at the first failure (ADR-292 §6).
* Naming any group other than the enclosing one and the process (Trio's passed
  nursery); C names only the process.
* The names: `sync` / `async` as a false pair (`sync` is a function property,
  `async` means "run beside"); `spawn`; Part I 8.1's title "Async by Default";
  12.4's "Racing Tasks"; 12.7's "Work That Borrows"; `--overlaps`. These go to
  the naming round after this record and ADR-async-2.
