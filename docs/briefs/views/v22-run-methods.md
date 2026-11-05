---
agent: builder
model: opus
max_turns: 80
budget_usd: 12
effort: medium
---

# v22: `swap`, `sort`, `sort_by_key`, `reverse`, `fill` on a changeable run and on a list (#1209, ADR-views-3 D11)

Score: ADR-views-3 D11, the bullet "(#1209, added November 5, 2026)", and Part I 2.x after "needs no word.": a changeable run (`mut xs: Array[T]`, a piece `v[a..<b]` of a `mut` place) and a list of a `mut` place change in place with `xs[i] = v`, `swap(i, j)`, `sort()`, `sort_by_key(f)`, `reverse()` and `fill(v)`; they lower to Rust's `[T]` methods. While a part is handed out as `mut`, the list is not read or changed otherwise (`NK2305`). Rust is allowed in the std runtime and the checker for the views-3 lane (owner 2026-11-03, #1214; list it in `Body:`).

Facts: Checker and views, Lowering and lower-std, Tests and the gate

Today: `v[2..<4].sort()` gives `NK1171` (*`Array` has no method called `sort`*), `v.fill(4)` gives `NK1171` (*`Vec` has no method called `fill`*).

**Owns:** the method table entries for `Array` and `Vec`, their lowering, the new tests.

**Not yours:** part arguments to a parameter (v21), `split_at_mut` (not decided: stop and report if you need it).

## To build
1. The five methods on `Vec[T]` of a `mut` place and on `mut xs: Array[T]` (inside a function taking it): a test `tests/compiler/tests/a_changeable_run_sorts_swaps_and_fills_in_place.nika` with its `.stdout`, covering each method once; `sort` needs an ordered element (a `.stderr` case for an element without order); a call on a list that is not `mut` is refused (`.stderr`). Commit `Part of #1209`.
2. The same on a piece: `v[2..<4].sort()`, `v[0..<2].fill(0)`, `v[1..<3].reverse()` on a `mut` list, in the same or a second test. Commit `Part of #1209`.

`Rs2nika: none`.

## Done when
The tests pass at both settings; existing method tests pass.

## Stop and report if
`sort_by_key` needs a closure form Nikaia does not have, or a method name clashes with an existing one of another meaning.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v22-run-methods.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
