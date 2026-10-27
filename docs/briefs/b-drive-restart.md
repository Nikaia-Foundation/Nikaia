---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 8
---
# Brief: `drive.nika` stops when its own source changed, so a fix to it is not dead until a restart

Score: process (`docs/priorities.md`: false cuts cost one architect run each). Three of the last five cuts were false `Pushed: rejected` cuts; the third (b885, d942502 on develop) came after 5a91e92 built the rewrite, because the running driver was started at 10:52, before 5a91e92 (12:07): `nikaia run` compiles `drive.nika` once at start, so a commit to it does nothing until someone restarts it. `cycle.nika`, `batch.nika`, `agent.nika` are spawned per call and already pick up changes.

## Facts
Read `docs/builder-facts.md`. The driver is running in this checkout while you work: do not stop it, do not run `drive.nika` without `--dry-run` / `--lines`.

## Locations
`scripts/drive.nika`: header comment :1-25, `drive_escalate` :39, `main` :244, loop top :285 (`while runs <= max`). Tests: `tests/compiler/tests/drive_actions.nika`, `drive_rejected_push.nika` (pattern for a test that runs the script with `process::run`).

## To build
1. At start (after argument parsing, before the loop) `main` reads `scripts/drive.nika`'s text (path relative to the repo root, the cwd). At the top of each loop round, before `drive_cycle`, it reads it again; if it differs, it prints the one final line `RESTART drive: scripts/drive.nika changed` and exits 3. Not in `--dry-run`/`--lines` mode. Header comment names the line and exit code.
2. `docs/architect.md`, *The main session's cycle* step 2: on `RESTART` (exit 3) the main session starts `drive.nika` again at once, no owner, no architect.
3. A test in `tests/compiler/tests/` (Nikaia, `.stdout` beside it): a hidden flag `--changed OLDFILE` (like `--rewrite`) that prints `RESTART ...` when OLDFILE's text differs from `scripts/drive.nika` and `same` otherwise; two cases. Run it and `drive_actions`, `drive_rejected_push`.
Commit message ends with no `Fixes` (no issue).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-drive-restart.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
