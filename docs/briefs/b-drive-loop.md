---
agent: builder
model: sonnet
max_turns: 80
budget_usd: 12
---
# Brief: one driver runs builders and the architect, the main session wakes only on an escalation (owner, 2026-10-26: faster, fewer tokens)

Why: today the main session spends 3-5 turns per builder (run `cycle.nika`, read the line, commit the report files, start `agent.nika`, clean false ESCALATE). Every turn re-reads its whole context. One program that chains the batch wakes it once per batch instead of once per builder.

## Owner's rules for this driver (2026-10-26)
* One agent at a time (stage 1): never two builders, never a builder beside an architect. The script itself may run cheap things in the background (a `gh run watch`, a `cargo build`).
* The architect keeps its focus: one batch of 2-3 briefs per run, not more.
* The main session is woken only on an escalation; otherwise it stays free for the owner.

## To build
1. `scripts/drive.nika` (pattern: `cycle.nika`, `release.nika --loop`; header comment as theirs): `nikaia run scripts/drive.nika -- DIR [--max N] [--dry-run]` loops on `cycle.nika -- DIR` and acts on its lines:
   * `NEXT <brief> ...`: run `agent.nika -- <brief> --dir DIR` (a program, as `cycle.nika` runs `status.nika`); then commit the main session's files of that run with `git add` of exactly these paths, if present: `<brief>.report.md` (and the `<stem>.report.md` form), `<brief>.run.json`, `DIR/cost.txt`, `DIR/tokens.txt`, `docs/tokenspeed.md`; message `briefs: <brief name> report`, the Attribution block below as trailer; `git push origin develop` (rejected: `git pull --rebase origin develop` once, push again; still rejected: escalate).
   * `BUMP pending` with CI green: run `cycle.nika -- DIR --apply`.
   * `CUT`, `RED`, `BATCH DONE`: start the architect with `agent.nika -- DIR/architect-task.md --dir DIR` after writing the stop line (and, for `RED`, the failing jobs) to `DIR/architect-input.txt` and deleting the old `DIR/architect-task.md.report.md`. Write `DIR/architect-task.md` once (committed): frontmatter `agent: architect`, `model: opus`, `max_turns: 80`, `budget_usd: 25`; body: read `docs/architect.md` and `DIR/architect-input.txt`, decide the next batch of 2-3 briefs in `DIR`, rewrite `DIR/batch.txt`, commit and push its own files; GitHub only through `gh api repos/nikaia-foundation/nikaia/...` (REST; GraphQL and the MCP tools are not available headless); report to `DIR/architect-task.md.report.md` with `Result:`, `Batch:` (the briefs in order), `Owner:` (a question or `none`). The architect does not count as a batch brief.
   * Escalate (stop, print exactly one final line, exit 2) on: `OWNER ...`; `ESCALATE` (agent.nika's file); the architect's report with `Owner:` not `none` or `Result:` not `done`; an architect run after which `cycle.nika` still prints the same stop line; a second `RED` on the same sha; `--max` agents run (default 8, architect runs count); a rejected push after one rebase.
   * Output while running: one line per agent run (`agent.nika`'s `done ...` line, prefixed `builder` or `architect`), nothing else, so the background task's log stays small.
2. Tests: `tests/compiler/tests/` in the pattern of `batch_frontmatter.nika` for the pure parts (which cycle line leads to which action; which paths are committed for a brief name). Probe the loop with `--dry-run` on a scratch DIR with a two-brief `batch.txt` and fake reports: it prints the commands it would run and the final line. Put the probe output in the commit message.
3. `docs/architect.md`, *The main session's cycle*: replace steps 1-5 with `nikaia run scripts/drive.nika -- docs/briefs` started with `run_in_background`; the main session acts only on its final line (escalations, step 6-7 stay). Commit `scripts: drive.nika runs builders and the architect until an escalation`.

## Files you may change
`scripts/drive.nika`, `docs/briefs/architect-task.md`, `tests/compiler/tests/drive_*`, `docs/architect.md` (*The main session's cycle* only). Not `batch.nika`, `status.nika`, `cycle.nika`, `agent.nika` (#862's builder changed them just before you; read, do not edit; a defect there goes under Found).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b-drive-loop.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
