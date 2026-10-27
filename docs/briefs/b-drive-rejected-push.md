---
agent: builder
model: sonnet
max_turns: 70
budget_usd: 10
---
# Brief: the driver pushes a builder's rejected commit and records it, so the batch is not cut (process)

Score: process fix, before any lane (`docs/architect.md`, *Self-optimisation*): two architect runs in a row (2026-10-27, cuts at e12-2 and b883) were spent on a false cut. The builder reported `Pushed: rejected` as `common.part` tells it to; `drive.nika`'s `drive_commit` then pushed the builder's commit with its own (`drive_push` rebases once), so the commit is on `origin/develop` (b883: f95b026), but `batch.nika`'s `batch_cut` still cuts on `Pushed: rejected`.

## To build
1. In `scripts/drive.nika`, `drive_commit` (line ~126), before `git add`: read the brief's report (`drive_paths` already finds it; the `Pushed:`, `Result:`, `Commit:` fields with `drive_field`). If `Result:` is `done` and `Pushed:` is `rejected` or `none`, look for a local unpushed commit whose subject equals the report's `Commit:` line exactly (`git log --format=%h%x09%s origin/develop..HEAD`). If found, rewrite the report's `Pushed:` line to `Pushed: <short sha> (pushed by drive.nika)`; the drive commit and push that follow carry both. If not found, leave the report alone (the cut stands, rightly). `--dry-run` prints `would run: rewrite Pushed: in <report>` and changes nothing.
2. Update the header comment of `drive.nika` (one line) and the `Pushed:` line of the report form in `docs/architect.md`, *Briefs and reports* (one clause: *drive.nika rewrites `rejected` when it pushed the commit itself*). `batch.nika` stays as it is.
3. Test in Nikaia: extend `tests/compiler/tests/drive_actions.nika` (or a new `tests/compiler/tests/drive_rejected_push.nika` with `.stdout`) for the pure part: put the rewrite in a function that takes the report text and the `git log` lines and returns the new text (or the same), and test the three cases (subject found -> rewritten; not found -> unchanged; `Pushed: abc123` -> unchanged). No git in the test; CI has no `target/fast` (`docs/builder-facts.md`, *CI has no target/fast*).

## Locations
* `scripts/drive.nika`: `drive_field` 50, `drive_paths` 95, `drive_push` 115, `drive_commit` 126.
* `scripts/batch.nika`: `batch_cut` 58 (read only; do not change).
* Tests: `tests/compiler/tests/drive_actions.nika`, `batch_lines.nika`. A script change runs the tests that name it (`docs/builder-facts.md`).

Done when: the tests naming `drive` and `batch` pass (`nikaia test tests/compiler -- drive batch` or the filter builder-facts names), clippy/fmt/pre-push-check clean. Commit: `drive.nika: a builder's rejected commit pushed by the driver is recorded as pushed`. No issue; no version raise (scripts). `Changelog: none`.

Stop and report if: the report is already committed by the builder itself before `drive_commit` runs (then say where the rewrite belongs; do not redesign).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-drive-rejected-push.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
