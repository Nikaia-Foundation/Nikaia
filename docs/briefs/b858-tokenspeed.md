---
agent: builder
model: sonnet
max_turns: 90
budget_usd: 12
---
# Brief: agent.nika records every run in docs/tokenspeed.md; tokenspeed moves to Nikaia

Owner approved, 2026-10-26. Tooling: no version raise, no CHANGELOG.

## Facts
* `scripts/tokenspeed.py` (189 lines) reads agent transcripts (tokens, turns, active minutes, gaps over 600 s cut) and git history (Rust code lines gone per commit, the `scripts/self_hosting.py` counting rule, the attribution rules in its `ATTRIBUTION` text and `gone_in`). `--short DIR` prints one *Runs* row: `| date | agent | module | turns | context M | Rust lines moved | tokens per moved line | |`.
* `scripts/agent.nika` (198 lines) starts `claude -p`, writes `<brief>.run.json` (it has `session_id`), checks the report and the gate (`agent_gate`, about line 188), then prints `done <sha> ...`. Nothing writes the *Runs* table today: it is filled by hand and lags.
* A headless run's transcript is `~/.claude/projects/-home-user-Nikaia/<session_id>.jsonl` (check the exact path on one `*.run.json` first); the Task-tool agents' transcripts are under `<session>/subagents/`.
* CLAUDE.md: helper tools are written in Nikaia; Python in `scripts/` moves when it is next reworked. What chafes on the way (a missing `std` entry, a lowering defect) is a finding: fix it in the compiler or file a `ready` issue, no workaround.

## Files (only these)
`scripts/agent.nika`, `scripts/tokenspeed.nika` (new), `scripts/tokenspeed.py` (deleted at the end, with its callers' mentions in these files), `docs/tokenspeed.md`, the test under `tests/compiler/tests/`. Two other builders run in the same tree (#856 in `tools/check_*` and the emitter, #857 in `emit/mod.rs`, `tools/emit_stmt.nika`): do not touch their files, do not build the compiler unless a finding needs it.

## To build
1. `scripts/tokenspeed.nika`: the same two modes and the same output as the Python, byte for byte (`--short DIR|FILE`, and the full tables); same counting and attribution rules. Compare with the Python on two transcripts before deleting it. Keep the row formatting in one function.
2. Test: `tests/compiler/tests/tokenspeed_row_formatting.nika` + `.stdout` (that function, a copy of it if the test cannot `use` a script: say which in the report): a row with moved lines, one with 0 (`-`), thousands separator.
3. `agent.nika`: after the gate step, run `nikaia run scripts/tokenspeed.nika -- --short <transcript>` and append the row to the *Runs* table of `docs/tokenspeed.md`, idempotent per agent id (a second run with the same id replaces nothing and adds nothing). agent.nika only writes the file and does not commit it; the supervisor's commit takes it (say so in a comment in agent.nika).
4. Backfill: rows for today's (2026-10-25/26) builder runs whose transcripts exist (`*.run.json` in the scratchpad's `briefs/`, the session's `subagents/`); a run without a transcript is named in the report, not guessed.
5. `nikaia run` both scripts once, `scripts/pre-push-check.sh`, commit, `Fixes #858`.

## Rules (CLAUDE.md)
* `develop` in the main checkout; `git add <files>` only; push fast-forward, no rebase. Commit after each step.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite.
* Count, never time: no seconds in a document (active minutes stay as the Python has them, only in the full table).
* `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/`; delete `target/release/nikaia-scratch`; report GB freed. Scripts in files, small reads, no foreground sleep.

## Report (at most 15 lines, `<brief path>.report.md`)
Result, Files, Commit, Fixes, Pushed, Tests, Deviation, Found, Needs, rows backfilled, GB freed, Owner (form: `docs/architect.md`, *Briefs and reports*).
