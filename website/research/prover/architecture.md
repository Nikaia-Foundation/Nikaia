# How the prover is built

This page describes the prover's internal structure, what exists today, and
what is decided and not yet built. The decisions are in
[ADR-270](/docs/specification/adr/adr-270.html) for the solver and
[ADR-361](/docs/specification/adr/adr-361.html) for loops, and the numbers
that shaped them are in the [solver workload](/docs/solver-workload.html).
The questions this structure serves are under
[open questions about the prover](./).

## Three layers

The prover is three layers, and only the top one knows Nikaia
([ADR-270](/docs/specification/adr/adr-270.html) D1).

```text
  frontend      walks the program, collects facts, asks questions,
                turns answers into dropped checks and messages
      |
  logic layer   terms, queries, answers and the checker, with no Nikaia
                syntax, a package any package may use (D24)
      |
  solvers       search for a proof or a counterexample, never trusted
```

The frontend is part of the compiler (`prover_walk`, `prover_claims`,
`prover_loops` and their neighbours in `crates/nikaia-std/src/tools/`). It
reads comparisons, guards, lengths, what a callee promises and a type's rule,
and asks each question as a self-contained query made of facts, a goal, and
opaque ids for what they stand for (D3, D7). An answer is one of three. It is
proved, with a certificate, or refuted, with values that are checked by
evaluating them, or unknown.

The logic layer holds the terms, the normal form a query is keyed by, the
checker, and SMT-LIB and Alethe at its edge for comparing with other solvers
(D6, D19). Loops reach it as problems in constrained Horn clauses, which state
what holds when the loop is reached, one turn as a relation, each way out,
and what must hold after it ([ADR-361](/docs/specification/adr/adr-361.html)
D1).

## What a build trusts

No solver is trusted. A proof is used only after the checker has replayed its
certificate step by step, with clause steps, combinations of inequalities and
their integer rounding, case splits, and the definition of a quotient by a
constant (D5). The checker replays and does not search, which keeps it small.

What a build uses is the committed file `nikaia.proofs`, with one line per
question, keyed by the question's normal form, holding its answer and the
evidence. Every entry is checked when it is read. A normal build searches only
for what is missing, and `--locked` searches nothing and fails on any
difference (D19-D22). The search itself may differ between machines and runs,
and what one machine proved is not rejected by another (D4).

## The solver, today and as decided

Today's solver is the reference solver, which decides linear integer
arithmetic by Fourier-Motzkin elimination with case splits on demand and is
written in Nikaia (`logic_lia`, `logic_search`). Its successor is decided
piece by piece, and each step is measured before the next (D8).

```text
                    strategy layer (D14)
            features before the search, signals during it
                              |
                   CDCL(T) skeleton (D12)
        decisions, unit propagation, conflict analysis,
        backjumping, restarts, learned clauses
                              |
          +-------------------+--------------------+
          |                   |                    |
    fast paths          linear arithmetic     e-graph (D13)
    intervals,          simplex over the      equality with
    differences         integers, cuts        uninterpreted functions,
    (D12)               (D5, D11)             where theories meet
                                                   |
                                     +-------------+-------------+
                                     |                           |
                                  arrays                    bit-vectors
                          read-over-write lemmas         blasted into clauses
                              on demand (D13)                 (D13)
```

The skeleton is a CDCL loop over Boolean literals, and every theory sits
behind one interface. A theory takes a literal, propagates what it implies,
checks consistency, explains a propagation, backtracks, gives a model, and
names the terms it shares (D12). The compiler's own questions are tiny, with
at most four variables and mostly bounds on single variables or differences
between two, so fast paths answer those without entering the general search
and without allocating (D9, D12). Theories are combined through the e-graph
by model-based combination. A theory's model proposes equalities between
shared terms, the search decides them, and a theory that disagrees answers
with a conflict (D13). Numbers start as `i64` and fall back to big integers
and fractions only where a value outgrows 32 bits (D11). Clauses live in one
arena of `u32` words with watch lists that hold a blocking literal (D16).

| Part | Decided in | State |
| :--- | :--- | :--- |
| Three layers, query and answer, certificates and their checker | D1-D5 | built, in Nikaia |
| SMT-LIB and Alethe at the edge | D6 | built |
| `nikaia.proofs`, checked on read, `--locked` | D19-D22 | built |
| Loops as Horn clauses, invariants by closed forms and intervals, each checked | [ADR-361](/docs/specification/adr/adr-361.html) | built for whole numbers, more strategies to come (D5 there) |
| The solver's inner loops measured in Nikaia against Rust | D8 step 1 | measured, the gaps found are in the lowering of indexes ([solver workload](/docs/solver-workload.html), section 8) |
| Reference solver moved from Rust into Nikaia | D8 step 2 | done |
| `u32` variables, sparse rows, big numbers, theory interface | D8 step 3, D10-D12 | not built |
| CDCL(T) skeleton, simplex, fast paths | D8 step 5, D12 | not built |
| Cuts, local search, strategy layer | D8 step 6, D14 | not built |
| E-graph, combination, arrays, bit-vectors | D13 | decided, not built |
| Parallel search within one question | D8 step 7, D18 | not built |

The same parts, each with a bar for how far it is built, are under
[what is being built](./#what-is-being-built).

## Invariants from a language model or another program

A solver inside the compiler is not the only place a proof can come from. An
outside program, a language model among them, may propose a loop invariant,
and the compiler uses it only after checking it like one of its own
([ADR-361](/docs/specification/adr/adr-361.html) D6).

```text
  nikaia prove --propose
      |  one JSON line per open loop (problem in SMT-LIB, source, names)
      v
  proposer          any program named in the manifest
      |  candidate invariants
      v
  checker           replays each candidate, keeps what holds
      |
      v
  nikaia.proofs     read by the next build, which calls no proposer
```

Only `nikaia prove --propose` starts a proposer, never `build`, `run` or
`test`, so the program's code leaves the machine only through that command
and the proposer the manifest names. A wrong proposal costs time and nothing
else. The proposer is a child process, as language servers are, so what a
provider, a model, a key or a prompt is stays out of the compiler. This is
decided and not yet built, and what it adds is asked under
[open questions about the prover](./#what-can-a-language-model-add-to-the-prover).

## What the language has to offer the solver

The solver is written in Nikaia, so building it is also a test of the
language, with arenas of `u32`, flat loops over lists, and in-place changes
to one field of the solver while another is read. A probe of the forms a
CDCL(T) core and an e-graph need found most of them working and a few
defects in how they are lowered, each filed with its probe. They are
[#1206](https://github.com/Nikaia-Foundation/Nikaia/issues/1206) (an argument
that reads the list whose element a call changes),
[#1207](https://github.com/Nikaia-Foundation/Nikaia/issues/1207) (the value
`or_insert` hands back),
[#1208](https://github.com/Nikaia-Foundation/Nikaia/issues/1208) (a part of
`mut self` handed to a `keep` parameter),
[#1209](https://github.com/Nikaia-Foundation/Nikaia/issues/1209) (a part of a
list handed to a function and changed there),
[#907](https://github.com/Nikaia-Foundation/Nikaia/issues/907) (`contains`
with a computed argument) and
[#591](https://github.com/Nikaia-Foundation/Nikaia/issues/591) (a declared
struct as the key of a map). The work on the solver waits for these rather
than writing around them.
