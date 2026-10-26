---
agent: builder
model: sonnet
max_turns: 50
budget_usd: 6
---
# Brief: regression tests for #706 (views of a `String` parameter pushed into a `mut Vec[String]` parameter)

Score: bug lane, S=3 cheap: the defect no longer reproduces (architect probe on 5aebafd, `target/fast/nikaia run`), but no commit names #706 and no test pins it. Test-only: no version raise, `Changelog: none`.

## Issue #706 (copied, short)
The checker accepted, rustc refused (`lifetime may not live long enough`): `fn words(s: String, mut v: Vec[String]) { for w in s.split(" ") { v.push(w) } }` lowered to `fn words(s: &str, v: &mut Vec<&str>)`. Same with `split_once` parts. Today both run: probe A prints `3`, probe B prints `6` then `1`; the `.stdout` holds the real output.

Probe A: `words` above, `main` does `let mut v: Vec[String] = []`, `words(f"a b {1}", v)`, `println(v.len())`.
Probe B: `fn parts(s: String, mut v: Vec[String]) { let (a, b) = s.split_once("=") ?? ("", ""); v.push(a); v.push(b) }`, called with `f"k={1}"` and with the literal `"x=y"`, plus a `for w in "c d".split(" ") { v.push(w) }` in `main`; print `v.len()` and `v[1]`.

## To build
1. Two tests in `tests/compiler/tests/`: `views_of_a_parameter_pushed_into_a_vec_parameter.nika` (probe A) and `split_once_parts_pushed_into_a_vec_parameter.nika` (probe B, literal and f-string callers), each with its `.stdout` (the real output). Copy the layout of a neighbour test there.
2. Run them: `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --both-settings --project tests/compiler` (all pass), and `cargo test --profile fast -p nikaia --test it -- nikaia_test::the_compiler_test_package`.
3. If a probe fails in rustc at either setting: stop, `Result: blocked`, the rustc error under `Found:`; do not fix the compiler in this brief.
4. Commit `tests: views of a parameter pushed into a Vec parameter run (Fixes #706)`.

## Files you may change
Only the four new files above. Known facts: read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b706-views-into-param-vec.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
