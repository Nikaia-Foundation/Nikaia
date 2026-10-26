---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 8
---
# Brief: a docs-only push must not promote an untested base (#861)

Score: S=3 (`main` moves onto a commit no green run built: a wrong state of what the outside sees), O=2 M (every report or brief push after a red run repeats it; it did so twice, 99837377 and c6a6bdd3), C=2 M (one shell script, no compiler build), CI lane, first of the batch.

## Issue #861 (copied)
Run 40334846995 on 8e86495d was red. The next push 99837377 touched only `docs/briefs/*.report.md`. `scripts/ci-changes.sh` diffs against `github.event.before` (= 8e86495d), so it decided docs-only and skipped build and test. `promote` only fails on `failure`/`cancelled`, so the skipped run counted as green and fast-forwarded `main` to 99837377, which contains the broken 8e86495d. It happened again: `origin/main` is c6a6bdd3 today, which contains befae38b/830d1970 (red in `nikaia_test::the_compiler_test_package_passes_at_both_settings`).

## Locations
* `scripts/ci-changes.sh` (the `BASE` case and `changed=$(git diff --name-only "$BASE" HEAD)`, about line 45).
* `.github/workflows/nikaia.yml`: job `changes`, step `Decide` (env `BASE: ${{ github.event.pull_request.base.sha || github.event.before }}`, about line 62); `checkout` there has `fetch-depth: 0`, so `origin/main` exists. Job `promote` about line 185.

## To build
1. `scripts/ci-changes.sh`: on a `push` to `refs/heads/develop`, the base is `git merge-base origin/main HEAD` (fall back to `$BASE` when `origin/main` is missing), not `event.before`: every change since the last tested commit counts, so a docs-only push on top of an untested code commit runs everything. Pull requests and `main` keep `$BASE`. Say so in the script's header comment (the `docs` bullet).
2. Probe, as the test (a CI script has no `nikaia test` form; this is the brief, not a Deviation): in a scratch clone (`git clone --shared`), make a code commit and a docs-only commit on top, with `origin/main` at the parent of both, and run `EVENT=push REF=refs/heads/develop BASE=<docs commit's parent> GITHUB_OUTPUT=/dev/stdout sh scripts/ci-changes.sh`: it must print `docs=false`. Then with `origin/main` at the code commit: `docs=true`. Put the two command lines and their outputs in the commit message. Delete the scratch clone.
3. Commit `CI: a docs-only push runs everything while main lacks a code change below it` with `Fixes #861`. Step 2 of the issue (a stale-`lowered` check in `bootstrap_compiler`) is not built: the architect dropped it (a cold `lower-std` is 181 G instructions per run); do not build it.

## Files you may change
`scripts/ci-changes.sh` only (the workflow only if step 1 cannot be done without it; then say why under Deviation). Do not touch `docs/briefs/b-fixforward-compiler-tests.md` or any file a running builder changes (`git status`).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b861-ci-docs-only-base.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|partial|blocked` (the bare word first), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
