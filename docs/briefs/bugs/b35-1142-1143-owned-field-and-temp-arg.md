---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# Brief: #1142 a field moved into `let mut` stays a view; #1143 a temporary handed to a parameter that is returned

Blocks: s2-bounds-report (the bounds_report move, #125)

Score: two defects, specified behaviour, rustc rejects the lowered code and its error reaches the user. Both found by the probes for s2 (supervisor, 2026-11-01).

Facts: Where things live, Traps already met, Commands

## Problem
The probes are verbatim in #1142 and #1143 (run them with `target/fast/nikaia run`):
* #1142: `let mut sites = walk.sites` (last use of `walk`), then `sites.sort_by_key(fn(s) { s.line })` or `sites.push(..)`: a warning "does not need to be mutable" and rustc's "Cannot borrow `*sites` as mutable, as it is behind a `&` reference". Expected `1`. `walk.sites.sort_by_key(..)` in place works.
* #1143: `pad_left(site.line.to_string(), 4)` inside `for site in sites`, `pad_left(text: String, width: i64) -> String` returning `text` on one path: rustc's "Temporary value dropped while borrowed". Expected `[   7]`. `pad_left(f"{site.line}", 4)` works.

## To build
1. #1142: a field read bound to a mutating `let mut` is owned (moved at the owner's last use, else what the existing rules say for a non-copy field; read the borrow/ownership ADRs the lowering cites and name the one you follow). Test `tests/compiler/field_moved_into_mut.nika` + `.stdout` (sort and push). Commit `Fixes #1142`.
2. #1143: a temporary argument to a parameter that flows into the result lives long enough (owned, or bound by the lowering). Test `tests/compiler/temp_arg_returned.nika` + `.stdout`. Commit `Fixes #1143`.
Run the tests your change touches (`nikaia test` on the new tests, the `--test it` filters of the files you change); `lower-std` and its fixpoint if a `tools/*.nika` changed.

## Rules
* Read `.claude/agents/builder.md` and `docs/builder-facts.md` first. Commit locally with `target/fast/nikaia run scripts/commit.nika -- REPORT`; the driver pushes.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's.
* No cause after about 60 turns: stop, `Result: partial` with what you ruled out and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b35-1142-1143-owned-field-and-temp-arg.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...`).
