# The architect and the main session

How work is organised when a session is started by the owner. Rules for the
agents' tokens are in [`tokensaverstrategy.md`](tokensaverstrategy.md); the
rules for commits are in `CLAUDE.md`.

## Roles

| Role | Who | Tools | Job |
| :--- | :--- | :--- | :--- |
| Owner | the person | - | Sets goals, stages and priorities; decides what is theirs |
| Main session | the session the owner started | all | Talks to the owner; starts and restarts the architect; starts builders from the architect's work orders; watches cost; carries information between them |
| Architect | `.claude/agents/architect.md`, Opus | read, search, `Bash`, `Edit`/`Write` for docs, GitHub issues read/write | Decides what is built next and how; writes ADR and specification text; keeps issues current; improves the process |
| Builder | `.claude/agents/builder.md` | six file and shell tools, no GitHub | Builds one or two steps, commits, pushes, reports, ends |

The architect cannot start agents (a subagent has no `Agent` tool). So the
architect **coordinates by orders**, and the main session carries them out.

## Effort

The main session starts the architect with `effort: medium` (owner's decision).
It raises it to `high` for one task only when the owner asks for it.

## The loop

1. The owner states a wish or answers a question to the main session.
2. The main session starts the architect with a short task: the wish, the issue
   numbers, and the last report. Nothing else; the state is in the repository.
3. The architect returns (at most 40 lines): the decision, **one work order**,
   the repository and issue changes it made, and questions for the owner.
4. The main session relays the owner's questions, and the answers go back to
   the architect (a fresh one, or the same while it is young).
5. The main session starts one builder with the work order. The builder builds,
   commits, pushes, reports and ends.
6. The main session reads the builder's report, puts it into the next
   architect task (the report, not the transcript), and goes to 2. A builder
   that hit an error does not go on: its report goes to the architect, whose next
   order is the repair (compiler defect first, `CLAUDE.md`'s *Fix, do not work
   around*), then the step that was blocked.

## The work order

What the architect hands the main session for a builder, in this form:

* **Issue and steps**: `#N`, the one or two steps of its *To build* list.
* **Files**: the files to read first, and the files the builder may change.
* **Probes**: the whole matrix of cases to run after the single build.
* **Done when**: the touched tests to run (never the full suite: CI runs it on
  the push), the commit message line (`Fixes #N` only for the last step).
* **Measure**: none, unless the step's point is a number. Then the count to
  report (a work count the tool prints, such as files re-lowered, comes
  first) and the smallest input for callgrind; never callgrind over a whole
  `lower-std` or a test run ([`long-jobs.md`](long-jobs.md)).
* **Stop and report if**: an open design choice, an error in another file.
* **Two lines of context** the builder cannot find in the repository.

## The main session's duties

* **Cost watch.** Look at each agent's size. A builder past about 120 turns or
  200k context is told to push what is verified and end. An architect past about
  80 turns or 150k context ends the same way. Restart with a fresh agent; never
  resume one for a second slice.
* **CI watch.** After each builder's push, one background call
  `gh run watch <run id> --exit-status` on that commit's run; it wakes the
  session at the end, no polling. The next builder starts meanwhile. A red
  run goes to the architect, whose next order is the repair.
* **Hand-over.** A fresh agent gets: the task, the issue numbers, the two or
  three facts that are in no file. If a fact will be needed again, it is written
  to the repository (the architect's *State* below, a doc, a commit message), not
  carried in the prompt.
* **Owner channel.** The owner talks to the main session only. When the
  architect needs a decision, the main session asks the owner, following
  `CLAUDE.md`'s *Decision rounds* (introduction from zero first).
* **Supervisor duties** from `CLAUDE.md` stay with the main session: version
  raises, `CHANGELOG.md`, badges, reserved `NK` blocks, rebases, a red `develop`.
* **No building in the main session**, except what a work order cannot reach.

## Stages

The owner sets the stage and moves to the next. Each stage ends with a short
note in *State* on what it measured.

| Stage | Rule | Goal |
| :--- | :--- | :--- |
| 1 Serial | One builder at a time. A defect found on the way is repaired first, by the same chain (architect orders, builder repairs), then the work resumes | Self-hosting forward (#125, `selfhosting-plan.md`); the right tools; the tools improved by the tools; compiler defects fixed |
| 2 Two | A second builder works on parts the first does not touch (the architect names the files of each). Each commits only its own files; the main session rebases | Speed, once stage 1 has removed the common stalls |
| 3 More | Decided by the owner, with the figures of stage 2 | |

## Self-optimisation

Every cycle the architect looks at the builder reports for waste: turns on
polling, repeated builds, a missed rule, a chafing tool, a missing script. It
changes the rule or the tool at the source and notes the figure in
[`tokenspeed.md`](tokenspeed.md). The measure is tokens and turns per finished
step, and steps finished per day; counted, not timed (`CLAUDE.md`, *Measuring
performance*).

## State

*Updated by the architect when it ends.* 2026-10-24, develop 97385b5, 0.0.1204.
Branch `wip/bounding-candidates-2026-10-19` (stash on 0.0.1080) checked
2026-10-24: everything in it is on develop in a later form (`bounding_grew`
returns the failed counters, `bounding_for`/`bounding_while` re-walk in a
loop); no order owed, branch deletable.
Stage: 1 (serial). Current issue: #125, C-7 of `selfhosting-plan.md`: U-d0 and
U-d1 done, **U-d2** (`check/attributes.rs`, 714 lines, to
`tools/check_attributes.nika`) running; then **#667** (owner: before U-d3):
steps 1-3 are in `sysroot.rs` (NIKAIA_LOWER_TIMES, `LoweredBefore`,
`in_parallel`), but the key is the whole package plus the binary's hash, so the
real cycle (edit one .nika, rebuild) always misses; order is per-file keys on
file + package interface, then step 4. Then U-d3 .. U-d6, then E10/E11.
Modular compiler (owner's question): basis in `modular-compiler.md`, #793
(`decision`); #667 step 4 is the next order after the current #667 builder.
Long jobs (owner's question): basis in `long-jobs.md`, #794 (`decision`); no measuring agent; CI is the verifier, the main
session watches it. develop red since 9f090392 (badges check, arm64 Test):
the repair comes before the next step.
Known risk for new tools files: #791 (methods matched by bare name).
Parked: branch `wip/bounding-candidates-2026-10-19` (loop counter candidates in
the bounds analysis, ~24 lines, 120 versions stale): not merged, no order until
self-hosting reaches `bounds`; re-done fresh then, the branch is a sketch only.
