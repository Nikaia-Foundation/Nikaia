---
agent: builder
model: sonnet
max_turns: 40
budget_usd: 5
---
# Fix-forward: `drive_actions` red since 7970db8 (drive.nika tidy)

Score: red `develop` is fixed forward before any other work (`CLAUDE.md`); `main` stands still until it is green.

## Problem
7970db8 added `drive_tidy` to `scripts/drive.nika` (called after `drive_commit` in the builder branch and the architect branch of `main`). In `--dry-run` it prints `would run: git checkout -- crates/rs2nika/convert/nikaia.lock` (and an `rm -f ...` line only when stray `*.step<N>.report.md` exist). `tests/compiler/tests/drive_actions.stdout` was not updated, so `drive_actions` fails (reported by b867, run 6cd8fea).

## Locations
* `scripts/drive.nika`: `fn drive_tidy` (about line 161), its two calls in `fn main` (after `drive_commit`).
* `tests/compiler/tests/drive_actions.nika`, `tests/compiler/tests/drive_actions.stdout`.

## To build
1. The tidy line is wanted behaviour: update `drive_actions.stdout` so the `next`, `batch done`, `cut` and `red` cases expect the new `would run: git checkout -- crates/rs2nika/convert/nikaia.lock` line after `git push origin develop`. Take the exact text from a run of the test, then check by eye that the only difference to the old file is that line (four times). If the test shows anything else (an `rm -f` line from the scratch dir, a different order), stop and report `blocked` with the diff.
2. Run: `target/fast/nikaia test --both-settings --project tests/compiler` filtered to `drive_actions` (or the whole `tests/compiler` package if no filter exists; it is about 120 tests). Commit message: `tests: drive_actions expects drive.nika's tidy line (fix-forward 7970db8)`.

Read `docs/builder-facts.md` first.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-fixforward-drive-tidy.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built; how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
