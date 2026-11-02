---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# Brief: #1161 collect items of T? into a declared Vec[T]?

Blocks: the grammar_run move step 2 (type_written, #125)

Score: found during the self-hosting moves (#125), 2026-11-02; the issue holds the decision and the probe.

Facts: Where things live, Traps already met, Commands

## Problem
Approved extension of ADR-293 (read #1161): `collect` into a declared `C[T]?` from items of `T?` yields null at the first null item. Today NK1103. Probe: /tmp/claude-0/-home-user-Nikaia/b7e2d91b-1a15-5371-a36e-8c201aeab2d2/scratchpad/m2/p3/main.nika.

## To build
1. The ADR-293 sentence and its place in docs/specification/30-nikaia-tooling.md, the checker accepting it, the lowering to `collect::<Option<_>>()`. Test `tests/compiler/collect_into_optional_list.nika` + `.stdout`. Commit `Fixes #1161`.
Run the tests your change touches (`nikaia test` on the new test, the `--test it` filters of the files you change); `lower-std` and its fixpoint if a `tools/*.nika` changed.

## Rules
* Read `.claude/agents/builder.md` and `docs/builder-facts.md` first. Commit locally with `target/fast/nikaia run scripts/commit.nika -- REPORT` (if it refuses because another agent's files are staged, `git commit --only <your files>`); the driver pushes.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's.
* No cause after about 60 turns: stop, `Result: partial` with what you ruled out and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b40-1161-collect-into-optional.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...`).
