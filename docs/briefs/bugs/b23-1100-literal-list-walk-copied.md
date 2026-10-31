---
agent: builder
model: opus
max_turns: 70
budget_usd: 14
---
# Brief: #1100, a walk over a list literal of open numbers is not a copied walk

Blocks: none (found by s1082, `docs/briefs/s1082-or-default-cloned.md.report.md`)

Score: compiler bug, a plainly meant form that reaches rustc (a checker that lets through what rustc refuses). One fact set in one place; cause suspected, not traced to the line.

Facts: Where things live, Traps already met, Commands

## Issue #1100 (probe checked by the architect on `target/fast/nikaia` at 0d8deb1f)
```nika
fn main() {
    let c = [1, 2, 3].iter().filter(fn(x) { x > 1 }).count()
    print(c)
}
```
Expected `2`. Got rustc `expected &&_, found integer` at `x > 1`; the lowering is `vec![1, 2, 3].iter().filter(|x| { x > 1 })`. Over a local (`let ys = [1, 2, 3]`, then `ys.iter()...`) it lowers to `ys.iter().copied().filter(|&x| ...)` and prints `2`; a typed `let xs: Vec[i64]` works too. `let s: i64 = [1, 2, 3].iter().filter(fn(x) { x > 1 }).sum()` fails the same way.

Cause (suspected, medium): `crates/nikaia-std/src/tools/check_core_method.nika`, the walk block after `check_core_method_resolved` (about :618-635): for a `Ty::Seq` whose item is an open number, `copied_walks` is set only when `walk_open != null && state.flow.walk_view`. For a list literal as the receiver of `iter()` neither is set (no variable carries the open number), so neither `.copied()` nor `|&x|` is written. Find where `walk_open` and `walk_view` are set for a variable receiver (`nikaia run scripts/where.nika -- walk_open walk_view`) and give the list literal the same. The emitter reads `copied_walks` in `emit_core_calls.nika` (:1514); it should need no change. ADR-231 D1 and #594 govern.

## To build
1. Reproduce the probe and the `sum` row on the unfixed binary (`Tests:`).
2. A list literal of open numbers as the receiver of `iter()` makes a copied walk as a local holding it does; its numbers default as the local's do.
3. `lower-std` and its fixpoint.
4. Test `tests/compiler/tests/literal_list_walk_copied.nika` + `.stdout`: the probe, the `sum` row, and `[1, 2].iter().map(fn(x) { x * 2 }).collect()` printed; failing on the unfixed binary.
Commit `check: a walk over a list literal of open numbers copies its items (Fixes #1100)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings`. Also: `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`: commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `check_core_method.nika`, `check_core_flow.nika`, `check_core_args.nika`, `check_facts.nika`, their lowered twins and the new test. Or you hit an error in another agent's file. #1098 (named fn and `filter_map` over `iter()` of copy elements) is a separate issue: do not fix it here.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b23-1100-literal-list-walk-copied.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (`green` or `red <check>`: your cheap checks, never `none`, which cuts the batch, j4 2026-10-31), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
