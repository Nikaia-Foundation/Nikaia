---
agent: builder
model: sonnet
max_turns: 30
budget_usd: 5
---
# Brief: fix-forward, CI red since c129f145 (#1029): one stale assertion in a Rust test

Score: red `develop` is fixed forward first (CLAUDE.md); `main` stands still until it is green.

## Facts (architect, at dd9d5470)
* CI red on every full run since the push of b0ba2293 + c129f145 (#1029, first red run 4fa242a6, last green full run 818f6cdf). Jobs `bootstrap_compiler` and `arm64`, step Test: `2886 passed; 1 failed`, the one being `selfhosting_defects::a_run_and_a_kept_lambda_in_a_statement_differ`.
* Cause, traced: c129f145 writes a kept lambda's boxed future through a typed let (accepted as no deviation in b1029's report: rustc carries no type through the `as` cast). The lowering is now
  `nikaia_std::func::Kept(std::sync::Arc::new(move |x| { let __nikaia_future: nikaia_std::func::BoxedIn<'_, i64> = Box::pin(async move {`
  while the test asserts the old text `Kept(std::sync::Arc::new(move |x| Box::pin`. The program still prints `11\n6` (probed with `target/fast/nikaia lower`/`run`). No other test asserts `|x| Box::pin` (grep over `crates/**/tests/**`).

## To build
1. In `crates/nikaia/tests/it/selfhosting_defects.rs:202`, change the asserted text so it holds for the current lowering and still says what the test is about (the second lambda is a shared `Kept` closure, the first a run-only `&async`): `rust.contains("Kept(std::sync::Arc::new(move |x|")`. Change nothing else (CLAUDE.md: an existing Rust test is changed only to keep it passing).
2. Run only that test: `cargo test -p nikaia --test it selfhosting_defects::a_run_and_a_kept_lambda_in_a_statement_differ`. Then clippy, fmt, `scripts/pre-push-check.sh`, commit, push.

Commit: `test: a kept lambda's lowering is asserted up to its closure head (CI red since c129f145)`. No `Fixes`. `Changelog: none`.

## Stop and report if
* The test fails for another reason than the asserted text (lowering, rustc, output): `Result: blocked`, `Found:` the message.
* Any other file would have to change.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bred2-kept-lambda-test.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
