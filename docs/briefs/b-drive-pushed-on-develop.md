---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 8
---
# Brief: drive.nika rewrites `Pushed: rejected` also when the builder's commit is already on origin/develop

Score: process; a false cut costs one architect run (about 1-3 USD) each time. The third false `Pushed: rejected` cut (b885, b-drive-rejected-push, now b998 at 2026-10-30 19:29).

## Problem

b998 reported `Pushed: rejected`, but its commit f2599c1c is on `origin/develop` (it is develop's tip: the push went through on a later try or by another session's push). `drive_commit` (`scripts/drive.nika:201`) looks for the report's `Commit:` subject only in `git log origin/develop..HEAD`, which is empty once the commit is on `origin/develop`, so the report is not rewritten and `batch.nika` cuts.

## Locations

* `drive_commit`, `scripts/drive.nika:192`; the log at :201; `drive_rewrite` :150 (unchanged).
* Tests that name the script: `grep -l drive.nika tests/ -r` (`drive_actions` and any other; builder-facts: a script change runs the tests naming it).

## To build

1. `git fetch origin develop` (quiet; a failed fetch is no stop) before the log, and take the log over the last 40 commits of both `HEAD` and `origin/develop` (`git log -40 --format=%h%x09%s HEAD origin/develop`), so a commit on either is found. Update the comment at :147 to say so.
2. Check by hand: a copy of `docs/briefs/b998-match-owned-text-nk2302.md.report.md` (untracked, in the main checkout; do not change or stage the original) in your scratch dir, run `drive_rewrite` over it with that log through the test or a `--dry-run`, and see `Pushed: f2599c1c (pushed by drive.nika)`. Run the tests naming `drive.nika`. Commit `drive: Pushed: rejected is rewritten when the commit is already on origin/develop`.

## Notes

* Read `docs/builder-facts.md`. Changelog: none (scripts/).
* Changing `drive.nika` makes the running driver stop with `RESTART` (exit 3); that is expected.
* Do not touch: the uncommitted files `git status` shows at start (`crates/rs2nika/convert/nikaia.*`, `docs/selfhosting-plan.md`, the k1/k3a1 drafts, other reports): another session's.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-drive-pushed-on-develop.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
