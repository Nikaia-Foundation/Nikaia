# Prover lane: state

*Current state only, rewritten by the lane's architect each run. What happened is in `git log` and the issues.*

**Goal (owner, 2026-10-31):** 100 % of the indexes and arithmetic steps in `crates/nikaia-std/src/tools/*.nika` proved. This is claim (b) of `docs/formal-verification.md`. Epic: #1053 (`wip`). Then come the gaps outsiders name (lengths, sorted lists, facts about data, postconditions, lemmas). After that, by the score: the prover as a product, interfaces for outside proposers (ADR-361 D6 already decides `nikaia prove --propose`), and current research.

**The headline counts at `prove-through-loops:on`** (architect, 2026-10-31). Claim (b) is about what is proved, and that is the strongest setting the toolchain offers. Which proved checks a build drops is a separate setting. Both figures are recorded.

## Figure (2026-10-31, 1ebd8fc, `lower-std --bounds` + `scripts/proved.nika`)

| Setting | Indexes | Operations |
| :--- | :--- | :--- |
| `off` | 1,493 / 2,583 (57.8 %) | 712 / 1,594 (44.7 %) |
| `prove-through-loops:on` | 1,525 / 2,583 (59.0 %) | 719 / 1,594 (45.1 %) |

4 files were refused (`modules`, `project_members`, `project_settings`, `sysroot_cache`), because the binary (12:25) predates the commits that call `std` `hash::sha256_hex` (578fd67). A fresh binary counts them. Reasons under `off`: 1,862 catch-all, 60 call results (#382), 37 lists of lists (#385), 6 products (#386), 4 lower bounds, 3 fields (#384). p3 splits the catch-all.

## Lane facts (for the architect and briefs)

* Tools sources: `crates/nikaia-std/src/tools/*.nika`. Count: `target/fast/nikaia lower-std --bounds [--optimization prove-through-loops:on] > R`, then `target/fast/nikaia run scripts/proved.nika -- R`. It takes about 50 s. Never count with per-file `check --bounds`: files checked alone are refused.
* `tests/compiler` is one project: no test can set `--optimization` or show `--bounds` output. Show them with `check --bounds FILE` on a probe outside `tests/compiler`. A `--asserts` result can be a test.
* The bounds walk (`bounds_walk.nika`) is separate from the prover walk (`prover_walk.nika`, `prover_state.nika`, `prover_loops.nika`). Until p4: its loop candidates are only halves of the entry facts (`bounding_loop_guesses`), and it proves a sum only below `solvable()` = 2^62 (`bounding_fits`).
* Certificate checker: `logic_search.nika` `lg_verify` and its callees, `logic_lia.nika`.
* Valid `Facts:` headings: `Where things live`, `Traps already met`, `Commands`, `Moving Rust into Nikaia`.

## Order of work (score = (S x 10 M + O) / C)

| # | Work | Issue | S | O | C | Score |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 1 | Package count, `proved.nika` | #1053.1 | | | | done (8aed3f6) |
| 2 | In-loop `assert`s through their loop | #1054 | | | | done (1ebd8fc) |
| 3 | Reasons by binding (p3) | #1053.3 | 2 | 2 | 6 | 3.7 |
| 4 | D10 checks through the prover's portfolio, plus the goal relative to a value of the type (p4) | #1054 | 3 | 3 | 10 | 3.3 |
| 5 | Counters: candidate `n - n@ <= k(i - i@)`, `for` turns bounded by the length | #1053.4, #432 | 3 | 3 | 12 | 2.8 |
| 6 | Certificate checker at zero: preconditions as `assert`s, guards that reject malformed certificates | #1053.5 | 3 | 1 | 6 | 5.2, after 4 and 5 |
| 7 | By p3's table: `if`/`match` values, parameters (an `assert` at the top), field facts #384, #388, #385, #386 | | | | | by the table |
| 8 | Facts about data: #797 (ready), #798 (decision); postconditions | | | | | |
| 9 | A `--bounds` expectation for `nikaia test` (a `.bounds` file beside a test), so proved checks are guarded by tests rather than only by the figure | not filed | | | | after 5 |

## Decisions in force

* ADR-306 D12: a length is below 2^60, so `xs.len() + 1` is proved.
* ADR-361 D2, D3, D10: claims are asked through loops, inside and after them. `assert`s are built (1ebd8fc). Checks are p4.
* p4 (architect, 2026-10-31, comment on #1054): D10 draws its candidates from `prover_loops`' portfolio, one portfolio for both walks. `bounding_fits` may state a one-way goal as `t <= y`, for a name `y` whose type's range lies inside the operation's type, at most 8 per goal.
* Open decisions this lane may need: #386 (products), #388 (equal lengths), #385 (lists of lists, `decision`), #798 (element rules), and postconditions (`ensures`, no issue yet).
* NK codes: `NK1320`-`NK1329`, `NK2720`-`NK2729`.

## Batch

p3 → p4 (`batch.txt`). Files: `bounds_reasons.nika`, `bounds_report.rs`, `scripts/proved.nika`; `bounds_walk.nika`, `prover_loops.nika`. None of them is in `docs/briefs/batch.txt` (s922, GR) or in the bug lane's batch (b9-b11). Next: item 5 (#432), written once p4's figure is in.
