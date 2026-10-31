# Prover lane: state

*Current state only, rewritten by the lane's architect each run. What happened is in `git log` and the issues.*

**Goal (owner, 2026-10-31):** 100 % of the indexes and arithmetic steps in `crates/nikaia-std/src/tools/*.nika` proved: claim (b) of `docs/formal-verification.md`. Epic: #1053 (`wip`). Then the gaps outsiders name (lengths, sorted lists, facts about data, postconditions, lemmas). Then, by the score: the prover as a product, interfaces for outside proposers (ADR-361 D6, `nikaia prove --propose`), current research.

**The headline counts at `prove-through-loops:on`** (architect, 2026-10-31): claim (b) is about what is proved, the strongest setting the toolchain offers; which proved checks a build drops is a separate setting (ADR-361 D11: `std` and the tools are lowered with it `off`). Both figures are recorded. **The figure must also count claims** before `assert`s go into the tools: a precondition (ADR-269 D5) moves a check to its callers, and an unproved one there is still a run-time check.

## Figure (2026-10-31, ecf435a, `lower-std --bounds` + `scripts/proved.nika`, 246 files, 0 refused)

| Setting | Indexes | Operations |
| :--- | :--- | :--- |
| `off` | 1,518 / 2,628 (57.8 %) | 729 / 1,612 (45.2 %) |
| `prove-through-loops:on` | 1,550 / 2,628 (59.0 %) | 736 / 1,612 (45.7 %) |

Unproved under `on` (1,954) by reason: 693 catch-all, 542 counter, 379 parameter, 127 list element (#797, #798), 96 another list's length (#388), 60 call result (#382), 37 list of lists (#385), 7 `if`/`match` value, 6 product (#386), 4 lower bound, 3 field (#384). Most per file: `fmt` 138, `text_tiers` 103, `logic_lia` 54, `emit_core_match` 52, `emit_function` 51. `tools/*.nika` holds no `assert` today.

## Lane facts (for the architect and briefs)

* Count: `target/fast/nikaia lower-std --bounds [--optimization prove-through-loops:on] > R` (about 45 s), then `target/fast/nikaia run scripts/proved.nika -- R`. Never per-file `check --bounds` on tools files: alone they are refused. The report's file header lines start in column 0; `awk '/^[^ ]/{f=$0} /REASON/{print f": "$1}' R` maps sites to files.
* Until p5 (#1069) lands, no test can show `--bounds`; after it, a program test carries `NAME.bounds` / `NAME.loops.bounds`, and every prover brief guards its gain with one.
* The bounds walk (`bounds_walk.nika`) is separate from the prover walk (`prover_walk.nika`, `prover_state.nika`, `prover_loops.nika`). p4 gave the walk the portfolio for `while` only; a `for`'s record (`loop_record_range`) is not built for the walk.
* Certificate checker: `logic_search.nika` `lg_verify` and its callees, `logic_lia.nika`.
* Valid `Facts:` headings: `Where things live`, `Traps already met`, `Commands`, `Moving Rust into Nikaia`.
* `nikaia.toml` has no optimization key: a setting is chosen on the command line only.

## Order of work (score = (S x 10 M + O) / C)

| # | Work | Issue | S | O | C | Score |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| 1-4 | Count, in-loop asserts, reasons (p3), D10 portfolio (p4) | #1053.1-3, #1054 | | | | done |
| 5 | `.bounds` expectations in `nikaia test` (p5) | #1069 | 3 | 3 | 6 | 5.5 |
| 6 | Counters: ADR-361 D13, `for` turn count, relative candidates (p6) | #1053.4 | 3 | 3 | 12 | 2.8, 542 sites |
| 7 | Claims in the figure: `lower-std --asserts`, `proved.nika` counts asserts and preconditions kept at calls | #1053.9 | 2 | 3 | 4 | 5.8, before 8 |
| 8 | `assert`s at the 379 parameter sites, file by file, files clear of the main lane | #1053.8 | 3 | 1 | 10 | 3.1 |
| 9 | Certificate checker at zero: preconditions as `assert`s, guards for malformed certificates | #1053.5 | 3 | 1 | 6 | 5.2, after 7 |
| 10 | The catch-all's 693: re-split by p3's classifier once 6 lands (what is left after the counters) | #1053.3 | 2 | 2 | 4 | 5.5, after 6 |
| 11 | A `for`'s record for the prover walk (`assert`s through a `for`), `loop_record_range` | #1053.4 | 2 | 2 | 8 | 2.8 |
| 12 | By the table: list elements #797, other list's length #388 (decision), call results (postconditions: decision), #384, #385, #386 | | | | | by the table |

## Decisions in force

* ADR-306 D12: a length is below 2^60, so `xs.len() + 1` is proved.
* ADR-361 D2, D3, D10: claims and checks are asked through loops (p2, p4). D13 (architect, 2026-10-31): a `for` counts its turns; a counter is bounded by `k` times them, at both settings.
* #1069 (architect, 2026-10-31): `NAME.bounds` (off) and `NAME.loops.bounds` (on) beside a program test, path relative, the build-dependent tail cut.
* p4: `bounding_fits` may state a one-way goal as `t <= y` for a name `y` whose type lies inside the operation's, at most 8 per goal.
* Open decisions this lane may need: #388 (equal lengths), #385 (lists of lists, `decision`), #386 (products), #798 (element rules), postconditions (`ensures`, no issue yet: write the decision paper when item 12 reaches call results).
* NK codes: `NK1320`-`NK1329`, `NK2720`-`NK2729`.

## CI and the tree (2026-10-31, ecf435a)

The lane's last suite was green (`suite.log`). `lowered/project_members.rs` is still uncommitted drift residue in the shared checkout (#1060, bug lane b15); briefs say never to stage it. Defects from the supervisor's script builders filed for the bug lane: #1066 (Duration in an f-string hole, NK1201 missing), #1067 (NK1102 on a `-> String` literal result), #1068 (bogus `ProbePlain` warning on `spawn`); #1062 was filed before. Not reproduced on ecf435a: `parts = f().split(" ").collect()` reassigned. `async e` refused is #984's planned build, no defect. Part III (`30-nikaia-tooling.md` :315) spells `time::Duration::new(30, 0)`, which the checker refuses (NK1149): a one-word spec fix for the supervisor.

## Batch

`batch.txt`: p5 (#1069, `project_tests.nika`, the program-test part of `project.rs` after line 2900, `bounds_report.rs`), then p6 (`bounds_walk.nika`, `bounds_reasons.nika`). P5c (main lane) cuts `project.rs` 2112-2586: p5 stays out of it. Next run: item 7 (claims in the figure), then 8 or 9 by p6's figure.
