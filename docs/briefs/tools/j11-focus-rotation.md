---
agent: builder
model: opus
effort: medium
max_turns: 110
budget_usd: 22
---
# Brief: focus rotation in the driver, effort per brief, architect only on demand (owner, 2026-11-01)

Score: process. The owner saw the last hours run with fewer stalls: one area at a time, sequential, who finds a defect fixes it, the next task chosen by a rule and not by an architect run after every batch. This brief puts that into `scripts/drive.nika` and `scripts/agent.nika`, so the normal process can start again.

Facts: Where things live, Traps already met, Commands

## What the owner decided (2026-11-01, no issue: build and try)

1. **One driver, one area at a time, rotating.** A file `docs/briefs/focus.txt` names the areas in order and the block length; the driver runs a block of up to N builder briefs in the current area, then moves on to the next. Areas: converter (`docs/briefs/converter/`, new), self-hosting (`docs/briefs/`), bugs (`docs/briefs/bugs/`). No second lane in parallel.
2. **Effort per brief.** Brief frontmatter `effort: low|medium`; `low` for an easy step (one step of a *To build* list, a known pattern, a bug with a probe and a known place), `medium` for a hard one (a defect of unknown cause, a new converter pattern, a move with unknown gaps). Missing: `medium`. The architect's tasks run at `medium`.
3. **The architect only on demand**, never after every batch: (a) a report cuts the batch (`partial|blocked`, a `Found:`, a `Needs: architect decision`), (b) the current area's queue is empty and needs briefs (it writes at most 3, for that area only), (c) an `OWNER` line needs a decision paper (unchanged). A finished block with briefs left in the next area goes straight on.
4. **Figures at the end of a block only**, written by the driver, no agent measures: one line per block in `docs/briefs/focus-log.md`.

## To build

1. **`scripts/agent.nika`: `--effort`.** Read `effort` from the frontmatter with `agent_front` (default `medium`), pass `--effort <level>` to `claude` beside `--model` (the CLI takes `--effort low|medium|high|xhigh|max`), also in `agent_debrief_args`. Refuse a value other than `low|medium|high` with an `ESCALATE` line. `--info` prints it as a fifth word; `cycle.nika`'s `NEXT` line and the driver's parsing of it take the fifth word too (check every reader of `--info` / `NEXT ... <budget>`: `batch.nika`, `status.nika`, `cycle.nika`, `drive.nika`). Add `effort: medium` to the frontmatter of every `architect-task.md` (`docs/briefs/`, `docs/briefs/*/`) and of `docs/briefs/qa-task.md`.
   Test: extend `tests/compiler/tests/agent_script.nika` (or a new `agent_effort.nika`) with a brief with `effort: low`, one without, one with `effort: huge`.
2. **`docs/briefs/focus.txt` and `--focus`.** Format, plain lines:
   ```
   block 3
   area converter docs/briefs/converter
   area selfhosting docs/briefs
   area bugs docs/briefs/bugs
   current converter 0
   ```
   `nikaia run scripts/drive.nika -- --focus [--max N] [--dry-run]` reads it and drives `current`'s DIR as today's single-DIR loop does (cycle, NEXT → agent, commit, suite at the end of the block, push, QA), counting builder runs in that area. The block ends when the count reaches `block`, or the cycle for that DIR says `BATCH DONE`. Then: suite as today; one line appended to `docs/briefs/focus-log.md`; `current` set to the next area with count 0 (wrapping), `focus.txt` committed with the run files; the loop goes on in the new area. A `CUT` (decision trigger a) starts the architect of that DIR as today. A `BATCH DONE` in an area entered with nothing to run (queue empty, trigger b) starts that DIR's architect with the stop line `QUEUE EMPTY <area>`; its task text says: write at most 3 briefs for this area, each with `effort:`, then end. If the architect leaves it empty again, skip the area for this rotation with a line `FOCUS <area> empty, skipped` (no escalation); all areas empty in one rotation: `ESCALATE focus: every area empty`.
   A red `develop` (RED) and bug briefs with a `Blocks:` line keep their priority: RED goes to the current DIR's architect as today; before each block, if `docs/briefs/bugs/batch.txt` holds an unrun brief with a `Blocks:` line, run it first (it counts for the bugs area).
   The bump: one driver now, so `--focus` bumps (no `--no-bump`): at a block end with CI green, apply `cycle.nika -- DIR --apply` for each area's DIR with a pending bump, one at a time.
   The single-DIR mode stays as it is (other lanes, `yaml`, `tools`, `prover`, still run with it).
3. **The focus log line.** `YYYY-MM-DD HH:MM <area> briefs <n> done <d> partial <p> blocked <b> noop <o> tokens <t> cost <c> queue <q>` where the counts come from the block's reports' `Result:` lines, tokens and cost from the run files the driver commits (`tokens.txt`, `cost.txt` of the DIR, the rows of this block's briefs), `queue` the unrun briefs left in that DIR's `batch.txt`. For the converter and self-hosting areas add `kept <k>/<m>` summed from the block's reports' `Rs2nika:` lines when present. No GitHub calls, no extra measuring run.
4. **`docs/briefs/converter/`**: create it with an empty `batch.txt` and an `architect-task.md` (copy of `docs/briefs/architect-task.md` reworded for the converter: `tools/rs2nika*.nika` and its ledger, the score in `docs/rs2nika-score.md`, briefs at `effort: low` where the pattern is known). The architect writes its first briefs on the first `QUEUE EMPTY converter`.
5. **Tests** in `tests/compiler/tests/` (the way `drive_actions.nika` tests `drive_action`): the focus-file parse and advance (wrap, skip of an empty area, all empty), the log line from three sample reports, the `Blocks:` priority. Pure functions taking text, as `drive_action` does; `--dry-run` of `--focus` on a copy of the tree shows the next step.
6. **Docs, short**: `docs/supervisor.md`, *Running the lanes*: the main session starts `drive.nika -- --focus` instead of the two lanes; `docs/architect.md`, *Lanes* and *Briefs and reports*: the rotation, the three triggers of the architect, `effort:` in the frontmatter with the low/medium rule. Replace, do not add beside the old text where it contradicts.

## Locations
`scripts/drive.nika`: `main` (args at about :837-870, loop after), `drive_action` (:251), `drive_cycle` (:823), `drive_agent` (:801), `drive_commit` (:746), `drive_suite` (:712). `scripts/agent.nika`: `agent_front` (:35), the args at :60-71, `agent_debrief_args` (:157), `--info` (:364). `scripts/cycle.nika` (NEXT line), `scripts/batch.nika`, `scripts/status.nika`. Resolve with `nikaia run scripts/where.nika -- NAME`.

## Done when
`--focus --dry-run` prints the next step on today's tree; the tests pass through `nikaia test --project tests/compiler` (the `drive_*` and `agent_*` ones and yours); single-DIR mode unchanged (its tests pass); build, fmt, clippy, `pre-push-check.sh`; committed locally in steps. Changelog: one line (tooling). Stop about 15 turns before `max_turns`: commit what is finished and report `partial` with the steps left.

## Stop and report if
An edit is needed in Rust or outside `scripts/`, `tests/compiler/tests/drive_*|agent_*|focus_*`, `docs/briefs/focus.txt`, `docs/briefs/converter/`, the `architect-task.md` / `qa-task.md` frontmatter, `docs/supervisor.md`, `docs/architect.md`. A form Nikaia lacks: stop (rules below). An error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except the ones named above and your report. A builder for #1120 works in the emitter at the same time: never touch `crates/`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the main session tests and pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia` (only if needed), `cargo fmt`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Who finds a blocking defect fixes it (owner, 2026-11-01): a compiler defect that blocks a script here is fixed in the compiler with a test as its own commit, if it fits the time box; else `Result: blocked`, `Found:` + probe. A form Nikaia lacks and no ADR refuses: `Result: blocked`, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`.
* Time box: going in circles: stop, `Result: partial` with what is done and left.
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/j11-focus-rotation.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (bare word), Files, Commit, Body, Fixes, Pushed, Tests, Gate (`green` or `red <check>`), Deviation, Found, Needs, Changelog, Owner, Disk freed, `Rs2nika: none`. `none` where nothing, as the bare word. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
