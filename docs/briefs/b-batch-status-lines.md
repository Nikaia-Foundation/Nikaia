---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 8
---
# Brief: batch, status and cycle say what is wrong, not "no target/fast/nikaia" (#862)

Score: S=1 (tooling, no program meaning), O=1 M (every main-session restart: the 2026-10-26 restart lost turns on a misleading line; a `Result: done. ...` report cuts a finished batch), C=2 M (three scripts under 150 lines each, no compiler build beyond the binary), process lane, after the two bugs.

## Problem (issue #862, copied)
* `scripts/status.nika` `status_batch` prints `BATCH unknown (no target/fast/nikaia)` whenever `batch.nika` fails, for any reason; on 2026-10-26 the reason was a missing `DIR/batch.txt` (it lived in an old scratchpad).
* `scripts/batch.nika` `batch_run` throws on a missing `DIR/batch.txt` and on a brief named there that does not exist.
* `batch_cut` requires `Result:` to be exactly `done`; builders write `Result: done. A literal ...` (b853, b856, b857), which cuts a finished batch.
* `batch_cut` ignores `Gate:` (#852: a report without `Gate: green` is not passed on).
* `scripts/cycle.nika` prints `no target/fast/nikaia` in a fresh container and stops; it should name the command that makes one.

## Locations
`scripts/status.nika` `status_batch` (about line 46); `scripts/batch.nika` `batch_run` (about line 66), `batch_cut` (about line 52), header comment lines 1-16; `scripts/cycle.nika` `main` (about line 108, `println("no target/fast/nikaia")`). Test pattern: `tests/compiler/tests/batch_frontmatter.nika` (+ `.stdout`), which runs `batch.nika` on a temporary DIR.

## To build
1. `batch.nika`: no `DIR/batch.txt` prints `BATCH none (no DIR/batch.txt)` and exits 0; a listed brief that does not exist prints `CUT <brief>: brief missing`; `Result:` counts as done when its first word is `done` (`done.` too); a `Gate:` field present and not starting with `green` prints `CUT <brief>: Gate: <text>`. Update the header comment.
2. `status.nika`: `BATCH unknown (no target/fast/nikaia)` only when neither binary exists; when `batch.nika` fails, `BATCH error: <first line of its stderr, or stdout>` (at most 160 characters).
3. `cycle.nika`: with no binary, print `BUILD cargo build --profile fast -p nikaia (first build about 3 min, run it in the background)` and exit 1.
4. Tests: extend `batch_frontmatter.nika` (or a new `batch_lines.nika` beside it, same pattern) with: no batch.txt; a missing brief; `Result: done. text`; `Gate: red x`; `Gate: green`. Run it with `target/fast/nikaia test` on that file; gate; commit `scripts: batch, status and cycle name what is missing` with `Fixes #862`.

## Files you may change
`scripts/batch.nika`, `scripts/status.nika`, `scripts/cycle.nika`, `tests/compiler/tests/batch_frontmatter.*` or the new `batch_lines.*`. The main session runs these scripts between builders: keep their output lines' first words (`NEXT`, `CUT`, `OWNER`, `BATCH DONE`, `CI`, `BUMP`) unchanged.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-batch-status-lines.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|partial|blocked` (the bare word first), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
