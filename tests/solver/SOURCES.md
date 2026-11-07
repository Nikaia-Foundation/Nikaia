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
  environment's proxy.
