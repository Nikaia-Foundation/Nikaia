# ADR-error-sets-1: A Code Parameter the Body Does Not Keep Fails, at Each Call, with What That Call Hands It

**Status:** Accepted
**Date:** October 29, 2026 (owner's decision on the architect's paper, option A; #625)
**Answers:** whether a call of a function with a code parameter fails with the
union of what every caller hands that parameter, or with what this call hands
it (#625)
**Supersedes / Related:**
[ADR-357](adr-357.md) (the set at a `catch` is closed; a `match` naming every type arriving needs no `else`; stands, and this record makes the set it sees exact);
[ADR-351](adr-351.md) D3 (the arguments every caller hands one parameter meet in it; changed by D1, D2 here: only for a kept parameter), D4 (`from(p)` in a ledger, replaced by the caller), D8 (kept code's set belongs to its storage; stands);
[ADR-094](adr-094.md) D2 (`keeps` names the parameters a body keeps; D2 here reads it);
[ADR-lambdas-1](adr-lambdas-1.md) D3 (once-ness of a code parameter is read off the body and never written; the same shape)

## 1. Question

**Does a `catch` around `retry(read)` see only what `read` fails with, or also
what every other caller hands `retry`?**

```nika
enum ReadError { Missing }
impl Error for ReadError { fn message(ref self) -> String { "missing" } }
enum ParseError { Bad }
impl Error for ParseError { fn message(ref self) -> String { "bad" } }

fn read() throws { throw ReadError::Missing }
fn parse() throws { throw ParseError::Bad }
fn retry(f: fn() throws) throws { f() }

fn main() {
    retry(read) catch {
        match error {
            ReadError::Missing => println("read failed")
        }
    }
    retry(parse) catch {
        match error {
            ParseError::Bad => println("parse failed")
        }
    }
}
```

ADR-351 D3 made a parameter a place where the arguments of every caller meet,
so each `catch` above sees `ReadError` and `ParseError`. While a `catch` over
several types needed `else` anyway, that union cost nothing. ADR-357 closed the
set: a `match` without `else` must name every type that can arrive. With the
union, each `match` above is `NK1151` for a type that cannot reach its call,
and the message cannot say honestly why it is asked for. The question has to be
decided before the rest of ADR-357 (growth narration, `NK2404`) is built on top
of the set the checker computes.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Swift | `rethrows`: a function throws only if its closure argument does; typed throws `throws(E)` is generic over `E` | `rethrows` says yes or no, not which; two failing closures need a written union type | from memory, not checked |
| Koka | Effect rows, inferred and polymorphic: a higher-order function's row is its own plus its argument's, per call | precise and unwritten, but messages show row variables users must learn | from memory, not checked |
| Zig | Error sets inferred per function, closed, `switch` exhaustive | inference stops at function pointers: a pointer type names its set or uses `anyerror` | from memory, not checked |
| Java | Checked exceptions, written by hand | the lists do not compose with lambdas (`java.util.function` declares none) | from memory, not checked |
| Rust | `Result<T, E>` with `E` generic, per call | the programmer writes the generic and the conversions | from memory, not checked |

Precision and lambdas coexist where the set is inferred and polymorphic per
call (Koka, Swift's typed throws, Rust's generic `E`). Nikaia already infers
the set and already has the polymorphic member `from(p)` (ADR-351 D4), without
a written variable; what does not transfer is the written row or `rethrows`.

## 3. Options, weighed

### Option A — per call, for every code parameter the body does not keep

The example builds as written: each `catch` names its own type.

### Option B — the union over callers, as ADR-351 D3 had it

Each `match` above names both types, or writes `else`.

### Option C — closed inside a package, open across its boundary

A `catch` over a set that holds a member from another package's ledger needs
`else`; the example builds only because everything is in one package.

### Weighed

| | D1 the special case pays, not the common one | D2 the compiler works, not the developer | D3 no cost for the common case | D4 consistent with itself | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | only kept code (stored, any caller) carries a union | the compiler substitutes per call, as it does for `from(p)` already | nothing written, no run-time cost beyond today's `from(p)` call | one rule for every `catch` | undoing it brings back arms for impossible types; nobody's program stops building |
| **B** | every program with a shared helper pays arms or `else` | the developer writes arms for types that cannot arrive | dead arms or a dead `else` | closed sets with an imprecise source contradict ADR-357's promise | — |
| **C** | every handler over `std` pays `else` again | the developer learns where a function lives | `else` in the common script | two rules; moving a function between packages changes whether a handler builds | — |

## 4. Decision

### D1 — A code parameter the body calls or hands on, and does not keep, is `from(p)` in the function's set, for every function, public or not.

At each call, `from(p)` is replaced by the set of what that call hands the
parameter. A `catch` around the call sees that set, the function's own errors
besides, and nothing other callers hand it. Inside the body a `catch` around
`f()` sees `from(f)`, which is open there (ADR-357 D2): the body runs for every
caller.

### D2 — A kept code parameter stays a place.

Where the body keeps the parameter (`keeps`, ADR-094 D2: stored in a field, a
list, a `Shared`, a task), its set belongs to the storage it enters, the union
of everything put there in the storage's life (ADR-351 D8). Stored code really
can come from any caller, and per-call precision for it would need flow
analysis, which ADR-351 D8 refuses.

### D3 — Nothing is written.

As for `throws` (ADR-023) and once-ness (ADR-lambdas-1 D3), whether a
parameter is kept is read off the body, and the ledger already publishes
`from(p)` (ADR-351 D4). There is no row variable and no `rethrows`.

### D4 — Lowering.

A call whose callee's set holds `from(p)` is lowered as ADR-351 D4 lowers one
of a `pub` function today; what the caller handed is known at the call, so a
complete `match` around it needs no catch-all arm (ADR-357 D5).

**Rejected.** B asks for arms for errors that cannot arrive. C brings back the
two-rule language and `else` over every `std` call. A written `rethrows` is a
word the body already answers (ADR-258 D2).

## 5. Consequences

* A `match` over `error` around a call of a helper with a code parameter names
  what that call can fail with, and `NK1151` names only types that can arrive
  there.
* Part I 7.1 says so: the arguments every caller hands one parameter meet only
  where the parameter is kept.
* A private function's code parameter is lowered as a `pub` function's is
  (D4); no new lowering form comes in.
* The cost accepted is unchanged from ADR-357: growth of a `pub` function's set
  stops complete handlers in dependents, and `NK2404` warns its author.
* The work is #625.
