# Working in this repository

**Session hooks are the sessions' to maintain** (owner, 2026-10-26): a session
may create and change the project's hooks in `.claude/settings.json` and
`.claude/hooks/`, and delete one that gets in the way of the work, without
asking first. `.claude/hooks/session-start.sh` builds `target/fast/nikaia` in
the background in a cloud session.

## Trunk-based: everyone pushes to `develop`, `main` only gets what was tested (#437)

* **`main` is the default branch, and what the outside sees: the last tested
  state.** A session starts on `main` or on a branch made from it, so the first
  thing to do is to switch to `develop`:
  `git fetch origin develop && git checkout -B develop origin/develop`.
* **`develop` is the trunk. Every session pushes to it directly, at once.**
  No pull requests, no waiting for CI, no full suite first: build,
  `cargo fmt`, and the tests the change touches are enough. Small commits,
  often.
* **Before pushing, `cargo clippy --all-targets -- -D warnings` must pass**: CI
  runs it as its first gate after the build, and a warning there turns every
  later job off (it kept `develop` red from the first self-hosting move until
  0.0.773, so `main` did not move). Lints about code the lowering writes are
  allowed on the `tools` module in `crates/nikaia-std/src/lib.rs`, not fixed in
  the generated files (`crates/nikaia-std/src/tools/lowered/*.rs`, index `lowered.rs`; a change to one `.nika`
  changes one file).
* **Before pushing, run `scripts/pre-push-check.sh`** (fmt, badges and version check, seconds): CI fails on these first.
* **Before pushing, rebase onto the latest `origin/develop`**
  (`git pull --rebase origin develop`). Other sessions push all the time.
  If the push is rejected, rebase again and push again.
* **A change to the language raises the version by one**: the `**Version:**`
  line of the three specification parts and a `CHANGELOG.md` heading, then
  `python3 scripts/badges.py` redraws the README's badges from that line
  (`scripts/pre-commit.sh` does it where it is installed). Never edit an SVG by
  hand; CI checks the badges with `--check`.
* **What raises it, and what does not.** The version is the specification's: it
  moves when what a program means, or what the toolchain does with it, moves.

  | Changed | Raises the version |
  | :--- | :--- |
  | `crates/` (compiler, parser, `std`) | yes |
  | Part I-III of `docs/specification/` | yes |
  | an ADR | only with the specification text it decides |
  | tests | no, unless they belong to a change above, in its package |
  | `guide/`, the notes in `docs/`, `README.md`, the website | no |
  | `examples/`, `benches/`, `scripts/`, CI, `editors/` | no |
  | `crates/rs2nika/` (the converter, a development tool not shipped with the toolchain) | no |
  | `packages/<name>/` (an official package, ADR-344 D1) | no: only that package's own version |

  A change that does not raise the version gets no `CHANGELOG.md` heading or
  entry either: its commit message is its record. It touches neither the
  version lines nor the badges, so it does not collide with other sessions.
* **Nobody pushes to `main`, and nobody resets or force-pushes it** (owner,
  2026-10-26): a `main` that moved onto an untested commit is fixed forward on
  `develop`, and the process that let it move is repaired (#861). CI runs everything on each push to `develop`.
  When every job passes, the `promote` job fast-forwards `main` to that commit
  (`.github/workflows/nikaia.yml`) - that commit, not `develop`'s tip, which
  may have moved on during the run. `main` is always a commit that passed.
* **Every push to `develop` gets its own CI run.** The concurrency group of a
  push contains the run id, so runs overlap and none replaces another. (With one
  shared group GitHub keeps one running and one pending run and cancels every
  older pending one, so most commits never got an answer.) `promote` skips a
  commit `main` already contains, so runs finishing out of order are safe. A
  docs-only push skips the build and test jobs (`scripts/ci-changes.sh`).
* **The commit that finishes an issue says `Fixes #N`.** `promote` closes an
  issue only when a commit it brings to `main` names it with `Fixes`, `Closes`
  or `Resolves` (`scripts/fixed-issues.py`); `(#N)` and `Part of #N` close
  nothing. So the commit that builds the last step of a *To build* list
  writes `Fixes #N`, and also names every issue that list absorbed
  (`Fixes #439, #482`). An issue left open on purpose gets a comment saying
  exactly what is still missing. One that no commit can finish - a process
  note, a duplicate, an answered question - is closed by hand, with the reason.
* **Every open issue carries exactly one status label**: `ready`, `decision`,
  `blocked` or `note` (`docs/README.md`). Whoever changes an issue's state
  changes the label: a decision taken makes it `ready`, a step that turns out
  to need the owner makes it `decision`, a blocker built frees what it blocked.
  A new issue gets its label when it is filed.
* **A red `develop` is fixed forward.** Whoever finds it red fixes it with the
  next commit, or reverts the commit that broke it, at once: until then `main`
  and everything that hangs on it (the website, `promote`) stand still, and
  work on `develop` goes on. How the main session watches CI:
  `docs/supervisor.md`.
* **Order of work: the architect's score (`docs/priorities.md`), delegated by
  the owner (2026-10-25).** Bugs before moving is the default: as a rule one
  builder works on compiler bugs and one on self-hosting, and self-hosting
  pauses while we find more bugs than we fix. The score overrides this
  default when its figures say so, and the brief states them.
* **Parallel agents in one session work together on `develop`, in one shared
  checkout.** Each agent commits after every finished step, so the others see
  the work at once. A commit adds exactly the files that agent changed
  (`git add <files>`), never `git commit -a` or `git add -A`: the working tree
  also holds the other agents' unfinished files. After each commit the agent
  pushes fast-forward only (`git push origin develop`). It does not rebase:
  if the push is rejected, the supervisor rebases when needed, and the
  agent goes on committing. No separate worktrees with
  `develop` checked out: a commit in one moves the branch under the others,
  and their index then shows the reverse of that commit as staged changes.
* **Version raises and new error codes are the supervisor's.** An agent
  commits without touching the version lines, `CHANGELOG.md` or the badges,
  names in its report what the change decided, and takes a new `NK` code only
  from the block the supervisor reserved for it.
* **An error in another agent's file is not touched, it is escalated.** A
  failing build, test or lint that comes from a file another agent changed is
  reported to the supervisor (the session that started the agents), not
  fixed. The supervisor resolves it if the agent that caused it does not
  notice and repair it itself.
* **Helper tools are written in Nikaia, not in Python or shell.** A new tool
  (a migration aid, a checker, a generator) is a `tools/*.nika` behind a
  `nikaia` subcommand or a program run with `nikaia run`; the existing Python
  in `scripts/` moves to Nikaia when it is next reworked. This is a principle,
  not a convenience: writing our own tools in the language shows where it has
  to be sharpened. What blocks or chafes on the way (a missing `std` entry, a
  syntax gap, a lowering defect) is a finding: fix it in the compiler - do not
  route around it, in the tool or in another language.
* **New tests are written in Nikaia only** (owner's rule, 2026-10-25, #821). A
  new test is a `.nika` program with its `.stdout` / `.stderr` beside it in
  `tests/compiler/` (or a package's own tests), run by `nikaia test`; no new
  `#[test]` and no new Rust test file. An existing Rust test is changed only
  to keep it passing, or moved to Nikaia (#821 steps 4-5). What cannot yet be
  said in Nikaia (a check on the Rust side, the C library) is a finding: fix
  or file it, and wait for the fix; do not write the Rust test as the way
  around.

## Who reads what else

* **The main session** (supervisor: starts agents, talks to the owner, raises
  versions, watches CI, runs decision rounds): `docs/supervisor.md`, and
  `docs/architect.md` for the architect and the hand-over.
* **Builders**: `.claude/agents/builder.md` (the cycle and the gate) and
  `docs/builder-facts.md` (where things live, traps, and the self-hosting
  rules: slices, syntax gaps of `tools/*.nika`, lowering).

## Measuring performance

* **Count, never time.** Instructions and mispredicted branches, from callgrind
  (`valgrind --tool=callgrind --branch-sim=yes`). On these shared machines the
  same binary moves by 1.4-1.9x from one day to the next; a count does not. No
  figure in seconds goes into a document.
* **Against tuned Rust**, `unsafe` allowed, never against a loop this project
  wrote to lose. Each Rust program is a binary of its own: two programs in one
  binary change what LLVM inlines (it cost `tuned` 40 instructions a row).
* **Release builds on both sides, the same settings**: `opt-level = 3`, link-time
  optimisation on both or on neither, and say which. Nikaia keeps its overflow
  checks; measure them off as a second row (`RUSTFLAGS="-C overflow-checks=off"`)
  rather than leaving them out.
* **The same output first**, byte for byte, or the counts mean nothing.
* **Length in tokens, not lines**: `nikaia run scripts/tokens.nika -- F...`, the
  same rule for both languages, whole files, `use` lines included.
* **A benchmark's README holds the current table and the method, not a diary.**
  Re-measuring replaces the table; how a number got there is the commit's and
  the issue's. Quote only that table, and before recommending a change from an
  attribution, check it was taken on today's code.
* **Understand a result before publishing it**: if a number moves, find out why
  (callgrind_annotate) before anything quotes it. `benches/brc/brc.sh` is the
  pattern: it builds, compares the outputs, counts and prints the table.

## Moving Rust into Nikaia (self-hosting, ADR-294, #125)

* **Fix, do not work around** (owner's direction): a compiler or lowering
  defect met on the way is fixed in the compiler, with a test, as its own
  commit, before or alongside the move. No workaround goes into our own code
  (owner's direction: no technical debt). A defect that cannot be fixed in
  this round is filed as a GitHub issue with a probe and a *To build* list
  (status label `ready`), never in a file in the repository, and the work that
  needs it waits for the fix.
* **The disk is shared and small.** Delete `target/` of a worktree you no
  longer use; keep `target/nikaia-project-tests` (the tests' cache, pruned by
  the compiler to `NIKAIA_CACHE_LIMIT`). Never run the full suite locally
  (about 45 minutes): CI runs it on every push to `develop`.
* The working rules for a move (slices, patterns, syntax gaps, lowered types,
  `lower-std`, the converter) are in `docs/builder-facts.md`.
