---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: a plain value in a tuple literal at a `T?` position is wrapped (Fixes #887)

Score: bugs lane (`docs/priorities.md`, bugs-first default; three `ready` bugs #886-#888 found by the #879 builder, this the smallest).

## Issue #887 (copied)
```nika
fn pair(p: (String?, i64)) -> i64 { p.1 }
fn main() { println(pair(("xy", 1))) }
```
The checker passes this program. rustc then fails with mismatched types: the `"xy"` at the `String?` position is not lowered as `Some(..)`. A plain value passed to a `T?` parameter is wrapped already. Inside a tuple literal it is not.

**To build**
1. Emit: when a tuple literal is checked against an expected tuple type, each element at a `T?` position whose own type is `T` is wrapped, as a `T?` argument is. The same applies to `let t: (String?, i64) = ("xy", 1)` and to `return`.
2. A test in `tests/compiler/tests/` covering the argument, the `let` and the `return` forms.

## Facts
Read `docs/builder-facts.md`. The wrap is a checker fact the emit reads: `state.facts.wrapped` / `nullable_args` with `FactWrap::Constructor` (`tools/check_facts.nika:14`), written `Some(..)` by `tools/emit_exprs.nika:96`. The list analogue exists: `check_overflow_list_into_nullables` (`tools/check_overflow.nika:254`, called at :232 from `check_widened_slot` :211 and at `tools/check_arguments.nika:363`) marks `[..]` items going into `Vec[T?]`. Build the tuple case beside it, the same way (a nested tuple / list inside a tuple recurses). Probe also a tuple element that is a non-literal `String` local and an `i64` into `i64?`; and that `(null, 1)` and an element already `T?` stay unwrapped. Tools files changed are re-lowered (`docs/builder-facts.md`, lowering).

## Done when
Probe fails before, passes after; new test passes at both settings; `nikaia test --project tests/compiler` filters for `nullable` and `tuple` green. Last commit says `Fixes #887`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b887-tuple-nullable-wrap.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
