---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# Brief: #1152 a struct with a `ref` field cannot be built from a local (NK1106) or a `ref` parameter (NK2302)

Blocks: t1-tools-test-blocks step 2 and t2-tests-ty-touch-manifest (the Rust unit tests move into Nikaia, #1147)

Score: a defect / obvious extension approved by the supervisor (2026-11-02): a `ref` field takes an owned local by lending it, as a `ref` parameter does. Found by T1.

Facts: Where things live, Traps already met, Commands

## Problem
The probe is verbatim in #1152 (run it with `target/fast/nikaia check`): `W { own: l, k: 1 }` with `own: ref L` and a local `l: L` is NK1106; the same literal from a `ref L` parameter inside a function it never leaves is NK2302 (`go keeps l ... doesn't say whose data`).

## To build
1. A struct literal lends an owned local to a `ref T` field (as a call argument is lent), and a struct value built from a `ref` parameter that does not leave the function is no keep. The existing keeps rules still refuse such a struct returned past the local. Tests `tests/compiler/ref_field_from_local.nika` + `.stdout`, and one refusal `ref_field_outlives_local.nika` + `.stderr`. Commit `Fixes #1152`.
Run the tests your change touches (`nikaia test` on the new tests, the `--test it` filters of the files you change); `lower-std` and its fixpoint if a `tools/*.nika` changed.

## Rules
* Read `.claude/agents/builder.md` and `docs/builder-facts.md` first. Commit locally with `target/fast/nikaia run scripts/commit.nika -- REPORT`; the driver pushes.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's.
* No cause after about 60 turns: stop, `Result: partial` with what you ruled out and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b37-1152-ref-field-from-local.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...`).
