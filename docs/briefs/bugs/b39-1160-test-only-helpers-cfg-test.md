---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# Brief: #1160 tools/*.nika helpers used only by test blocks lower as dead code

Blocks: every Rust-test move into Nikaia (#125): clippy -D warnings goes red (threads, ty, touch)

Score: found during the self-hosting moves (#125), 2026-11-02; the issue holds the decision and the probe.

Facts: Where things live, Traps already met, Commands

## Problem
A function of a tools/*.nika that only `test` blocks call lowers as a plain `fn`; `cargo clippy --all-targets -- -D warnings` reports it unused. 03fc5eca fixed threads.nika by hand; ty.nika and touch.nika have the same. Read #1160.

## To build
1. A function reached only from `test` blocks lowers under `#[cfg(test)]`; remove the hand fixes that become unneeded and check clippy is clean on lowered/threads.rs, ty.rs, touch.rs. Test `tests/compiler/test_only_helper_lowers_under_cfg_test.nika` + `.stdout`. Commit `Fixes #1160`.
Run the tests your change touches (`nikaia test` on the new test, the `--test it` filters of the files you change); `lower-std` and its fixpoint if a `tools/*.nika` changed.

## Rules
* Read `.claude/agents/builder.md` and `docs/builder-facts.md` first. Commit locally with `target/fast/nikaia run scripts/commit.nika -- REPORT` (if it refuses because another agent's files are staged, `git commit --only <your files>`); the driver pushes.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's.
* No cause after about 60 turns: stop, `Result: partial` with what you ruled out and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b39-1160-test-only-helpers-cfg-test.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...`).
