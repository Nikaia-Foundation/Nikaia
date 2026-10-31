# A formally verified compiler? Decision basis

Owner's question, 2026-10-24, [#796](https://github.com/Nikaia-Foundation/Nikaia/issues/796). Nothing concrete
needs the answer now: no user, program or measurement waits on it. It is a
question of direction, asked because self-hosting (#125, about 69 % today)
puts the compiler's own code within reach of the language's own proofs.

## The topic from zero

Nikaia proves things about the programs it compiles. In

```nika
fn clamp(x: i64, lo: i64, hi: i64) -> i64 sync {
    assert(lo <= hi)
    let mut r = x
    if x < lo { r = lo }
    if x > hi { r = hi }
    assert(r >= lo && r <= hi)
    return r
}
```

`nikaia lower --asserts` reports both claims *proved*: the first at every call
the compiler sees, the second from the branches. No check is emitted for
them. A claim the prover cannot show refuses the build (`NK1300`, ADR-380); an
index or an arithmetic step it cannot show keeps its run-time check
(`--bounds` lists which). The proof is not taken on the solver's word: a
small checker replays a certificate (LRAT-like clause steps, Farkas
combinations, integer rounding; ADR-270 D5) before a check is dropped.

A **formally verified compiler** is something else: a machine-checked proof
that the compiler *preserves meaning* - every behaviour of the compiled
program is a behaviour the source program allows. The question is how much of
that the language's own proofs, applied to the compiler's own Nikaia code, can
reach, and what an honest name for the result is.

## 1. What the term means in the field

| Work | What is proved | Trusted base | Cost |
| :--- | :--- | :--- | :--- |
| CompCert (Coq) | semantic preservation, C subset to assembly, for the optimising passes | Coq's kernel, the formal C and assembly semantics, assembler/linker, an unproved front part | about 6 person-years first, ~100 k lines of proof (from memory) |
| CakeML (HOL4) | end-to-end, ML to machine code, and the compiler bootstrapped *inside* the logic | HOL4's kernel, the ISA model | a decade, several groups |
| Vellvm (Coq) | a formal semantics of LLVM IR, some passes verified | Coq | research, partial |
| seL4 (Isabelle) | a kernel's C proved against its spec, and the binary checked against the C by *translation validation* of gcc's output | Isabelle, the spec, the hardware model | about 20 person-years |
| Alive2, Cranelift's ISLE checks (Crocus), Wasm validators | translation validation or rule-by-rule checks with an SMT solver: each optimisation or lowering rule, not the whole compiler | the solver, the IR semantics written for the tool | small per rule |
| SPARK "silver", Dafny, F* | *programs* proved free of run-time errors or correct against contracts; not compilers | the verifier and its compiler | per program |

Three claims are honest and different: **(a) semantic preservation** (CompCert,
CakeML); **(b) absence of run-time errors** in the compiler's code (no panic,
overflow, out-of-range index; SPARK's silver level); **(c) verified
components** (a certified checker, a validated parser, a rule checked per
build). Only (a) is what the field calls a *formally verified compiler*.

**Our trusted base** today: rustc and LLVM (unverified, with known
miscompilations), the lowering of Nikaia to Rust, the Rust half of the
compiler (~31 %) and of `nikaia-std` and the runtime, the crates (`winnow`,
`clap`), the certificate checker, the OS.

**Is there a formal semantics?** No. Parts I-III (about 6,700 lines) are
precise prose: rules and refusals, not a grammar in a formal notation, typing
judgements or an operational semantics. Part of a program's meaning is
defined by its lowering to Rust, and Rust has no complete formal semantics
either (MiniRust and RustBelt cover cores). So claim (a) has today no
statement it could be proved against; that is the first missing piece, not
the proofs.

## 2. What the language proves today (probed on `target/fast/nikaia`)

* Linear integer arithmetic over bindings that do not change, `let mut`
  followed through assignments, loop invariants (ADR-361), struct rules
  (ADR-369), preconditions at every call (ADR-269 D5), `comptime` values;
  refutation with a model (`NK1207`); certificates checked before use.
* Not reached: an `if` used as a value (`let r = if .. { lo } else ..` gives
  *`r` is not a whole number the prover reads*), quantifiers over lists,
  recursive definitions and induction, strings, heap shapes, termination,
  postconditions (`ensures`), lemmas.
* On the compiler's own code, measured on the tree of the commit that
  builds ADR-385 D1 (#1053, on 9bdbd5a) with
  `nikaia lower-std --bounds --asserts`, which checks each of the 247
  `tools/*.nika` files with the rest of the package beside it, as
  `lower-std` does (#1053), summed by `scripts/proved.nika`: **2,677
  indexes, 1,544 proved (57.7 %); 1,633 arithmetic steps, 923 proved
  (56.5 %)**, no file refused. With `--optimization prove-through-loops:on`:
  1,576 indexes proved (58.9 %), 929 steps (56.9 %). The tools hold no
  `assert` yet (`claims: 0 asserts`) and 3 `panic(` calls in 3 files, one
  of them in text the lowering writes for a test build. So the checks left
  at run time number 1,846, and 1,808 with loops proved through: that
  figure is the lane's target, and 0 would be claim (b). Most of what stays
  checked is a value no fact bounds: of the 1,843 unproved sites, 708 read
  a counter, a loop or a length without a fact, 396 are a counter a loop
  changes, and 382 a parameter no precondition bounds (#432, #383, #388).
  An earlier count with `nikaia check --bounds` on each file alone listed
  fewer sites, because `check` refuses a file that names another file's types
  and the code it cannot type drops out. The rest keep their checks: a defect
  there stops the compiler, it does not miscompile silently. The `unsafe` in `tools/` is
  text the lowering writes, not code the tools run.
* Where the bounds walk stops today (prover lane, 2026-10-31; work in #1053):
  it states a goal only below 2^62 (`solvable()` in `bounds_walk.nika`, #444),
  so a bound like `n <= 2^63 - 1` cannot prove `i + 1`; what proves it is the
  relative goal against an `i64` (`bounding_relatives`), and every length is
  such an `i64` (ADR-385 D1). It keeps no type for a local, only for
  parameters. Its loop guesses are halves of the facts the loop starts with,
  never the prover's relational closed forms (`s == i`), and its `if` join
  forgets every name a branch changes. A list's length is known where the
  checker records it (`std_lengths` / `sized_lengths`: a `.len()` receiver,
  a `for`'s collection), and below 2^60 only where its elements take space
  (ADR-306 D12), which now includes a struct another tools file declares.
  `nikaia check --bounds FILE` checks a file alone (another file's struct is
  unsized there); only `lower-std --bounds` checks it with the package.

## 3. Requirements and their category

A reachable now with our tools; B some extensions; C large rebuilds of tools
or function, huge effort; D only with external tools (proof assistants, SMT
solvers) or major rework; E probably never.

| # | Requirement | Cat. | Why |
| :--- | :--- | :--- | :--- |
| 1 | Count of proved / checked indexes, overflows and asserts in the compiler, tracked per commit | A | `--bounds` / `--asserts` exist; a sum over `tools/` (above) |
| 2 | Differential testing: Rust stage against Nikaia stage | A | what every self-hosting move already does with the suite |
| 3 | Stage 2 fixpoint: the lowered compiler lowers itself byte-identically; reproducible build | A/B | ADR-294 D5; reachable once Stage 1 is |
| 4 | Absence of run-time errors in the compiler's Nikaia code (claim b) | B | prover extensions (`if` values, lengths and counters #432 #383 #388, `push`/`len`), then each remaining check proved or turned into a guard; incremental, rides with the moves |
| 5 | Absence of run-time errors in the certificate checker (the trusted kernel) | B | small and leaf-shaped; first candidate for item 4 |
| 6 | Parser against a formal grammar: grammar written down, round-trip `parse(fmt(t)) == t` as a property test | B | grammar text plus fuzzing (14.3); a proof of the parser is D |
| 7 | Local functional contracts on components (struct rules, preconditions on the tree, spans nest) | B | ADR-369 rules and ADR-269 D5 exist; need postconditions |
| 8 | Contracts over collections and recursion (the checker's tables are consistent, a walk visits every node); the ladder from a named property (B, ADR-381) to rules over every element is [`facts-about-data.md`](facts-about-data.md) | C | needs quantifiers, inductive predicates, `ensures`, lemmas, termination measures: a Dafny-level verifier in our own tool |
| 9 | Reference semantics (optional, not needed for claim b): an executable reference interpreter declared the meaning, compiled output compared with it | B/C | today's `interpreter` evaluates nothing (Stage 0); a new one, rule by rule ([`formal-semantics.md`](formal-semantics.md), level 2) |
| 10 | Mathematical semantics of Nikaia (formal grammar, typing, operational rules) | C/D | rewriting Parts I-II as a formal object; mechanised only in a proof assistant; levels and examples in [`formal-semantics.md`](formal-semantics.md) |
| 11 | Soundness of the checker (well-typed programs do not go wrong; contracts imply rustc's borrow checker accepts) | D | needs item 10 and a proof assistant; rustc's borrow checker is a backstop for memory safety meanwhile |
| 12 | Translation validation of the lowering per build (each function's Rust equals its Nikaia) | D | needs items 10 and 13 and an equivalence checker; per-rule validation (Alive2/ISLE style) for arithmetic and indexing alone is C |
| 13 | A formal semantics of the Rust the lowering writes | D | MiniRust/RustBelt are partial; a model of our narrow safe subset in a proof assistant |
| 14 | Semantic preservation of the lowering, CompCert-style (claim a) | D | items 10, 13 plus a proof per lowering rule; person-years. Inside our LIA prover it is E: that would turn it into a higher-order proof assistant |
| 15 | The Rust half of `nikaia-std` and the runtime | D | Kani, Verus, Creusot, Prusti, per function |
| 16 | rustc and LLVM in the trusted base removed | E | not ours; seL4-style validation of the binary is D at best, and a new back end moves the problem |
| 17 | Trusting trust beyond the fixpoint (diverse double compiling against an independent compiler) | D/E | needs a second independent Nikaia compiler; rustc stays the root |

## 4. Honest claims short of (a)

* **Now (A):** "the compiler's Nikaia code proves 58 % of its indexes and 56 %
  of its arithmetic (on ae09d77, `nikaia lower-std --bounds --asserts`, §2) and
  checks the rest, 1,846 checks at run time; no unproved `assert` builds; every
  dropped check rests on a checked certificate"; "Stage 1 and the Rust stage
  agree on the whole suite".
* **With extensions (B):** "the proof kernel is free of run-time errors"
  (item 5), then "the compiler's Nikaia code is free of run-time errors"
  (item 4, claim b), "parser round-trip checked", "compiled programs agree
  with the reference interpreter".
* **Not to be said:** "formally verified compiler", until item 14 exists.

## 5. Recommendation

Do not aim at claim (a) now: it needs a formal semantics of Nikaia and of the
Rust we write, and an external proof assistant (items 10-14), which is a
second project of CompCert's size and would stall self-hosting. Do aim at
claim (b), because it is the language's own promise applied to itself, it
needs only prover extensions every user also gets, and it moves with the
self-hosting work rather than against it.

**First cheap step** (A, no compiler change): a small Nikaia program
(`scripts/proved.nika`, run with `nikaia run`, CLAUDE.md's helper-tool rule)
that sums the `--bounds` and `--asserts` counts over `tools/*.nika` and
prints one line, recorded with each self-hosting snapshot, so the share of
proved checks is visible beside the share of moved lines and a regression
shows. Everything else waits for the owner's answer.

The counting tool exists for `--bounds` (#1053): `nikaia lower-std --bounds
[--optimization NAME:LEVEL]... > REPORT`, then `nikaia run scripts/proved.nika
-- REPORT` prints the totals, the unproved sites by reason and the 15 files
with the most of them; with `lower-std --bounds --asserts` it also sums the
claims and the `panic` calls into `left at run time`, the lane's target 0
(#1053).

## 6. Owner's decisions (2026-10-24, #796)

* **Claim (b) is the goal**: the compiler's Nikaia code proved free of
  run-time errors. It rides with stage 1: the counting tool first, then the
  prover extensions (#386, #432, #388) as compiler work in the serial
  queue, the certificate checker (item 5) as the first file brought to zero
  unproved checks.
* **No "formally verified" in `README.md`, the website or `guide/`** until item
  14 exists; the honest claims of section 4 may be used once they are true and
  measured.
* **A formal semantics (item 10) is open**: the basis is
  [`formal-semantics.md`](formal-semantics.md).

## 7. What claim (b) needs, and what it does not

Claim (b) needs the prover and its checked certificates, applied to
`tools/*.nika`, and nothing that runs the program: no interpreter, no formal
semantics. What it needs beyond today is prover reach - `if` values, lengths
and counters (#386, #432, #388), then facts about what the compiler's tables
hold (an index stored in a node points into its table; [`facts-about-data.md`](facts-about-data.md),
#797, #798). An interpreter (item 9, `formal-semantics.md` level 2) is an
optional later stage, a test oracle for the lowering, not a step on this path.
