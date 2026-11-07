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
* A test of "a `keep` parameter keeps it" needs a callee that really keeps (stores or spawns); `fn hold(keep w: W) -> i64 { w.k }` only gets an NK1334 warning and proves nothing. (issue-1225)
* `scripts/pre-push.nika` is red on `develop` (self-hosted badge) whatever is staged; compare with an empty index and never reset to find out. (issue-1277)
* `NIKAIA_LOWER_CACHE_SALT=<fresh>` plus `lower-std --sysroot <copy>` gives an uncached reference run to `diff -r` against for byte-identity, without touching the shared cache. (issue-1277)
* A `let` over a field of a `ref` parameter (`let k = index.keys`) is NK2106 by design; read the field in place at each use. (issue-1279)
* Two `--lib` filters in one `cargo test` call (`emit:: sysroot::`) printed nothing for me; run one filter per call. (issue-1279)
* A rustc "temporary dropped while borrowed" or "mismatched types" on text usually comes from the text-tier pass (`text_tiers.nika`, ADR-282), not the keep code: lower the probe and check whether a `Vec[String]` became `Vec<&str>`. (issue-1287)
* `.to_string()` of text is emitted as the bare receiver (`text_as_is`, ADR-282 D8); copying text out of a view is `.clone()`, and a kept view of a temporary is NK1102 by design. (issue-1287)
* `nikaia test --filter X` prints no `test result` line when nothing matches, so a wait loop that counts result lines never ends. (issue-1287)
* `nikaia check` on a single `crates/nikaia-std/src/tools/*.nika` fails with NK1135 for imported types (`Span`); only the `lower-std` cycle checks a tool file. (issue-1289)
* `nikaia lower F.nika` in the scratchpad writes `F.rs` beside it; read `__nikaia_main` there before guessing at an emitter cause. (issue-1289)
* `scripts/pre-push.nika` checks only what is staged, so run it after `git add` of your files, or it is green on nothing. (issue-1290)
* `cargo test -p nikaia-std --lib emit_core_args::` matches no test, because lowered modules have no unit tests; skip it for a `tools/*.nika` change. (issue-1290)
* Typing a path expression `T::X` (variant, method value, `T::fields`, integer limits) is in `check_core_expr_path` in `tools/check_core_expr.nika`. (issue-1294)
* Nikaia's integer types are only `int`, `i32`, `i64`, `u8`, `u32` and `u64` (Part I 2.2); `integer_named` in `check_names.nika` tests for the fixed widths, and `int` is rewritten to `i64` only in emitted types (`emit_core_types.nika`), not in paths. (issue-1294)
* A lifetime or borrow probe needs a non-`Copy` element type (give the struct a `String` field), or the emitter copies the element and the defect does not show. (issue-1228)
* The cycle's first `lower-std` with `$S/old/nikaia` can fail on files nobody changed; the new binary's `lower-std` is the one that counts, and that failure goes in the report. (issue-1228)
* A `from` fix for text must also be probed with a struct type: the trait lowers only text `from` as a view, so a fix in `keeps.nika` breaks the struct case. (issue-1250)
* A text `+` is a `Binary` node that the emitter recognises by `facts.concatenations.contains(span.start as i64)`, using the operator span, not `node_key`. (issue-1251)

