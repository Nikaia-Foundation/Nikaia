# ADR-failure-return-1: A Failure's Status Travels Beside the Value, and the Throw Path Is Cold

**Status:** Accepted
**Date:** November 9, 2026
**Answers:** ADR-023 §3.1 (where the success/failure decision lives), and how
ADR-023 D10's "the success path pays no memory round trip" is kept on each
backend
**Supersedes / Related:**
[ADR-023](adr-023.md) D10 (the cost guarantee; made precise here), §3.1
(answered here), §4 (the literal carry flag; reopened for one backend);
[ADR-280](adr-280.md) D24, D25 (the one-word tail whose non-zero word this
uses), §6 (in-value versus out-of-band status; answered here);
[ADR-283](adr-283.md) (views, the two-word return this cares about);
[ADR-004](adr-004.md) (the Rust backend);
issue [#1235](https://github.com/Nikaia-Foundation/Nikaia/issues/1235) (our own
optimiser, Cranelift, copy-and-patch);
[`docs/measuring.md`](../../measuring.md)

## 1. Question

ADR-023 D10 promises, normatively, that a call which can fail pays no memory
round trip on success. ADR-023 §3.1 left open how: the status **in the value**
(A, a tag in a niche, what Rust gives) or **beside it** (B, a register of its
own, for a backend that owns its calling convention). Issue #1235 plans exactly
such a backend. So the question is no longer theoretical, and it has to be
measured against what the Rust backend actually does today.

## 2. What was measured

`nikaia-std` as it stands, `rustc 1.97`, `opt-level = 3`, x86-64, callgrind;
the functions `#[inline(never)]` so the call boundary is visible. The shapes
are what `nikaia lower` writes for `fn one(n: i64) -> i64 throws` and its
neighbours.

| Return on success | Size | Comes back in | Callee instructions |
|---|---|---|---|
| `Result<i64, Thrown<Kind>>` (fieldless error) | 16 B | memory (sret) | 10 |
| `Result<i64, Thrown<Code>>` (`Code(u64)`) | 16 B | memory (sret) | 13 |
| `Result<(), Thrown<Kind>>` | 16 B | registers | – |
| `Result<&str, Thrown<Kind>>`, `Result<(i64, i64), …>` | 24 B | memory (sret) | – |
| pair `(i64, usize)`: second word 0 = success, else the tail word | 16 B | registers | 5 |

Findings:

* **Today ADR-023 D10 holds for no failing call that returns a value.** The tag
  sits in the error's own byte, and Rust returns such an enum through memory
  even at 16 bytes. `crates/nikaia-std/src/error.rs` asserts the size, not the
  way it is returned.
* **The two-scalar pair** saves 4 instructions a call net (21.4 M against
  17.4 M over 10⁶ calls) and the whole memory round trip.
* **`throwing` is not cold.** For `Thrown<Code>` LLVM inlines the trace
  machinery, and every call, failing or not, pays two pushes and a 168-byte
  frame (13 against 10 instructions).
* **Cranelift with its own convention (`tail`)** returns pointer, length and
  status in `rax`, `rcx`, `rdx`; the caller is `call; test; jnz`, no memory.
  With System V the status goes to memory.
* **An inlined call has no ABI.** Everything above concerns calls LLVM does not
  inline: across crates without LTO, recursion, large functions. How common
  those are in real programs is **not measured yet**.

## 3. Options

* **A as it is**: keep `Result<T, Thrown<E>>`. Breaks ADR-023 D10 as measured.
* **A with a two-scalar lowering**: in-value status, but written by the
  lowering as two plain words Rust returns in registers. Holds ADR-023 D10 for
  `T` up to one word; cannot hold it for a two-word `T`.
* **B, a status register**: on a backend that owns its convention. Holds
  ADR-023 D10 for `T` up to two words.
* **B in the carry flag**: one register and one instruction fewer than B on
  x86-64; reachable only where the throw and check sequences are hand-written.

## 4. Decision

### D1 — The throw path is cold, and the failing function stays inlinable.

`throwing` and every constructor of an envelope on the throw path are
`#[cold]` and `#[inline(never)]`. A function that can fail is **not** kept from
inlining: its body shrinks to a compare, a branch and a cold call, which makes
it a better inlining candidate, and an inlined call has no return convention
to pay for at all.

A test checks the code a failing call compiles to, not only its size: a
non-failing call to `fn one(n: i64) -> i64 throws` touches no stack memory.

### D2 — On the Rust backend, a failing call returns two scalars where the value and the error fit one word each.

Where `T` fits one word and `E`'s payload fits one word, the lowering writes
`(T-or-payload, tail)` instead of `Result<T, Thrown<E>>`. The tail word is
never zero (ADR-280 D25); zero is success. A crossing into Rust code the
program did not lower converts back to `Result` in a shim, which ADR-023 §3.1
already counts as ordinary at a boundary.

Built only after the measurement in §6 shows non-inlined failing calls carry
real weight in our programs and benchmarks.

### D3 — ADR-023 D10 is stated per backend.

ADR-023 D10's guarantee reads: **on the Rust backend**, a failing call
returning a value of up to one word pays no memory round trip; a two-word
value (a view, a pair) does, until the direct backend exists. **On the direct
backend**, the guarantee holds up to two words. The gap is named, not hidden.

### D4 — On the direct backend, the status is a register of its own (ADR-023 §3.1: B).

The Nikaia-internal calling convention of the direct path in issue #1235
returns the status beside the value, in a register: in Cranelift a further
return value under its own convention. The value keeps every return register
it would have without `throws`. Calls to and from Rust and C use their
conventions and meet through a shim; a program of Nikaia and `std` only has no
such boundary.

### D5 — The carry flag is a measurement, not a decision.

ADR-023 §4 rejected the carry flag because no backend we used could reach it.
Copy-and-patch can: the two sequences involved (`stc; ret` and `jc` after the
call) are short enough to write by hand as stencils, and nothing between `ret`
and `jc` touches the flags. A successful return then needs `clc` unless the
last flag write is known to leave carry clear (`xor`, `test`, an unsigned
operation proven or checked not to overflow).

The prototype in issue #1235, step 7, measures it against D4's register on the
same kernels. It is adopted on x86-64 only if it gains measurably; ARM64 and
RISC-V keep the register.

## 5. Reasons

* **The guarantee was not met, and nobody saw it**, because the test checks
  size, not convention. D1's test closes that.
* **D1 costs nothing and helps inlining.** The common small failing function
  gets faster by being inlined, which is better than any return convention.
* **D2 is the most Rust can give.** A two-word `T` plus a status is three
  words; Rust's convention has two registers for it. Only B gets past that.
* **D4 follows from issue #1235.** Designing the direct path's convention
  without the status would mean redesigning it later.
* **D5 keeps the question honest.** callgrind counts instructions, not µops;
  `test` and `jnz` fuse on current cores, so the carry flag's real gain is a
  register and perhaps nothing more. Only a measured prototype says.

## 6. Open

* **How many failing calls are not inlined** in our programs (the compiler
  through `lower-std`, the benchmarks, `tools/*.nika`), counted as
  `docs/measuring.md` says. Gates D2.
  **Counted (November 2026), too few for D2 to pay:** the compiler
  (`target/fast/nikaia`, after D1) holds 462 failing functions in
  `tools/*.nika`; 177 survive as functions of their own, reached from 176
  direct call sites, against 8 477 direct call sites of lowered functions in
  all. Under callgrind, `nikaia check` of `compiler/libraries.nika` (the first
  116 G instructions) made 9 490 calls to a failing lowered function out of
  202 M calls to lowered functions (0.005 %); `nikaia lower` of
  `benches/template.nika` made 3 975 of 184 k in 132 M instructions. At D2's
  4 instructions a call that is under 0.02 % of either run. The benchmarks
  fail outside `main` only in `game/fasta` (3 calls a run) and
  `game/k-nucleotide` (7). D2 stays unbuilt until a program shows weight.
* **The carry flag against the register**, issue #1235, step 7. Decides D5.
* **Whether the ledger narrows a call site's set to empty** where a proof makes
  the only `throw` unreachable; such a call could take the plain convention.

## 7. Considered and rejected

* **Forbidding inlining of failing functions.** It would put every small
  failing call back across a return convention, the one place the cost lives.
* **A as it is, ADR-023 D10 unchanged.** A normative guarantee that is not met
  is worse than a precise one that is.
* **The carry flag on Cranelift or LLVM.** Neither models a flag across a
  call; D4's register is their form of B.
* **A wider pair (three scalars) on the Rust backend.** Rust returns three
  words through memory; nothing is gained over today.

## 8. Consequences

* `crates/nikaia-std/src/error.rs`: `#[cold]` and `#[inline(never)]` on the
  throw-path constructors; a test on the compiled call, beside the size
  assertions.
* ADR-023: D10 rewritten per D3; §3.1 marked answered; §4's carry-flag entry
  points to D5.
* ADR-280 §6: the in-value versus out-of-band question marked answered.
* Issue #1235: the direct path's convention carries the status register (D4);
  step 7 adds the carry-flag measurement (D5).
* After §6's count: the two-scalar lowering (D2), with shims at Rust crossings.
