---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# Brief: #1160 finish: test-only helpers under `#[cfg(test)]`, built but not committed

Blocks: every Rust-test move into Nikaia (#125): clippy -D warnings goes red (threads, ty, touch)

Score: the first run (b39) built both steps and ran out of turns before the gate; nothing is committed. Its debrief: `docs/briefs/bugs/b39-1160-test-only-helpers-cfg-test.debrief.md`.

Facts:
* The work is uncommitted in the shared tree, all of it b39's (no other agent has changes in these files):
  `crates/nikaia-std/src/tools/{check_core_stmt,emit_function,manifest,modules,threads,touch,ty}.nika`, their `lowered/*.rs`, `crates/nikaia-std/src/tools/lowered.rs`, `crates/nikaia/src/sysroot.rs`,
  `tests/compiler/tests/a_let_over_a_part_of_a_drained_element_moves_it.nika/.stdout`,
  `tests/compiler/tests/test_only_helper_lowers_under_cfg_test.nika/.stdout`.
* Two changes are mixed in it: (a) a compiler defect: on a `drain()` loop element, `let mut x = item.field` lowered to `&item.field` (fix in `check_core_stmt.nika`, test `a_let_over_a_part_of_a_drained_element_moves_it`); (b) #1160 itself (`modules_only_tests_reach` in `modules.nika`, the `#[cfg(test)]` mark in `emit_function.nika`, `sysroot.rs` keying the cache on it, hand fixes removed in threads/ty/touch/manifest).
* Program tests live in `tests/compiler/tests/`. `nikaia run` refuses a file inside the project: copy it to a scratch folder to run one.
* The debrief names a second defect it wrote the test around: a function returning a local view of a parameter lowered with a `&'static str` result. Do not fix it in this brief; file it as an issue with a probe and a *To build* list (label `ready`) and name it in the report.

## Problem
Finish and commit b39's work; do not rebuild it from scratch. Read #1160 and the debrief.

## To build
1. Defect (a): review the `check_core_stmt.nika` change, run its test (both settings), commit it alone with its lowered file and test.
2. #1160 (b): review the rest; `lower-std` and its fixpoint; `cargo clippy --all-targets -- -D warnings` clean (lowered threads.rs, ty.rs, touch.rs, manifest.rs); `cargo fmt`; the `sysroot::` and `nikaia-std` unit tests; check both `.stdout` files with `nikaia test --both-settings` (the two programs); `pre-push`. Commit `Fixes #1160`.
Run the tests your change touches; never the full suite.

## Rules
* Read `.claude/agents/builder.md` and `docs/builder-facts.md` first. Commit locally with `target/fast/nikaia run scripts/commit.nika -- REPORT` (if it refuses because another agent's files are staged, `git commit --only <your files>`); the driver pushes.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's.
* No cause after about 60 turns: stop, `Result: partial` with what you ruled out and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b39b-1160-finish.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...`).
