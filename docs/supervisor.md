# The main session (supervisor)

What only the main session does: the session that starts the agents, talks to
the owner and holds the versions. Builders and the architect do not need this
file; the rules for everyone are in `CLAUDE.md`, the architect's charter and
the hand-over between main sessions in `docs/architect.md`.

## Watching CI

* **The supervisor checks CI on `develop` with `nikaia run scripts/release.nika
  -- --loop 20 DIR` in the background, or `scripts/status.nika` by hand.** A red
  run is fixed forward at once (a fix-forward builder), because `main` and
  everything that hangs on it (the website, `promote`) stands still while
  `develop` is red.

## Versions and error codes

* **Version raises and new error codes are the supervisor's.** An agent
  commits its change without touching the version lines, `CHANGELOG.md` or the
  badges, and names in its report what the change decided, for the
  supervisor's heading; the supervisor raises the version once per finished
  package. A new `NK` code is taken only from the block the supervisor reserved
  for that agent; one that needs more asks for it in its report.
* `scripts/bump.py new` raises the version everywhere (spec lines, **Date:**,
  CHANGELOG, `self_hosting.py` row, badges); after a rebase `scripts/bump.py
  rebase-fix` repairs a collision, `check` verifies. Compute the next version
  after the rebase; on a conflict in the version lines, `CHANGELOG.md` or
  `assets/badges/*.svg`, take the next free number and re-run `python3
  scripts/badges.py`. Never merge an SVG by hand.

## Agents in the shared checkout

* **Stop hook:** while agents run, the tree is dirty and the stop hook
  (`stop-hook-git-check.sh`) complains about uncommitted changes. Those files
  are the agents' to commit. The supervisor answers the hook with a single `.`
  and neither commits nor pushes them.
* If an agent's push is rejected, the supervisor rebases when needed; the
  agent goes on committing. An error one agent causes in another's file is the
  supervisor's to resolve if the agent that caused it does not repair it.
* **Agents: few, one per large module, none on a file another one edits.**
  Splitting one file into slices costs more in conflicts and repeated context
  than it saves. Leaves first: a consumer moved before its dependencies finds
  its errors late. Record the state in the commit message after each chunk.
  One builder at a time for now.
* **Tokens: one agent, one slice, at most 100-150 turns, then a fresh one.**
  Build agents are started as the `builder` type (`.claude/agents/builder.md`).
  The measure is tokens per moved line (target below 50 k); turns are the guard
  (about 120). A slice runs as a chain: a research agent writes a brief of at
  most 80 lines to a file, the builder gets its path and the steps. The rs2nika
  draft is the default start (report `rs2nika: kept N of M lines`); a compiler
  defect is fixed first. After each agent, `nikaia run scripts/tokenspeed.nika
  -- --short AGENTS_DIR` prints one row for the *Runs* table of
  `docs/tokenspeed.md`. The figures and the reasons are in
  `docs/tokensaverstrategy.md`; the ordered steps for the core of `emit` and
  `check` in `docs/selfhosting-plan.md`.

## Decision rounds with the owner

Some sessions only prepare and take decisions; another session builds them.
What the owner expects from such a round:

* **Start every topic with an introduction from zero**, before any table,
  option or error code: what the feature is, a small example, what the
  program or the user sees today, and why a question arises at all. Never
  open with the question or the options.
* **Say where the question comes from**: which issue or record asks it, who
  wrote it, and whether anything concrete needs the answer now (a program,
  a user, a measurement) or it is bookkeeping. A question nothing needs may
  be answered *as it is today, until someone needs more*.
* **Check what is already decided before asking.** Read the records the issue
  cites (they may have been merged into another record since) and the code.
  Do not ask again what a record already settles, and do not take a record's
  *deferred* for the owner's current view either - say which it is.
* **Facts before options, checked in the code**: what the compiler does today
  (try it), what is measured and what is not, what goes wrong if the
  decision is wrong and how badly (a wrong value, a refusal, undefined
  behaviour). Name the risk honestly, also where it argues against the
  recommendation.
* **A topic that is not prepared well enough is not asked yet.** Say what is
  missing and prepare a decision basis first: the problem, today's state,
  how other languages and tools do it, options with their consequences, the
  measurements still owed, and a recommendation.
* **Do not ask which issue to take next**: the order is arbitrary, pick one.
* **What the language offers is decided from what it is for**, the coherence of
  its types and what other languages count as basic - never from what this
  repository's programs happen to call (ADR-288 D11 is about how a `std` entry
  is made, not about what exists). *Waits for a program that needs it* is not a
  reason; say *not decided here* and why.
* **A decision lands in three places, each with its own job: the ADR says
  why, the specification says what, the issue tells the history.** The
  deciding session writes the ADR (a new D, or a new record) and the
  specification's text (including Part III's code table) itself, in the same
  round - not left to whoever builds it. An ADR states the reasons and the
  rejected alternatives, not how the question was found or what was broken
  before; that is the issue's. The specification states the rule and nothing
  else: no status, no history, no reasons, no padding.
* **The issue gets the history and the build order**: how the question came up,
  what was checked, the decision in short with links to the ADR and the
  specification, and a numbered *To build* list with tests that another session
  can build from. Leave the issue open until it is built.
