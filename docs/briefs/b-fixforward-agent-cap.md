---
agent: builder
model: sonnet
max_turns: 40
budget_usd: 5
---
# Fix-forward: `agent_script` red since 26db5c6 (day's cost cap 200 to 1000 USD)

Score: red `develop` is fixed forward before any other work (`CLAUDE.md`); `main` stands still until it is green. Found by the b890 builder.

## Problem
26db5c6 raised the day's cost limit in `scripts/agent.nika` (`fn main`, about line 191) from 200 to 1000 USD (owner, 2026-10-27; wanted, do not change it back). `tests/compiler/tests/agent_script.nika` still writes a `cost.txt` of `100.6` + `99.6` = 200.2 USD into `{tmp}/cap` and expects `cap: code=2 ESCALATE escalate=true` (`agent_script.stdout` line 8); now the cap is not reached, so the test fails.

## Locations
* `tests/compiler/tests/agent_script.nika` line 60: `fs::write(f"{tmp}/cap/cost.txt", ..., f"{today} 100.6\n{today} 99.6\n")`.
* `tests/compiler/tests/agent_script.stdout` (expected output; should stay unchanged).

## To build
1. In `agent_script.nika` line 60, write two entries that sum just above 1000: `{today} 500.6\n{today} 499.6\n` (1000.2 USD). Leave the `small` case (14.35 USD) as it is. `agent_script.stdout` must not need a change; if the run shows any other difference, stop and report `blocked` with the diff.
2. Run `target/fast/nikaia test --both-settings --project tests/compiler` filtered to `agent_script` (or the whole package if no filter exists). Commit message: `tests: agent_script's cap case sums above 1000 USD (fix-forward 26db5c6)`.

Read `docs/builder-facts.md` first.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-fixforward-agent-cap.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
