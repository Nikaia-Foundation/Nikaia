# ADR-proof-kani-v1: Kani checks the prover's proofs on the emitted Rust, in a mode of its own

**Status:** Accepted
**Date:** 2026-11-22
**Answers:** Can a second, independent tool check the proofs that let the
compiler drop a bounds or overflow check, without anyone paying for it who does
not run that check? (the owner, 2026-11-22; the work is the issues in section 5)
**Supersedes / Related:**
* [ADR-306](adr-306.md): a proved check is dropped (D26); a dropped check
  lowers to a `proven` call behind a `debug_assert` (D19), which stops a wrong
  proof "in every build that keeps debug assertions"; every level is held to
  the same output (D11).
* [ADR-002](adr-002.md) D5: a Nikaia build has `debug-assertions = false`,
  with no key to change it. Not changed here: section 4, D7.
* [ADR-361](adr-361.md): a loop's invariants are found (D5), each one used is
  checked (D4) and kept in `nikaia.proofs` with its origin; a `for` counts its
  turns (D13).
* [ADR-391](adr-391.md): the callers' facts become a function's facts (D1),
  its frame is derived (D2), both are kept in `nikaia.proofs` (D3); derived is
  not promised (D4).
* [ADR-270](adr-270.md): the prover; a proof read from `nikaia.proofs` is
  checked again when read (D19, D20).
* [ADR-385](adr-385.md) and [ADR-306](adr-306.md) D12: a length is at most
  2^63-1, and below 2^60 where its elements take space; these are axioms.
* [ADR-001](adr-001.md) D1: one toolchain builds; a checker on another
  toolchain (Miri today) checks and builds nothing that ships.
* [ADR-platform-layer-v1](adr-platform-layer-v1.md): new code in this mode is
  Nikaia; the emitter is `.nika`.

## 1. Question

Should the proofs behind every dropped check be checked a second time, by a
tool that shares no code with our prover, and if so, how is that tool handed
what the prover knows, and who pays for it?

Today a dropped check rests on one prover. A wrong proof is undefined behaviour
in every build (ADR-306 D19), and the only second line ADR-306 names, a build
that keeps debug assertions, is never run: ADR-002 D5 switches them off in
every build, the suite's included. A wrong fact is found only where a test
happens to take the path that breaks it. The question has to be decided here
because the answer touches the emitter, which every build runs, and the CI,
which every push runs.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Kani (Rust, bounded model checker on CBMC) | Checks a Rust crate for panics, overflow and undefined behaviour (an unchecked add that overflows, `get_unchecked` past the end); function contracts (`requires`, `ensures`, `modifies`) and loop contracts (`loop_invariant`, `loop_modifies`, `loop_decreases`) turn a bounded check into an unbounded one where they hold | Nightly toolchain of its own; `-Z function-contracts -Z loop-contracts` are unstable; loop contracts on `for` only over array, slice, `Iter`, `Vec`, `Range`, `StepBy`, `Chain`, `Zip`, `Map`, `Enumerate`; invariants are not checked for side effects; a missing `loop_modifies` target fails the proof | github.com/model-checking/kani, `CHANGELOG.md` (0.63 contracts compose, 0.64 `loop_modifies`, 0.66 `for` loops, 0.68 `loop_decreases`) and `docs/src/reference/experimental/loop-contracts.md` |
| SPARK (Ada) | GNATprove proves the contracts; the same contracts compile to run-time checks (`-gnata`) for the tests | A second build with checks on for testing; proofs and tests share one contract language | from memory, not checked |
| Frama-C (C) | One contract language (ACSL); WP proves it deductively, E-ACSL turns it into run-time checks | The contracts are written by hand | from memory, not checked |
| Creusot, Prusti, Verus (Rust) | Deductive proof of a Rust crate with specifications in their own attributes or dialect | Every `std` function used needs a specification; Verus needs its own dialect of Rust | from memory, not checked |
| CompCert (C) | The compiler itself is proved correct in Coq | Years of proof work per compiler pass | from memory, not checked |

What does not transfer: SPARK and Frama-C hand-write their contracts, while
ours are found by the prover and never written by the programmer. Creusot,
Prusti and Verus would need a specification for every `nikaia_std` function
the emitted code calls. CompCert proves the compiler; we want the facts of one
program checked. Kani is the one that takes the emitted crate as it is and
needs only the facts we already have.

## 3. Options, weighed

Under every option the programmer writes the same program; the options differ
in what the compiler emits and what the CI runs. The example is a loop whose
index check the prover drops (ADR-306 D3):

```nika
fn total(xs: ref Vec[i64]) -> i64 {
    let mut sum = 0
    for k in 0..<xs.len() {
        sum = sum.wrapping_add(xs[k])
    }
    return sum
}
```

Today's Rust reads `proven::read(&xs, (k) as usize)` inside
`for k in 0..xs.len() as i64 { … }`.

### Option A — nothing

The prover stays the only check; ADR-306 D11's tests find what they reach.

### Option B — a build with debug assertions

Run the suite once with debug assertions on: every `proven` call checks its
`debug_assert`, so a wrong proof on a path a test takes stops the test.

### Option C — Kani on the emitted crate, without our facts

Kani unwinds every loop to a bound and checks the dropped operations; it
proves nothing past the bound and knows none of the prover's invariants.

### Option D — Kani with our facts as contracts, the `for` kept

In this mode only, the emitter writes what the prover proved as contracts on
the same code:

```rust
#[cfg_attr(kani, kani::requires(xs.len() <= (1usize << 60)))]
fn total(xs: &Vec<i64>) -> i64 {
    let mut sum: i64 = 0;
    #[cfg_attr(kani, kani::loop_invariant(0 <= k && (k as usize) <= xs.len()))]
    for k in 0..xs.len() as i64 {
        sum = sum.wrapping_add(*unsafe { nikaia_std::proven::read(&xs, (k) as usize) });
    }
    sum
}
```

Kani must prove each invariant on entry and per turn, and then checks
`proven::read` against it for every length, not up to a bound.

### Option E — as D, but the loop written as `while` in this mode

Kani's loop contracts are older for `while`; the mode would rewrite every
`for` into a `while` with its own counter.

### Option F — every loop written as `while`, in every build

As E, but the `while` is the only form, so the checked code is the shipped one.

### Option G — a deductive Rust verifier (Creusot, Prusti, Verus)

### Weighed

| | D1 the special case pays, not the common one | D2 the compiler works, not the developer | D3 no cost for the common case | D4 consistent with itself | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | nobody pays | yes | none | ADR-306 D19 names a build that does not exist | a wrong proof is undefined behaviour in a user's program, found by the user |
| **B** | the CI pays one run | yes | none: builds keep ADR-002 D5 | makes ADR-306 D19's build real | finds only the paths the tests take |
| **C** | the CI pays | yes | none | checks the code but not the proof | proves nothing past its bound; finds what B finds and little more |
| **D** | only a run in this mode pays; normal output unchanged | yes: the facts exist, the emitter writes them | none: the contracts are written only in the mode | checks exactly the facts the drop rests on, on the shipped loop form | Kani's contracts are unstable: a release can break the mode, never a build |
| **E** | only the mode pays | yes | none | Kani proves a different program from the one shipped | a defect of the `for` lowering goes unseen |
| **F** | every program pays for a form chosen for a checker | yes | a counter and a bounds test where the range iterator had none | consistent | every build's code changes for a tool most never run |
| **G** | the mode pays | no: `nikaia_std` needs a specification per function | none | a second specification language beside ours | a large cost before the first result |

Two weights decided: D3 rules out F, and D4 rules out E and C (each checks
something other than what ships, or other than what the drop rests on). A and
B alone leave a wrong proof on an untested path in place. D and B together
check both the facts (D, for all inputs where Kani reaches) and the paths the
tests take (B), and only the run that asks for them pays.

## 4. Decision

### D1 — Kani is a mode, switched on by its own switch

The compiler has a Kani mode, switched on only by its own switch on `nikaia
test` (and the lowering it drives). Without the switch the emitted Rust is
byte for byte what it is without this decision, which a test holds. No build,
no test run and no setting other than the switch writes anything for Kani.

### D2 — loop facts become `loop_invariant` on the unchanged `for`

In the mode, each invariant the prover used for a loop (ADR-361 D4, read from
`nikaia.proofs` or from the walk that made it) becomes a
`#[kani::loop_invariant(…)]` on that loop, in the loop's own variables. The
loop keeps the form every build emits (`for k in 0..n as i64`, `for x in
xs.iter()`); the mode rewrites no loop.

### D3 — function facts become contracts, frames become `modifies`

A function's facts from its callers (ADR-391 D1) become
`#[kani::requires(…)]`; what the prover proved of its result becomes
`#[kani::ensures(…)]`; its derived frame (ADR-391 D2) becomes
`#[kani::modifies(…)]` on the function and `#[kani::loop_modifies(…)]` on each
loop. A call is replaced by its contract only where Kani has proved that
contract (`stub_verified`).

### D4 — a fact is an obligation, never an assumption

Every fact the prover proved is written as something Kani must prove: an
invariant, an `ensures`, the `requires` of a callee at its call. Only two
things are assumed: the axioms of the language (ADR-385, ADR-306 D12) and a
function's own `requires` inside its harness. No `kani::assume` stands for a
fact the prover found.

### D5 — a loop Kani's contracts do not cover is unwound and reported

A loop over a form outside Kani's list (a map's `values_mut()`,
`nikaia_std::par::listed`, an iterator of `nikaia_std`'s own) gets no
contract; Kani unwinds it to the run's bound, and the run reports each such
loop by file and line as checked to a bound, not by contract.

### D6 — harnesses are generated under `cfg(kani)`

In the mode, the emitter writes a `#[kani::proof_for_contract(f)]` harness for
each function with a contract whose parameters Kani can choose (`kani::any()`
for integers and `bool`, a `Vec` up to the run's bound), and the crate's
`#![cfg_attr(kani, feature(stmt_expr_attributes, proc_macro_hygiene))]`. A
function it cannot write a harness for is reported, as in D5. Everything the
mode adds sits under `cfg(kani)` or `cfg_attr(kani, …)`, so a crate emitted in
the mode still builds with `cargo build`.

### D7 — one run on `main`: debug assertions, then Kani

A workflow runs on each commit `main` is moved to, and on demand, beside and
never in front of the CI that moves it. It has two legs:

1. **Debug assertions**: the suite, built and run with
   `RUSTFLAGS="-C debug-assertions=on"`. This is the build ADR-306 D19 names.
   ADR-002 D5 stands: no Nikaia setting turns debug assertions on, and no
   other run sets the flag.
2. **Kani**: the tests' emitted crates in the Kani mode, checked with
   `cargo kani -Z function-contracts -Z loop-contracts` on a pinned Kani
   release of at least 0.68.

The CI on `develop` stays as it is. Kani's nightly toolchain checks and
builds nothing that ships (ADR-001 D1, as Miri does).

### D8 — what fails is a prover defect

A failed invariant, contract or `debug_assert` in this run is a defect of the
prover (a fact it should not have proved) or of the mode (a fact written
wrong), filed as a bug with the program, the fact and Kani's or the
assertion's message. A failure is never answered by taking the fact out of the
mode.

## 5. Consequences

* A wrong proof now has two ways to be found besides a test that crashes: the
  debug-assertions leg on every path the suite takes, and Kani on every input
  up to its bound or, where the contracts hold, every input.
* The emitter grows a second output for the same tree. It is paid for only in
  the mode; the byte-identical test of D1 keeps it out of every other build.
* The main-only run takes time and disk on its own runner; nothing waits on it.
* The mode depends on unstable Kani features. A Kani release that breaks it
  breaks the main-only run, never a build or the `develop` CI.
* Not every loop is covered by contract (D5); the run says which.
* The work, in order:
  * Kani loop contracts on our emitted Rust: a probe by hand (#2720)
  * Kani mode in the emitter: the switch, harnesses and a byte-identical guard (#2721)
  * Kani mode: loop invariants from the prover (#2724)
  * Kani mode: function contracts and frames from the prover (#2725)
  * The suite with debug assertions on, on `main` (#2726)
  * Kani over the tests' emitted crates, on `main` (#2727)

## 6. What this does not decide

* Contracts for `nikaia_std`'s own iterators (a `KaniIter` under `cfg(kani)`):
  it would be new Rust in `std`, asked when D5's report shows how many loops
  it would cover.
* Whether the Kani leg moves into the `develop` CI: asked once the main-only
  run has a record of its time and its findings.
* A Kani mode for user programs (`nikaia check --kani` outside the suite):
  asked after the main-only run.
