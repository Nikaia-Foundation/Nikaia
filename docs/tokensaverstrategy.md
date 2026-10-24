# Token saver strategy (agents)

Measured on two long self-hosting agents (2026-10-11, #125): 695 and 768
turns, 219 M and 276 M context tokens read, 6-7 k tokens written. What costs
is not the output but the **prefix every turn re-reads**.

| Fact | Number |
| :--- | :--- |
| Context at start (system prompt, skills, `CLAUDE.md`) | about 49 k |
| Growth per turn | about 750 tokens |
| Share of the cost at context > 300 k | 71-78 % |
| Context at the end | 566-626 k |
| Same work, agent restarted every 120 turns (computed) | about one third |
| What fills the context | tool results 630-770 KB, thinking 520-555 KB, tool calls (inline scripts) 410-440 KB |
| Short Opus planning agents | 72-159 turns, about 100-130 k, 7-21 M in all |

## What to measure: tokens per moved line, guarded by turns

* **The result is judged by tokens per moved line**: context tokens read
  divided by the Rust code lines the agent's commits removed
  (`scripts/tokenspeed.py`, the `scripts/self_hosting.py` line rule). Measured
  so far: long agents 143 k, restarted ones 71 k, a clean slice 37-54 k, an
  entangled one 88-164 k. The target is **below 50 k**. A slice that moves no
  Rust (a compiler fix, a plan) is not given a figure; its value shows in the
  slices after it.
* **Turns are the guard, not the measure.** Every turn re-reads the whole
  context, which grows about 1 k per turn, so the cost grows with the square
  of the turns: the last third of a long agent is more than half its cost.
  An agent can count its turns but cannot see its context, so its limit is
  set in turns (about 120, `.claude/agents/builder.md`); the supervisor, who
  can see the transcript, checks the context (about 200 k).
* **The measurement is cheap for the supervisor.** After each agent ends,
  `scripts/tokenspeed.py --short AGENTS_DIR` prints one Markdown row (the
  latest agent, counted only over the commits of its window); the supervisor
  appends it to the *Runs* table in `docs/tokenspeed.md` with the agent's
  rs2nika line. The full tables are drawn only for a dated snapshot.

## Chain: research agent, then builder

The supervisor's own context is the most expensive one: it lives the whole
session. So it does not read the plan, the call graph or the draft itself.

1. A **research agent** (`builder` type, told not to edit or commit) reads the
   plan's section (`docs/selfhosting-plan.md`), the call graph
   (`scripts/c6/cg.py`) and the rs2nika draft of the slice, and writes a
   **brief of at most 80 lines** to a file in the scratchpad: the functions in
   order, the callees not yet in Nikaia, the draft's path, the defects it
   expects. It reports the path and five lines.
2. The **builder** gets the path to the brief and the steps, nothing else.
3. The supervisor reads only the two reports.

## The rs2nika draft is the default

Every move of a Rust function starts from the draft of
`crates/rs2nika/convert` (`rs2nika --profile check|emit F.rs`), never from an
empty file. The builder reports in one line how much of it it kept
(`rs2nika: kept N of M lines; fixed by hand: ...`), and the supervisor
records it in the *Runs* row. A pattern the builder fixes by hand more than
once becomes a rule of the converter (`--rules`) or a `--lint` check, in its
own commit.

## Compiler defects first

A lowering or checker defect that a slice meets is fixed in the compiler, in
its own commit or by its own agent, **before** the slice goes on (owner's
direction: no workaround). A builder that meets one stops and reports it with
a probe; it does not route around it, which costs more turns than the fix.

## No parallelism for now

One builder at a time (owner's direction, 2026-10-24): first see what the
rules above bring in the *Runs* table, then decide on more agents at once.

## Rules

* **One agent, one slice, at most 100-150 turns.** It builds, tests, pushes,
  reports and ends. The next slice gets a fresh agent with a short task that
  names its section of the plan (`docs/selfhosting-plan.md`). Never resume an
  agent for a second slice: its context is what costs.
* **A task is short and points to files.** Plan, rules and state live in the
  repository (`CLAUDE.md`, the plan file, the last commit message), not in the
  prompt, and not in issue comments. Do not make every agent read issue
  threads (19-30 KB each): the orchestrator summarises them in two lines.
* **Scripts live in files.** A call graph, a bump, a bulk edit is written
  once to a file in the scratchpad and called; no heredoc of 100 lines in
  every turn (410 KB of the context were such calls). `scripts/` keeps the
  ones worth sharing.
* **More work per turn.** Set the directory once (`git -C`, absolute paths),
  chain what depends on nothing into one call, no `cd x && ...` per call
  (386 of them in one agent). Independent tool calls go in one message.
* **Small reads.** `grep -n` first, then `sed -n` of at most 150 lines; never
  a whole 15 k-line file; `grep -c` or `head` for counts; cargo with
  `--message-format short`, test output filtered to the failures.
* **Think little on mechanical work.** A move by the call graph needs the
  graph, not deliberation. Deliberate where a design choice is open, and then
  stop and report it instead of guessing.
* **The architect is a standing role** (`.claude/agents/architect.md`, charter
  in [`architect.md`](architect.md)): started and restarted by the main session.
* **Architecture goes to one Opus agent on high effort, read-only, with a
  question and a page limit** (the plans of 2026-10-11 cost 7-21 M each, a
  fraction of one long build agent). It returns a decision and ordered steps;
  builders get those steps, not the research.
* **Builds are shared.** Delete `target/nikaia-project-tests` and
  `target/release/nikaia-scratch` after each test run; no two full test runs
  at once (disk, and the same machine).
* **Check the cost while it runs.** The orchestrator looks at an agent's
  transcript size: past about 150 turns or 250 k context the agent is told to
  push what is verified and finish.

## Measured again (2026-10-17, ten build agents)

| Fact | Number |
| :--- | :--- |
| Context at start | 52 k, of it about 35-40 k the descriptions of 41 tools a builder never calls (remote sessions, artifacts, docs) |
| Growth per turn | about 1 k |
| Growth, by kind | about half thinking, a quarter to a third tool results (`grep`, `sed`/`cat`, `git`), the rest tool calls |
| Turns | 63-357; seven of ten past 200 |
| Share of the cost after turn 120 | 52-79 % for the long ones |
| Turns spent polling a build or a test | 10-20 % of the cost |

So: build agents are the `builder` type (`.claude/agents/builder.md`, six
tools, the limits above in its prompt); a task names one or two steps of a
*To build* list, not the whole list; waiting is one call.

## Orchestrator

* Start build agents as `builder`; give each one or two steps.
* Watch each running agent's transcript (turns, context); past about 150
  turns or 200 k tokens tell it to push what is verified and report.

* One builder at a time for now (above). When more run again: one per module,
  none on a file another one edits, each in its own worktree (a shared
  checkout lets one agent switch the branch under the other).
* After each agent: `scripts/tokenspeed.py --short`, one row into
  `docs/tokenspeed.md`.
* Uncommitted files in a worktree belong to its agent; a stopped agent cannot
  be resumed, so a fresh one starts with `git status` and `git diff`.
* Version, `CHANGELOG.md` and badges collide between agents: tell each to
  take upstream's number after the rebase and re-run `scripts/badges.py`.
