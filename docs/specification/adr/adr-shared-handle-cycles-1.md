# ADR-shared-handle-cycles-1: The Compiler Refuses a Store That Closes a Cycle of Handles, Unless It Proved a Back Edge

**Status:** Accepted
**Date:** November 15, 2026 (owner's decision)
**Answers:** how a cycle of counted handles is freed, which ADR-312 left
open (#1626)
**Supersedes / Related:** [ADR-312](adr-312.md) (a handle is duplicated,
never moved; a plain or an atomic count); [ADR-389](adr-389.md) D2
(`immutable` reaches no `SharedMut`, `Locked` or channel end, `NK1331`);
[ADR-327](adr-327.md) (the budget and `SharedMut::supervised`);
[ADR-258](adr-258.md) (the four weights); Part I 6.2

## 1. Question

```nika
struct Node { name: String, next: SharedMut[Node]? }

fn main() {
	let a = SharedMut(Node { name: "a", next: null })
	let b = SharedMut(Node { name: "b", next: a })
	a.update(fn(mut v) { v.next = b })
}
```

Each handle counts its holders. After the `update`, `a` holds `b` and `b`
holds `a`; when `main` ends, both counts stay at one and neither value is
freed. The program compiles and leaks, without a word.

What follows from the rules already in force: a cycle of handles is closed
only by a *write after construction*, a handle that already exists stored
into a value its target already reaches. A `Shared[T]` value is complete when
its handle is made and can only point at values that existed before; Nikaia
has no recursive construction. Only two things write into a value behind a
handle: a door of `SharedMut[T]`, and a `Locked[T]` field inside a `Shared`
(Part I 6.2). So a type that reaches no `SharedMut` and no `Locked` is never
part of a cycle, and an `immutable` value never is (ADR-389 D2 excludes both).
Only a type that reaches *itself* through a `SharedMut` or `Locked` path, a
**self-reaching type**, can be.

A programmer who makes a parent link weak does so because the parent owns the
child: every path from the program's variables to the child goes through the
parent, so the edge back to it is a *back edge* (the parent dominates the
child). That is an invariant over all states of the program, so a compiler
can prove it for the shapes people build on purpose. Where no node owns
another, in a general graph, no link is a back edge, and no placement of weak
links is correct, by hand or by compiler.

## 2. How others do it

| Language | Answer |
| :--- | :--- |
| Rust, C++ | `Rc`/`Weak`, `shared_ptr`/`weak_ptr` placed by hand; graphs with indices (petgraph, slotmap with generations); no warning on a cycle |
| Swift | `weak` / `unowned` placed by hand; `unowned` is trusted, not proved; cycles through closures are a common bug |
| Koka, Lean 4 | precise counting over immutable values, which cannot form cycles; cycles through mutable references leak without a warning |
| Python, PHP | counting plus a cycle collector (PHP: Bacon-Rajan trial deletion) for every value |
| Go, Java, C# | a tracing collector; no leak, no predictable destruction |
| ML Kit | inferred regions, later a tracing collector because regions lived too long |
| Zig, Odin | indices and arenas; a stale index is the user's problem |

No mainstream language infers the back edge or refuses a cycle at compile
time.

## 3. Options, weighed

* **A. Do nothing.** The leak stays silent.
* **B. Indices for graphs, the compiler warns on a self-reaching type.** No
  new concept, correct for a general graph; the warning also fires on a list
  that never closes a cycle.
* **C. `Weak[T]` placed by the user.** Staleness is visible in the type; a
  forgotten weak link still leaks, and the decision is the user's.
* **D. A tracing cycle collector for everything.** The user writes nothing;
  every program with a handle pays, and values are freed when the program does
  not see it.
* **E. The store that closes a cycle is found at compile time.** Points at the
  line; by itself it cannot say which edge is the back edge (a child built with
  its parent link first is closed by the push into `children`).
* **F. The compiler infers back fields** (owners-as-dominators, after Clarke,
  Potter and Noble; shape analysis). The user writes nothing; the analysis is
  large and has an honest "could not decide".
* **G. Synchronous trial deletion for the self-reaching types only**
  (Bacon-Rajan). The meaning of a program stays as it is; a runtime cost on each
  such decrement, and a concurrency design for values that cross threads.
* **H. Inferred regions.** Fully static; regions outlive their values.
* **I. Freeze after building.** Impossible for a cyclic structure, by the
  argument of section 1.

### Weighed

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | nobody pays; the leak is everyone's | no | yes | no: a guarantee with a hole | nothing to undo |
| **C** | the writer of a cycle | no: the user decides | yes | a second handle kind | a forgotten link leaks silently |
| **D** | every program with a handle | yes | no | a second memory model | cannot be withdrawn once relied on |
| **E+F** (chosen) | only self-reaching types, at compile time | yes | yes: no runtime part; F1 links are cheaper than a handle | one rule beside the sharing check | a refusal can be loosened later without breaking a running program |
| **G** | self-reaching types, at run time | yes | yes for the rest | a runtime part the message has to explain | cannot be withdrawn once programs rely on it |
| **H** | structures in a region | yes | no: regions grow | a second memory model | as D |

E with F against G decided it, and the weights alone did not: both leave the
user writing nothing. The owner decided by reversibility and by the message
(section 4, *Why*).

## 4. Decision

The user writes nothing for cycles. The compiler guarantees at compile time
that a program never builds a cycle of handles, except where it has proved a
back edge and lowered that edge as non-owning. There is no runtime cycle
detection.

### D1 - What is free of cycles, by proof

A value whose type reaches no `SharedMut[T]` and no `Locked[T]`, through
fields, elements, variants or captures, is never part of a cycle; an
`immutable` value never is (ADR-389 D2). The checker computes the
self-reaching types: the strongly connected components of the type graph
whose edges pass through a `SharedMut` or a `Locked`. D2 to D4 apply only to
them; every other type pays nothing, at compile time or at run time.

### D2 - A store that can close a cycle is refused, `NK1352`

A store through a door of `SharedMut[T]`, or into a `Locked[T]` field, is
refused when the stored handle's target can reach the value written into,
among self-reaching types, and no back edge on that cycle is proved (D3). The
check is sound: it may refuse a correct program, it never lets a cycle
through. The message names the line, the cycle (which value reaches which,
through which fields) and the index form. Where reachability is not known, the
store is refused.

### D3 - A proved back field is lowered as non-owning

The compiler proves, over every store into a field of a self-reaching type,
that the value in the field dominates the value that holds it. A store that
closes a cycle through a field so proved is accepted, and the field is lowered
as:

* **non-owning, unchecked (F1)** where it is also proved that the target
  outlives the holder: no count and no check, like Swift's `unowned`, proved
  instead of trusted;
* **weak (F2)** where only domination is proved and the holder may outlive
  its target through another handle: reading the field gives `null` once the
  target is gone. Only a field whose type is already optional can be lowered
  so; a field that is not stays owning, and the closing store stays refused.

The user writes the ordinary field, `parent: SharedMut[Node]?`. Trees with
parent links, doubly linked lists and observer pairs are the shapes D3 must
pass. D3 decides how much D2 allows; until it passes a shape, that shape is
refused and written with indices.

### D4 - Summaries carry the judgement across functions and packages

A function has a summary: what each parameter and its result can reach, and
which fields it stores into. A package carries the summaries of its public
functions in its interface. With them a store is judged across functions and
packages without reading their bodies. Where no summary decides, the store is
refused (D2). Generic code and trait objects get conservative summaries.

### D5 - Where no edge is a back edge, indices

A general graph, where no node owns another, is built with indices into a
list; the message of `NK1352` says so, and later points to a `std` arena with
generations. `Weak[T]` is not the answer to a cycle. Whether a cache or an
observer list gets a `Weak[T]` for its own intent, a value it should not keep
alive, is a separate decision.

### D6 - No runtime cycle detection

No collector, no trial deletion, no region runs at run time for cycles.
Trial deletion for the self-reaching types (G) stays possible as a later
loosening of D2: it would accept programs D2 refuses and break none that
runs.

**Why.** A refusal at compile time can later be loosened, by a better
analysis or by a runtime part, without breaking a program that runs. A runtime
collector, once programs rely on it, cannot be withdrawn without breaking
them. And a runtime cycle detector in this phase would send an unclear message
about what the language does; the message is that the compiler does the work,
and that clarity matters more now than a runtime answer for edge cases. The
effort goes into compile time instead: D3 and D4 are to reach far enough,
across functions and packages, that a correct program almost never meets the
refusal.

**Why the others lost.** A leaves a hole in the guarantee. C puts the decision
on the user, and a forgotten link leaks silently. D makes every program pay.
G is a runtime part that cannot be withdrawn once relied on (D6 keeps it for
later). H: regions live much longer than their values, by the ML Kit's own
experience. I: an `immutable` value cannot hold a cycle, so a cyclic structure
cannot be frozen.

## 5. Consequences

* A program that closes a cycle of handles, like the one in section 1, is
  refused with `NK1352`. It compiles today and leaks; the change breaks it on
  purpose, which pre-alpha accepts.
* Until D3 and D4 reach far enough, correct programs with parent links,
  doubly linked lists and observer pairs are refused and written with indices.
  This cost is accepted; how many stores in our own code it refuses is
  measured as the rule is built (#1626, section 9).
* `immutable` gains a property: no cycle handling applies to an `immutable`
  value (ADR-389).
* The lowering gains a non-owning link (F1, a plain reference the proof makes
  safe) and a weak one (F2, `std::rc::Weak` or `std::sync::Weak` beside the
  handle's count, by ADR-312's plain or atomic count).
* The builds, in this order: the specification and the self-reaching set
  (D1); the rule within one function (D2); function summaries (D4); back
  fields for trees with parent links, then doubly linked lists and observer
  pairs (D3); package summaries (D4). Each is its own issue, linked from #1626.

## 6. What this does not decide

* `Weak[T]` for caches and observer lists, as intent (D5): asked when a
  program needs it.
* Trial deletion as a later loosening (D6): asked if the measurements of
  #1626 show correct programs D3 and D4 cannot reach.
* A `std` arena with generations (D5): its own issue.
