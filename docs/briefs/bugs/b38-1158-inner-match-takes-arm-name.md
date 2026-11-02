---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# Brief: #1158 an inner `match` on an arm-bound name that takes a non-copy part lowers to `ref x` / `match *x`

Blocks: the comptime_run move (M1, #125): `ComptimeRan` cannot carry `CheckWall`.

Score: a defect, specified behaviour (taking a part of a value bound by value moves it), rustc rejects the lowered code (E0507). Found by the M1 builder (2026-11-02).

Facts: Where things live, Traps already met, Commands

## Problem
The probe is verbatim in #1158 (run it with `target/fast/nikaia run`): `match r { Ran::Hit(wall) => match wall { Wall::Refused { detail } => detail, ... } }` lowers to `Ran::Hit(ref wall) => match *wall { ... }`, which rustc refuses with E0507. `let w = wall; match w` lowers correctly. Suspect: `check_compare.nika` `parts_lent` / `check_core_flow_lent_parts`: the inner match taking a part does not count as giving `wall` away.

## To build
1. An inner match that takes a non-copy part of an arm-bound name gives that name away, so the outer arm binds by value. Test `tests/compiler/inner_match_takes_arm_bound_part.nika` + `.stdout`. Commit `Fixes #1158`.
Run the tests your change touches (`nikaia test` on the new test, the `--test it` filters of the files you change); `lower-std` and its fixpoint if a `tools/*.nika` changed.

## Rules
* Read `.claude/agents/builder.md` and `docs/builder-facts.md` first. Commit locally with `target/fast/nikaia run scripts/commit.nika -- REPORT`; the driver pushes.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's.
* No cause after about 60 turns: stop, `Result: partial` with what you ruled out and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b38-1158-inner-match-takes-arm-name.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...`).
