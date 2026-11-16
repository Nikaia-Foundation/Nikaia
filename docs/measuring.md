# Measuring performance

Moved from `CLAUDE.md` (2026-11-01). For whoever measures, benchmarks or
quotes a figure.

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
* **A loss against another entry is split by a control before it is blamed**
  (#1847): write the Nikaia program's own algorithm in plain Rust (scratch, not
  committed) and count it too. Nikaia near the control: the gap is the other
  entry's algorithm, and the loss names the form Nikaia lacks for it (a
  `Found:`, e.g. a run-time byte shuffle, #1845). Nikaia well above the
  control: the cost is Nikaia's own, a defect to trace with
  callgrind_annotate. Quote the control's count beside the loss (#1614:
  fannkuch-redux 516 against a control of 499 a permutation, Rust #6 81.5).

