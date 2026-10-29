# ADR-lambdas-1: A Lambda That Runs at Most Once May Give Away What It Captures; for `std` the Contract Says So, for User Code It Is Inferred

**Status:** Accepted
**Date:** October 29, 2026 (owner's decision on the architect's paper, options (a) and (b), no written marker; #973)
**Answers:** whether a lambda handed to a parameter that runs it at most once
may move a captured value into something that keeps it, which is `NK2105`
today (#973)
**Supersedes / Related:**
Part I 5.4 C (the context of a code parameter is inferred, not written; stands, extended by D3);
[ADR-277](adr-277.md) (a code parameter has the type `fn(A) -> R`, lowered by that type and by whether the body keeps it; D3 adds once-ness to what the lowering reads);
[ADR-094](adr-094.md) D2 (`keeps` names the parameters a body keeps; `once` sits beside it);
[ADR-collections-1](adr-collections-1.md) D1 (`entry` with `or_insert_with` and `and_modify` on both maps)

## 1. Question

**May a lambda that runs at most once hand a captured value to something that
keeps it?** Part I 5.4 decides whether a lambda borrows or moves what it uses
by the function that takes it: one that runs it before it returns borrows, one
that keeps it moves. Inside a lambda, handing a captured value over is refused
(`NK2105`), because *a lambda may run more than once, and the second run would
find it gone*. That is true of `map`. It is false of `or_insert_with`:

```nika
let t: Vec[i64] = [1]
m.entry("k").or_insert_with(fn() { H { a: "x", t: t } })
```

The way out today is `or_insert(H { .. })`, which builds the value and its
copies on every call, also where the key is already there, or a lookup before
the write. The self-hosted compiler meets it in `tools/sharing.nika`
(`Sharing::note`). No record spoke of a lambda that runs at most once.

## 2. How others do it

| Language | Its answer | Source |
| :--- | :--- | :--- |
| Rust | three closure traits; a parameter written `FnOnce` may be given a closure that moves out of what it captured; `Entry::or_insert_with`, `Option::unwrap_or_else`, `thread::scope` take `FnOnce` | std docs, from memory, not checked |
| Swift | a closure parameter is non-escaping unless written `@escaping`; how often it runs is not part of the type | from memory, not checked |
| Kotlin | `inline` functions with a `callsInPlace(f, EXACTLY_ONCE)` contract let a lambda initialise a `val`; the contract is written by the library | from memory, not checked |
| Java, Python, Go | values are shared by reference; the question does not arise | - |

Rust and Kotlin both state the fact on the library's side. Rust also asks the
user to write it on every parameter of their own; Nikaia infers what a body
already shows (5.4 C), so that part does not transfer.

## 3. Options, weighed

* **(a) `std` only**: the contract of a `std` lambda parameter that runs at
  most once says `once`. A lambda handed to it is not a repeat for the take
  walk.
* **(b) the same for user code, inferred**: a code parameter of a user
  function is once when its body calls it at most once on every path and hands
  it on only to a once parameter.
* **(b') the same, with a written marker** (`f: once fn()`, `@once`).
* **(c) keep refusing it**: `or_insert` with the copy, or the double lookup.

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **a** | nothing to learn | one contract field, one test in the take walk | no copy, no second lookup | the counterparts in Rust already take `FnOnce` | a wrong `once` row lets a retried lambda find a value gone: each row cites the sentence of Part II that says it runs once |
| **b** | nothing to learn | the walk exists; a ledger column and a fixpoint over calls, as for kept and lent | user helpers that wrap `entry` or a lock work as `std`'s do | follows 5.4 C | a callee edited to call `f` twice turns *many*; its callers get `NK2105`, naming that second call |
| **b'** | a word every reader meets | parser, formatter, LSP, and a check that the marker is true | the same as b | contradicts 5.4 C | - |
| **c** | the program reads wrong: a copy for nothing | none | a copy or a second lookup on every call | - | - |

(a) and (b) are one rule: the fact (a) writes into `std.contracts` is the fact
(b) computes for user code and writes into the ledger. Nothing of (a) changes
when (b) is built.

## 4. Decision

### D1 — A lambda handed to a parameter that runs it at most once may give away what it captures.

Inside such a lambda, a captured value may be handed to something that keeps
it: a field, a container, a kept argument. The lambda is not a repeat for
`NK2105`. After the call the value is gone, as an argument kept by the call
is, whether or not the lambda ran; a later use is `NK2105`. A lambda handed to
any other parameter keeps the rule it has. A loop around the call is still a
repeat. The lambda stays a function of its own for `return`, `touches` and the
rest.

### D2 — In `std`, the contract names the parameters that run at most once.

`std.contracts` gets the key `once`, beside `keeps`: `once = ["f"]` names the
lambda parameters the entry runs at most once. It stands on five entries:
`Entry::or_insert_with` (`f`), `Entry::and_modify` (`f`), `task::scope`
(`body`), `Locked::access` (`f`) and `SharedMut::access` (`f`). `update` is
not one: its block may run again where another task got there first (Part II
12.2). Every `Seq` entry, `Vec::retain`, `Vec::sort_by_key`, `Par::fold` and
`SharedMut::supervised`'s `build` run their lambda more than once, or may.
**Absent means the entry may run it more than once.** An analysis that needs
the fact reads this key, not a list of names.

### D3 — For user code, once-ness is inferred and never written.

A code parameter of a user function is **once** when its body calls it at
most once on every path and hands it on only to a once parameter. A call in a
loop, two calls on one path, or keeping it (a field, a `let`, a result, a
task) makes it *many*. It is the take walk the checker already runs, with a
call of `f` counted as handing `f` over. The answer is written to the ledger
as `once`, beside `keeps`, and a once parameter is lowered as `impl FnOnce`.
There is no syntax for it. A function value that is kept is never once.

### D4 — The refusal says why.

Where `NK2105` refuses a hand-over inside a lambda, it names the reason: the
parameter that may run it more than once (*`map` may run its lambda more than
once*), or, for a user function, the second call or the loop in the callee
that made the parameter *many*.

**Rejected:** (b') a written marker answers a question the compiler answers
from the body, and is a word a reader of a scripting language should not have
to learn (D2 of ADR-258); (c) makes the program that reads right pay a copy or
a lookup for nothing (D3 of ADR-258).

## 5. Consequences

* A program builds an owned value inside `or_insert_with`, `and_modify`,
  `access` or `task::scope` without a copy and without a second lookup.
* The lowering of `std`'s five entries changes nothing: their Rust
  counterparts take `FnOnce`, and rustc infers the move. For user code (D3), a
  once parameter becomes `impl FnOnce` instead of `impl Fn`.
* A user function's once-ness is part of its contract: an edit that calls the
  parameter a second time changes the ledger, and a caller that moved into its
  lambda is refused there, with the edit named (Part I 6.7).
* The work is #973: D1, D2 and D4 first, with the move in
  `tools/sharing.nika`; D3 after.
