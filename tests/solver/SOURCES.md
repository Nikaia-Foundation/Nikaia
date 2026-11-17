# The measuring rod's sets (ADR-270 D17, #1281)

Run with `nikaia run scripts/solver.nika -- run tests/solver` (the whole) or
one directory. A file's expected answer is its `(set-info :status ...)`, else
its `; EXPECT:` line.

| Directory | What | Origin | Licence |
| :--- | :--- | :--- | :--- |
| `smoke/` | three tiny files: one sat, one unsat, one the reader refuses | written here | the repository's |
| `pre/` | four tiny files the preprocessing (#1283 item 5) decides: gcd, fixed column, tightening | written here | the repository's |
| `stress/` | D9's four families plus `blank = 2·n`, n = 1..12 (parity 2..12) | `scripts/solver.nika gen`, deterministic; status from z3 5.3.3 | the repository's |
| `qflia/` | QF_LIA, non-incremental, under 64 KiB | see below | see below |
| `qflra/` | QF_LRA, non-incremental, under 64 KiB | see below | see below |
| `sample/` | the fixed 50-file sample of the SMT-LIB set below (25 QF_LIA, 25 QF_LRA), run in CI by `tests/compiler/tests/solver_sample.nika` (#1338) | `scripts/solver.nika sample target/smtlib tests/solver/sample 25` | per family, see below |
| `compiler/` | the compiler's own questions (D9's set, #1285 item 3): 4884 distinct normal forms the bounds walk and the prover ask over the tools package, 3014 `unsat`, 1854 `sat`, 16 with no status (unknown to both engines); file name the `nikaia.proofs` key | `NIKAIA_DUMP_QUERIES=DIR nikaia lower-std --bounds`, status the compiler's checked answer (Fourier-Motzkin, then the new engine) | the repository's |
| `bugs/` | disagreements `scripts/solver.nika fuzz` finds, with z3's status (none yet: 300 queries of seed 1 agree, over Fourier-Motzkin and the CDCL(T) engine) | generated | the repository's |

## `qflia/` and `qflra/`

* **cvc5 regression tests**: <https://github.com/cvc5/cvc5>, commit
  `00147fb17d7ed9046f3b09deee0b4951c0243b3f` (2026-11-05), `test/regress/cli/`,
  modified BSD (BSD-3-Clause, cvc5's `COPYING`). Taken: every `.smt2` whose `set-logic` is
  `QF_LIA` (23 of 103) or `QF_LRA` (19 of 82) with exactly one `check-sat`, no
  `push`, `pop` or `check-sat-assuming`, and under 64 KiB. The file name is
  the path below `cli/` with `/` written `-`. Status: the file's own
  `:status` or `; EXPECT:` line; where it had neither (`regress0-nl-issue8755-*`,
  `regress0-expect-scrub.*`, `regress0-parser-issue10813-1`,
  `regress1-arith-div.09`, `regress1-arith-mult.02`), z3's answer was written
  as `(set-info :status ...)` by `scripts/solver.nika oracle --write`, the only
  change to the files. z3 agrees with every stated status.
* **pysmt**: <https://github.com/pysmt/pysmt>, commit
  `c97c1894745f4fd4f737a3e7c452a2f3d7675203` (2026-10-06),
  `pysmt/test/smtlib/small_set/QF_LIA/`, Apache-2.0. Taken: `issue_159.smt2`
  (decompressed, as `pysmt-issue_159.smt2`, status from z3). Not taken: the 12
  `prp-*.smt2` (SMT-LIB's own), 150-190 KiB each decompressed, over the
  64 KiB limit.
* **Not reachable**: the SMT-LIB release on zenodo answers 403 through this
  environment's proxy; the set below is fetched from GitHub instead.

## The SMT-LIB set (`target/smtlib/`, not committed) and `sample/`

`nikaia run scripts/solver.nika -- fetch [DIR]` (default `target/smtlib`)
clones each source below at its fixed commit (blobless, sparse: only the
listed paths), and copies into `DIR/qflia/FAMILY/` and `DIR/qflra/FAMILY/`
every `.smt2` whose `set-logic` is `QF_LIA` or `QF_LRA` with exactly one
`check-sat`, no `push`/`pop`/`check-sat-assuming`, a `:status` of `sat` or
`unsat`, and at most 1 MiB (a larger file is refused before it is read). A
file whose content an earlier source already gave is left out. FAMILY is the
SMT-LIB family directory (below `non-incremental/LOGIC/`), else the source's
name. The run prints, per source, how many it took and why it left the others
out. Fetched on 2026-11-17: 578 QF_LIA, 519 QF_LRA.

| Source | Repository, commit | Paths | Taken | Licence |
| :--- | :--- | :--- | ---: | :--- |
| `seeds` | <https://github.com/testsmt/semantic-fusion-seeds>, `93029d5e832fbba80dcda7e6032e60fbc861ed39` | `QF_LIA/*`, `QF_LRA/*` | 663 | no licence file; most files are SMT-LIB's and carry `:license` CC BY 4.0 |
| `smtlib-2022` | <https://github.com/SMT-LIB/benchmark-submission-2022>, `81a39628827f61fbd74c6f3e67feb5c1b740d2f0` | `non-incremental/QF_LIA/20220307-SMPT/*/RC-00.smt2`, `RF-00.smt2` (one file of each kind per model) | 362 | CC BY 4.0 (each file's `:license`) |
| `smtlib-2023` | <https://github.com/SMT-LIB/benchmark-submission-2023>, `7f993a017850ba547c6ea19a1bf3b54adf54a449` | `non-incremental/QF_LIA/*`, `QF_LRA/*` | 0 (2 without status) | CC BY 4.0 |
| `smtlib-2024` | <https://github.com/SMT-LIB/benchmark-submission-2024>, `02e00a1ad661405c438059820d82a5cf5ef11569` | `non-incremental/QF_LIA/*`, `QF_LRA/*` | 31 (49 over 1 MiB) | CC BY 4.0 |
| `z3test` | <https://github.com/Z3Prover/z3test>, `b09a2011aabe656b4a4f22c3bf13c883563fa8e1` | `regressions/smtlib-sample/QF_LIA/*`, `QF_LRA/*` | 41 | MIT (`LICENSE.txt`); the files are SMT-LIB's, CC BY 4.0 |

QF_LIA reaches 500 only with the SMPT family: 362 of its 578 files are SMPT
(Petri-net reachability, RC and RF of each model). The 2022 submission holds more
SMPT queries per model (`RC-01` ...); only `RC-00` and `RF-00` are taken, so
the family does not swamp the set further.

`sample/` is `nikaia run scripts/solver.nika -- sample target/smtlib
tests/solver/sample 25`: per logic, the files of at most 64 KiB sorted by
path, 25 taken at even steps, the family directory kept. The files are copied
unchanged.

The comparison (`scripts/solver.nika compare`) runs z3 5.3.3 (the one on
`PATH`) and cvc5 1.4.2 (the static Linux binary of
<https://github.com/cvc5/cvc5/releases/tag/cvc5-1.4.2>), named by `$Z3` and
`$CVC5` or found on `PATH`; the results are in `docs/solver-workload.md`.
