---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 8
---
# Brief: `scripts/commit.nika` finds the brief of a `<brief>.md.report.md` and takes a `Body:` field

Score: process (`docs/architect.md`, *Self-optimisation*): two builders in a row (b885, b887) committed by hand or lost the Attribution block because of these two gaps.

## Facts
* `scripts/commit.nika:121-124`: `brief_path = path.strip_suffix(".report.md")`, then it reads `f"{brief_path}.md"`. Every brief names its report `<brief>.md.report.md`, so `brief_path` already ends in `.md` and `<brief>.md.md` is looked for: the brief's `Attribution:` block is lost (b887 worked around it with `Model:`).
* b885 needed a commit body (a note the brief asked for) and committed by hand: the report form has no field for it.
* `scripts/batch.nika:49-52` and `scripts/agent.nika:202-203` already accept both report names; do the same here.

## To build
1. `commit.nika`: the brief is `brief_path` itself when it ends in `.md` and exists, else `f"{brief_path}.md"` (old name). Update the header comment.
2. `commit.nika`: an optional report field `Body:` (one line) goes into the message as a paragraph after the subject, before `Changelog:`. Add it to the report form in `docs/architect.md`, *Briefs and reports* (one line: `Body: <one paragraph for the commit message>   (optional)`), and to `docs/briefs/common.part`'s field list.
3. A test in `tests/compiler/tests/` (Nikaia; see how `drive_actions`/`drive_changed` test a script, and *CI has no target/fast* in `docs/builder-facts.md`) running `commit.nika -- --dry-run` on a fixture report named `X.md.report.md` beside a fixture brief `X.md` with an `Attribution:` block and a `Body:` line; the expected stdout shows both.

## Done when
The new test passes at both settings; tests naming `commit.nika` green. Commit message: `commit.nika: brief of a .md.report.md, Body: field`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-commit-brief-name.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
