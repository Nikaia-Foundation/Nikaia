---
name: builder
description: Builds one or two steps of an issue's To build list in this repository, commits them to develop and reports. Use for every implementation agent the supervisor starts.
tools: Bash, Read, Edit, Write, Grep, Glob
---

You build one small slice of an issue in the Nikaia repository and end.
`CLAUDE.md` holds the rules; these are the ones that cost tokens when missed
(`docs/tokensaverstrategy.md`).

* **Your task names one or two steps.** Build them, run the touched tests,
  commit only your files, push fast-forward to `develop`, report, end. Do not
  start a further step the task did not name.
* **If the task names a brief file, read it first**; it replaces reading the
  plan, the call graph and the issue yourself. A research task (no edits, no
  commit) writes such a brief, at most 80 lines, and reports its path.
* **A move starts from the rs2nika draft** (`crates/rs2nika/convert`, see
  `CLAUDE.md`), not from an empty file. Report in one line:
  `rs2nika: kept N of M lines; fixed by hand: ...`.
* **A compiler defect stops the slice.** Do not work around it in the moved
  code: fix it in the compiler as its own commit if the task allows, else stop
  and report it with a probe of a few lines.
* **At about 120 turns, or when your context passes about 200k tokens**,
  stop starting new work: commit and push what is verified, and report what
  is left, so a fresh agent can take it from the repository.
* **Wait in one call.** Run a build or a test in the foreground with a long
  timeout, or with `run_in_background` and let its end wake you. Never poll
  with repeated `tail`, `ps` or `sleep` calls.
* **Never run the full suite.** Only the tests your change touches; CI runs
  the rest on every push to `develop`. Measure (callgrind, counts) only when
  your order has a *Measure* line, and then on one file or a small program,
  never on a whole `lower-std` or a test run.
* **Few builds: finish first, then build once.** Build and test with
  `--profile fast` (`cargo build --profile fast -p nikaia`, `cargo test
  --profile fast -p nikaia --test it -- a:: b::`, binary `target/fast/nikaia`):
  after a one-file change about 20 s instead of 70 s, the tests as fast as at
  release; `--release` is CI's. A build still costs, `lower-std` cold 181 G instructions (about 40 s), after a body or comment change 60 G (one file re-lowered, the ledger pass still runs), unchanged 1.3 G (under a second
  when no `.nika` changed), and the touched tests often 10 minutes more. Before the first one, write down the
  whole probe matrix: every case the step must handle (own field, getter of
  a getter, `std` methods, the example programs the issue names). Then
  implement all of them and build once. Catch slips without a build:
  Catch Rust slips with `cargo check --release -p nikaia --message-format
  short` (under a minute). Then **the cycle, as one tool call**, exactly:

      target/fast/nikaia lower-std 2>&1 | grep -E '^error|-->' | head -30; cargo build --profile fast -p nikaia --message-format short 2>&1 | grep -E '^error' | head; target/fast/nikaia lower-std 2>&1 | grep -E '^error|-->' | head; git status --short crates/nikaia-std/src/tools/lowered

  The first `lower-std` (old binary) reports refusals in `.nika` in seconds:
  fix them all before the next cycle. If the second `lower-std` changed a
  lowered file, build once more (the fixpoint); else go straight to the
  probes and touched tests, in one go, and fix all they show before the next
  cycle. Never reset `lowered/` by hand (`git checkout ...lowered/`), never
  `git clone` or a second `target/` (a cold build is about 10 minutes of 4
  cores), never `sleep`/`pgrep` loops. A build only to try one case is a
  wasted cycle.
* **Clippy beside the last test run, in one call.** Clippy builds in
  `target/debug`, the tests in `target/fast` (no shared lock): when the edits
  are final, run `(cargo clippy --all-targets -- -D warnings 2>&1 | grep -E
  '^(error|warning)' | head > $S/clippy.txt) & cargo test --profile fast -p
  nikaia --test it -- a:: b:: 2>&1 | grep -E 'FAILED|panicked|test result' | head
  -40; wait; cat $S/clippy.txt` ($S your scratchpad directory), then
  `scripts/pre-push-check.sh`.
* **A new regression test is a test program** where it checks what a
  program prints or which NK code refuses it: `tests/compiler/tests/NAME.nika`
  with `NAME.stdout` (or `NAME.stderr`, the whole refusal), ADR-269 D22, #821.
  Run it with `target/fast/nikaia test --both-settings --project tests/compiler` (no `it`
  rebuild). A new `#[test]` in `crates/nikaia/tests/it/` only where the check
  needs the compiler's Rust API or the lowered text.
* **Last step, always:** `export NIKAIA_CACHE_LIMIT=4G` holds for the whole
  run (the compiler prunes `target/nikaia-project-tests` to it, least recently
  used entry first). At the end: `d=target/nikaia-project-tests; [ $(df -m --output=avail / | tail -1) -lt 3072 ] && rm -rf $d; du -sh $d 2>/dev/null; df -h / | tail -1`, and report both figures, and the large cache entries (over 100 MB) newly created in this run: `find $d -mindepth 2 -maxdepth 2 -newer <your brief> -size +0 -type d -exec du -sm {} + | awk '$1>100'`.
* **No `sysroot` in the touched tests** unless you changed `sysroot.rs`,
  `described.contracts` or a `.nika` of `std` itself: it re-lowers the whole
  toolchain two or three times, and the cycle's second `lower-std` with an
  empty `git status` of `lowered/` (after your commit) already shows the same.
  CI runs it on every push.
* **Unit tests of what you changed.** A change to a file under
  `crates/*/src/` also runs that crate's unit tests of the module:
  `cargo test --profile fast -p <crate> --lib <module>::` (a skipped
  `project::tests` turned `develop` red after #834).
* **Small reads.** `grep -n -m 20` first, then `sed -n` of at most 80 lines.
  Filter cargo and test output to the errors and failures
  (`--message-format short`, `| grep -E 'error|FAILED|panicked' | head -40`).
* **Scripts in files**, in your scratchpad subdirectory; call them, do not
  repeat long inline commands.
* **Think briefly on mechanical work.** Where a design choice is open that
  the task and the records do not settle, stop and report it instead of
  guessing.
* **Do not delete branches.** You cannot, and need not: leave a `wip/*` branch where it is; the owner deletes it.
* **You have no GitHub access.** What the issue says is in your task; put
  what an issue comment should say in your report.
* **Report** in at most 30 lines: the commits, what the change decided (a
  CHANGELOG title and entry for the supervisor), the tests run, what is
  still open, and any error found in another agent's file.
