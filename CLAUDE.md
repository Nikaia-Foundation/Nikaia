# Working in this repository

The rules every session and agent needs. What only the main session does
(versions, CI, `promote`, issues, the driver, hooks) is in
`docs/supervisor.md`.

## Trunk-based: everyone works on `develop` (#437)

* **`main` is the default branch and the last tested state; `develop` is the
  trunk.** A session starts on `main` or a branch made from it, so it switches
  first: `git fetch origin develop && git checkout -B develop origin/develop`.
  Nobody pushes to `main`.
* **Before each commit, the cheap checks**: build, `cargo fmt`, the tests the
  change touches, `cargo clippy --all-targets -- -D warnings` (a lint in a
  lowered file is allowed on the `tools` module in
  `crates/nikaia-std/src/lib.rs`, not fixed in the file) and
  `nikaia run scripts/pre-push.nika` (fmt, ADR citations, badges, version, on
  what is staged, not the working tree). Never the full suite.
  Small commits, often.
* **Agents commit, the supervisor pushes.** A builder or QA agent
  commits locally; the driver or the main session tests and pushes
  (`docs/supervisor.md`). A session working alone pushes its own commits to
  `develop` after `git pull --rebase origin develop`; no pull requests.
* **A red `develop` is fixed forward**: whoever finds it red fixes it with the
  next commit or reverts the commit that broke it, at once.
* **Version raises and new error codes are the supervisor's.** An agent
  commits without touching the version lines, `CHANGELOG.md` or the badges,
  names in its report what the change decided, and takes a new `NK` code only
  from the block the supervisor reserved for it.
* **The commit that finishes an issue says `Fixes #N`**; that word closes it
  when `promote` brings it to `main`. Never write `fix #N`, `fixed #N` or
  `closes #N` (any tense, in prose too) for an issue the commit does not
  finish; write `the fix for #N`. `(#N)` and `Part of #N` close nothing.
* **Every open issue carries exactly one status label**: `ready`, `decision`,
  `blocked` or `note` (`docs/README.md`). Whoever files an issue or changes its
  state sets the label.

## Parallel agents share one checkout

* **Commit after every finished step, only your own files**: `git add
  <files>`, never `git commit -a` or `git add -A`; the tree also holds the
  other agents' unfinished files. No rebase and no separate worktree with
  `develop` checked out.
* **Nobody runs `git reset`, `git stash`, `git checkout .` or `git restore .`**
  in the shared checkout: it throws away the others' work. Restore only your
  own files, by name.
* **An error in another agent's file is not touched, it is escalated** to the
  supervisor (the session that started the agents).

## Written in Nikaia only (owner, 2026-10-25 and 2026-10-31)

* **New features: no new Rust.** A feature of the compiler, `std` or the tools
  is written as `.nika` (`crates/nikaia-std/src/tools/*.nika`, a package, a
  `tools/*.nika`); the `.rs` the lowering writes from it
  (`crates/nikaia-std/src/tools/lowered/`) is output. A step that cannot be
  done without writing Rust stops before the first line: `Result: blocked`,
  `Found:` the gap in Nikaia, `Needs: supervisor decision`. Briefs that still
  order Rust are read with this rule over them. A defect in existing Rust is
  fixed in Rust only where that code cannot be converted to Nikaia directly;
  where it can, the fix moves it into Nikaia.
* **Exception: the specification first (owner, 2026-11-03, #1214).** Views-3
  (#1213) and async-5 (#983) are built now, in Rust where the code they change
  is Rust (the parser's grammar block, the std runtime, the compiler's Rust
  parts); directly in Nikaia where that works without changing behaviour and
  converting at once. A brief of these lanes that orders Rust is meant so. The
  report lists the Rust it added, for #557 / #125.
* **Helper tools** (a migration aid, a checker, a generator) are a
  `tools/*.nika` behind a `nikaia` subcommand or run with `nikaia run`, not
  Python or shell; the Python in `scripts/` moves when it is next reworked.
  What blocks on the way is a finding: fix it in the compiler, do not route
  around it.
* **New tests** are a `.nika` program with its `.stdout` / `.stderr` beside it
  in `tests/compiler/` (or a package's tests), run by `nikaia test` (#821); no
  new `#[test]`, no new Rust test file. An existing Rust test is changed only
  to keep it passing, or moved to Nikaia. What cannot yet be said in Nikaia is
  a finding: fix or file it.

## Who reads what else

* **The main session** (supervisor: starts agents, talks to the owner, raises
  versions, watches CI, runs decision rounds, writes the briefs and keeps the lanes and the
  issues): `docs/supervisor.md`.
* **Builders**: `.claude/agents/builder.md` (the cycle and the gate) and
  `docs/builder-facts.md` (where things live, traps, and the self-hosting
  rules: slices, syntax gaps of `tools/*.nika`, lowering).
* **Whoever measures performance or quotes a figure**: `docs/measuring.md`
  (count with callgrind, never time; against tuned Rust; tokens, not lines).
* **Whoever writes for the website, `README.md` or `guide/`**:
  `docs/writing-guide.md` (lead with safety, no absolute claims, natural prose
  without bold lead-ins, nothing shown as runnable before it is built). Its
  last section is for whoever writes the specification: compact for models
  that read it whole, one rule in one place, one meaning per word.

## Moving Rust into Nikaia (self-hosting, ADR-294, #125)

* **A missing form stops the work, and is escalated** (owner, 2026-10-29: the
  P5 moves wrote get-clone-insert for a missing `zip`/`mem::take`,
  `scalars.take(16)` for a refused slice, a loop for a missing map `.iter()`;
  each is debt). Whoever meets something Nikaia, `std` or the toolchain lacks
  sorts it first, before writing a line around it:
  1. **Absent on purpose**: an ADR or a section of Part I-III refuses it.
     The ADR is the record; there is no separate list.
     Then build it the way that ADR says, and name the ADR in the report.
  2. **Not decided yet** (it did not matter when the part was written) or
     **missing because nobody thought of it**: stop. Commit nothing that writes
     around it; the report says `Result: blocked`, `Found:` the gap with a probe
     of a few lines, `Needs: supervisor decision`. The supervisor decides it
     (owner, 2026-10-31 and 2026-11-01): an obvious language change or
     extension, by the language principles (`docs/supervisor.md`, *Language
     decisions*), it approves itself, writes the ADR or specification text and files the
     build (`ready`); only a genuinely non-obvious question goes to the owner
     as a `decision` issue with a decision paper. Either way the work waits
     for the build of that decision.
  3. **A defect** (it is specified, or plainly meant, and does not work): fixed
     in the compiler, as below.

  No ADR to cite means it is not on purpose: in doubt, it is 2. "It can be
  written otherwise" is never the reason to go on, and a brief may not order a
  way around a gap of kind 2 or 3.
* **Fix, do not work around** (owner's direction): a compiler or lowering
  defect met on the way is fixed in the compiler, with a test, as its own
  commit, before or alongside the move. No workaround goes into our own code
  (owner's direction: no technical debt). A defect that cannot be fixed in
  this round is filed as a GitHub issue with a probe and a *To build* list
  (status label `ready`), never in a file in the repository, and the work that
  needs it waits for the fix. Who finds a defect that blocks the work fixes
  it at once, in any lane (owner, 2026-11-01); only what does not fit the
  round goes to the bug lane (`docs/supervisor.md`, *Lanes*).
* **The disk is shared and small.** Delete `target/` of a worktree you no
  longer use; keep `target/nikaia-project-tests` (the tests' cache, pruned by
  the compiler to `NIKAIA_CACHE_LIMIT`). Never run the full suite locally by hand
  (about 45 minutes; a driver runs it per batch): CI runs it on every push to `develop`.
* The working rules for a move (slices, patterns, syntax gaps, lowered types,
  `lower-std`) are in `docs/builder-facts.md`.
