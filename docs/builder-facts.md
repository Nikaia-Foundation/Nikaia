# Builder facts

Fixed facts a builder would otherwise look up itself. `scripts/agent.nika`
puts *Core* into every builder's prompt, and after it the sections the brief
names (`Facts: Probes, Grammar`; a `##` heading brings its `###` sections
along), taken from `docs/builder-facts-lookup.md`. That file is not read
whole: `nikaia run scripts/syntax.nika -- KEYWORD` prints its lines on a topic
together with the specification section and an example test.

## Core

### Where things live

| What | Where |
| :--- | :--- |
| Checker, moved parts | `crates/nikaia-std/src/tools/check_*.nika`; Rust adapter in `crates/nikaia/src/check/` |
| Emitter | `crates/nikaia/src/emit/mod.rs`, `emit/host.rs`; moved parts `tools/emit_*.nika` |
| `std` methods by receiver | `std_receivers` in `tools/check_core_method.nika` |
| Generated Rust of a tools file | `tools/lowered/<file>.rs`, index `tools/lowered.rs` (never edit, `lower-std` writes them) |
| Program tests (new tests) | `tests/compiler/tests/NAME.nika` + `.stdout` / `.stderr` |
| Rust tests by NK code | `grep -rln NK1234 crates/nikaia/tests/it/` |
| Specification | `docs/specification/10-nikaia-light.md`, `20-nikaia-advance.md`, `30-nikaia-tooling.md` (code table) |
| A name, Rust or Nikaia | `nikaia run scripts/where.nika -- NAME...`: definitions with file:line and the files that call them; for an `NKnnnn` the tests and the spec row (a field is not found: grep it); `-- --outline FILE` lists each item of a file with its line range |
| A snippet through the built compiler | `nikaia run scripts/probe.nika -- 'let x = 1\nprintln(x)'` (`--check`, `--file F.nika`): runs it with `target/fast/nikaia`, prints at most 60 lines and `exit N`; a probe for a report is the snippet and these lines |
| How a form is written | `nikaia run scripts/syntax.nika -- KEYWORD`: the specification section, an example test and the facts lines on it (e.g. `slice`, `view`, `throws`) |

### Every run

* **Copy `target/fast/nikaia` to your scratchpad before your first build**: it
  is the before binary for the fails-before run, and the binary long test runs
  use, since another agent may rebuild `target/fast/nikaia` mid-run (P5a, j5).
  Keep the copy in its own directory and name it exactly `nikaia`
  (`$S/old/nikaia`): script tests check the basename and otherwise fall back
  to `target/fast/nikaia` (b42). `target/release/nikaia` is older than the
  views syntax and cannot serve.
* **The checkout is shared.** A file another agent has modified: stage only
  your hunks (`git diff FILE > $S/p.diff`, delete theirs from the copy, `git
  apply --cached`, check with `git diff --cached FILE`; `git add -p` is
  refused here). A build that shows none of your edits may have run during a
  driver rebase: check `git status` and your file, then rebuild; never touch
  others' `UU` files, name them under `Needs:` (BT, b1043).
* **Long runs go in the background** with `run_in_background` and `timeout:
  3600000`, and you wait for the wake-up, never a `sleep` or `pgrep` loop. A
  `lower-std` and build cycle takes 10-15 minutes: find every change it needs
  with probes on the old binary first; one cycle less is the largest saving a
  run can make (a9, v6, v7). Put a `&` on one command, never on a chain (P4b).
* **Probes live in the scratchpad**, outside any project: `nikaia check FILE`
  inside `tests/compiler/tests/` reads the sibling files (prover lane).
* **Never commit `nikaia.lock`**; it is git-ignored and changes with each build.
* **Never run `nikaia fmt` on `crates/nikaia-std/src/tools/*.nika`**: it
  reindents the whole file (b1012b). Format by hand there.
* **`pre-push` fails with *out of date: self-hosted.svg*** when a `.nika`
  changed the self-hosting share: regenerate it from what you commit, `git
  checkout-index -a --prefix=target/badge/`, run `scripts/badges.py` in that
  copy, copy its `assets/badges/self-hosted.svg` back and list it under
  `Files:`.
* **Builders have no GitHub**: the prompt holds the issues the brief names; a
  missing one goes under `Deviation`. A pre-flight names a commit by subject
  (`git log --grep`), never by hash: rebases change hashes (p9).
* A script test that compares a driver's `--dry-run` text fails when `scripts/*.nika` wording changes; bless it from the project dir (`nikaia test --filter NAME --bless` in `tests/compiler`) and review the diff, since `nikaia run FILE` refuses a non-entry file. (qa-task)
* Two `qa-input.txt` files exist (`docs/briefs/` and `docs/briefs/views/`); read the one in the lane directory your task names. (qa-task)
* Before putting back a "workaround" an issue names, check `git log -S` on that line: a later commit may have removed it on purpose. (issue-1125)
* The tether's keep plan (`tools/buffers.nika`, `is_a_buffer` in `tether.nika`) treats only text and bytes as buffers, so a view of a list or struct local handed back is NK2304 (ADR-283 D9), never NK2303, which is not a refusal. (issue-1155)
* `scripts/where.nika -- NKnnnn` can run past the 120 s tool timeout; grep the `NKnnnn` in `crates/` and the spec row instead. (issue-1155)
* Running a copied test file with `nikaia run` inside `tests/compiler/tests/` is refused as not the entry point; run the fails-before check on the scratchpad copy. (issue-1266)
* After a `.nika` change, the cycle's first `lower-std` must run with the before binary (`$S/old/nikaia`); `target/fast/nikaia` still holds the logic of the previous attempt. (issue-1209)
* An issue's probe may pass on `develop` because a later fix masks it; vary the shape (index, `.iter()`, a second call site of another kind) before judging the defect gone. (issue-1210)
* A `git mv` staged before `commit.nika` makes it refuse (staged and not listed, or `git add` of a missing path); delete files unstaged and list both paths. (issue-1218)
* Before planning, `git log --grep=<issue>`: earlier lane runs may have finished part of the issue under `Part of #N`. (issue-1224)

