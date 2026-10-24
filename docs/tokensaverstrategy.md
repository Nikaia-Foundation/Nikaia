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

* At most 3 agents, one per module, none on a file another one edits; give
  each its own worktree (a shared checkout lets one agent switch the branch
  under the other).
* Uncommitted files in a worktree belong to its agent; a stopped agent cannot
  be resumed, so a fresh one starts with `git status` and `git diff`.
* Version, `CHANGELOG.md` and badges collide between agents: tell each to
  take upstream's number after the rebase and re-run `scripts/badges.py`.
