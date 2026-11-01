---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# Brief: #1144 `rest.to_string()` of a `strip_prefix` view, put into a returned set, gets no owned type

Blocks: s3-describe (the describe move, #125)

Score: a defect, specified behaviour (`to_string` yields an owned `String`), rustc rejects the lowered code and its error reaches the user. Found by the probes for s3 (supervisor, 2026-11-01).

Facts: Where things live, Traps already met, Commands

## Problem
The probe is verbatim in #1144 (run it with `target/fast/nikaia run`): `let rest = line.strip_prefix("x ") ?? continue`, then `out.insert(rest.to_string())` into a `BTreeSet[String]` that the function returns. rustc reports "Missing lifetime specifier": the lowering types the set from the view, not from the owned result of `to_string`. `out.insert(f"{rest}")` works.

## To build
1. The type of `v.to_string()` is the owned `String` whatever `v` borrows from, in inference of the container's element type and in the lowered signature. Test `tests/compiler/to_string_of_view_owned.nika` + `.stdout`. Commit `Fixes #1144`.
Run the tests your change touches (`nikaia test` on the new test, the `--test it` filters of the files you change); `lower-std` and its fixpoint if a `tools/*.nika` changed.

## Rules
* Read `.claude/agents/builder.md` and `docs/builder-facts.md` first. Commit locally with `target/fast/nikaia run scripts/commit.nika -- REPORT`; the driver pushes.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's.
* No cause after about 60 turns: stop, `Result: partial` with what you ruled out and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b36-1144-to-string-owned.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...`).
