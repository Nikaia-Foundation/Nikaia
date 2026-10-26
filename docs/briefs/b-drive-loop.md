---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 8
---
# Brief: one driver runs a whole batch, the main session wakes once (owner, 2026-10-26: faster, fewer tokens)

Why: today the main session spends 3-5 turns per builder (run `cycle.nika`, read the line, commit the report files, start `agent.nika`, clean false ESCALATE). Every turn re-reads its whole context. One program that chains the batch wakes it once per batch instead of once per builder.

## To build
1. `scripts/drive.nika` (pattern: `cycle.nika`, `release.nika --loop`; header comment as theirs):
   `nikaia run scripts/drive.nika -- DIR [--max N]` loops: run `cycle.nika -- DIR`; on a `NEXT <brief> ...` line run `agent.nika -- <brief> --dir DIR` (a program, as `cycle.nika` runs `status.nika`), then commit the main session's files of that run (`<brief>.md.report.md` or `<stem>.report.md`, `<brief>.run.json`, `DIR/cost.txt`, `DIR/tokens.txt`, `docs/tokenspeed.md` if changed) with `git add` of exactly those paths, message `briefs: <brief name> report`, the Attribution block below as trailer, `git push origin develop` (rejected: `git pull --rebase origin develop` once, push again; still rejected: stop). Then loop. Stop on anything else and print exactly one final line: the `cycle.nika` line that stopped it (`CUT`, `RED`, `OWNER`, `BATCH DONE`, `BUMP pending`), or `ESCALATE <text>`, or `MAX <n> builders run`. `--max` defaults to 5. Intermediate output: one line per builder (`agent.nika`'s `done ...` line), nothing else.
2. Test: `tests/compiler/tests/` same pattern as `batch_frontmatter.nika` for the pure parts (which line stops, which paths are committed for a brief name); the loop itself is probed with `--dry-run` (prints the commands it would run for a DIR with a two-brief `batch.txt` and fake reports). Put the probe output in the commit message.
3. `docs/architect.md`, *The main session's cycle*: step 1-3 become `nikaia run scripts/drive.nika -- docs/briefs` with `run_in_background`; the main session acts only on its final line. Commit `scripts: drive.nika runs a batch to its first stop`.

## Files you may change
`scripts/drive.nika`, `tests/compiler/tests/drive_*`, `docs/architect.md` (*The main session's cycle* only). Not `batch.nika`, `status.nika`, `cycle.nika`, `agent.nika` (#862's builder changed them just before you; read, do not edit; a defect there goes under Found).

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
