---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: #938 (last step), a list of number lists has one open number for its innermost elements (ADR-285 D34, D43)

Score: compiler bug, a false acceptance (rows b, c: the checker passes, rustc refuses) and a false refusal (rows a, f: NK1245); bugs-first default of `docs/priorities.md`. Kind 3 (defect), not a decision: D34 makes a list's literal elements one open number, and D43 extends it to a map's values to keep "one rule for every place a number literal is stored"; a list of lists is that rule one level further.

## Issue #938 (state after a6ec14b3, b938's partial)
Step 1 is built for one level (`for n in [1, 2]`, a6ec14b3, test `tests/compiler/tests/a_loop_over_an_untyped_number_list_copies.nika`). Left: a list whose items are lists of numbers gets no open number, so its `let` gets no written type, rustc falls back to `i32`, and a use that asks for `i64` fails in rustc (b938's report, traced with a replica of `num::value`).

Probe matrix (architect, at dd9d5470; every program starts `fn twice(n: i64) -> i64 { return n * 2 }` where it calls `twice`, and `fn main() {` + `let grid = [[1, 2], [3]]`):
| Row | Body after the `let` | Today | Expected |
| :--- | :--- | :--- | :--- |
| a | `println(f"{grid[0][1] + grid[1][0]}")` | NK1245 "can't tell what type `grid[0][1]` is" | `5` |
| b | `for n in grid[0] { println(f"{twice(n)}") }` | rustc Mismatched types | `2`, `4` |
| c | `for r in grid { for n in r { println(f"{twice(n)}") } }` | rustc Mismatched types | `2`, `4`, `6` |
| d | `let mut s = 0` / `for r in grid { for n in r { s = s + n } }` / `println(f"{s}")` | `6` | `6` (guard) |
| e | `println(f"{total(grid)}")` with `fn total(xs: Vec[Vec[u8]]) -> u8 { return xs[0][0] + xs[1][0] }` | `4` | `4` (guard: the parameter asks `u8`) |
| f | `let cube = [[[1, 2]], [[3]]]` / `println(f"{cube[1][0][0] + 1}")` | NK1245 | `4` |
| g | `let row = grid[0]` / `let x: u8 = row[0]` / `println(f"{x}")` | `1` | `1` (guard: `row`'s elements are the same number) |

Each of a, b, c with `let grid: Vec[Vec[i64]] = ...` prints the expected output today, so the written type is the whole cause. Traced to the missing open number: b, c (b938). a, f: NK1245 from `open_numbers_in` finding no open number for `grid[0][1]` (`open_element_in` takes only a `Variable` base); unverified beyond that. d, e, g run today only because no type is written; once one is, they hold only if the asks reach the number: they are the regression rows.

## Locations
* The `let`: `crates/nikaia-std/src/tools/check_core_stmt.nika:726-770` (`Expr::ListLit` whose items are all `number_shaped` opens the number, `element_at`; stored at :883 in `state.flow.open_elements`).
* The `for`: same file :1556-1595 (a `Variable` walk reads `open_elements`, binds `frame[0].open_number`, and since a6ec14b3 records `copied_loop_bindings`). `for r in grid` must give `r` its elements' number; `for n in grid[0]` must find it through the index.
* The open number: `CheckOpenNumber` in `crates/nikaia-std/src/tools/check_flow.nika:26` (`element: bool`; `open_elements` :246); 5 struct literals of it (`grep -n "CheckOpenNumber {"`), the `let`'s is `check_core_stmt_open_number` (check_core_stmt.nika:335).
* The written type: `crates/nikaia-std/src/tools/check_numbers3.nika:207-220` (`written = f"Vec<{ty}>"` for an element number, into `facts.number_lets`; the emitter writes it, no emitter change expected). Message at :104 and :136 ("the elements of `grid`").
* Reads and asks: `crates/nikaia-std/src/tools/check_numbers4.nika` `open_element_of`/`open_element_in`/`open_element_named` :146-170, `open_numbers_in` :205, `elements_asked` :479 (row e: `Vec[Vec[u8]]` must ask `u8` of the inner number; `number_list_element` :401), `open_items_of` :744.
* Other readers of `open_elements`: `check_core_call_on.nika` :375, :432, :558, :600-634 (method calls on an open list).

## Shape (a suggestion; any shape that keeps one number per `let` is fine)
`CheckOpenNumber` gains the count of list levels around its numbers (0 for a name's own number, 1 for `[1, 2]`, 2 for `[[1, 2], [3]]`), and `number_lets` writes that many `Vec<..>`. A local whose items are lists of the number (`grid`) is recorded with the number and its level, so `grid[i]` and a `for` over `grid` hand the next level down (`row`, `r` get `open_elements`), and `grid[i][j]` is the number. A list literal of list literals opens one number only when every innermost item is `number_shaped` and every level is a list literal; anything else stays as today.

## To build
1. The open number for nested list literals at the `let`, its written type, and the reads and asks above, so rows a-g give the expected column. One commit, with the new test `tests/compiler/tests/a_list_of_number_lists_has_one_open_number.nika` + `.stdout` holding rows a, b, c, e, f, g in one program (d is covered by them; keep its `s` loop in if it costs nothing). Check the test fails on the unfixed compiler.
2. Run: `nikaia test --both-settings --project tests/compiler > out.txt 2>&1` once (no name filter exists; grep the file for the new test and `a_loop_over_an_untyped_number_list_copies`), the guard set of `.claude/agents/builder.md`, `lower-std` to fixpoint as `docs/builder-facts.md` says.

Commit: `check: a list of number lists has one open number for its innermost elements (Fixes #938)`.

## Stop and report if
* Stop about 15 turns before `max_turns`: commit what is finished and report `partial`.
* A file outside `check_core_stmt.nika`, `check_flow.nika`, `check_numbers3.nika`, `check_numbers4.nika`, `check_core_call_on.nika`, their `lowered/*.rs` and the new test would have to change: report, do not change it.
* A row needs a refusal or an NK code: `Result: blocked`, `Needs:`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b938b-nested-number-lists.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
