# Measuring performance

Moved from `CLAUDE.md` (2026-11-01). For whoever measures, benchmarks or
quotes a figure.

* **Count, never time.** Instructions and mispredicted branches, from callgrind
  (`valgrind --tool=callgrind --branch-sim=yes`). On these shared machines the
  same binary moves by 1.4-1.9x from one day to the next; a count does not. No
  figure in seconds goes into a document.
* **A Nikaia program runs under callgrind with `io-method = "blocking"`**
  (`NIKAIA_RUNTIME_CONFIG` naming a file that holds that line, Part III
  13.3b). valgrind 3.22 runs the executor's `io_uring_enter` wait as a call
  that does not block, under its one-thread lock, so a program that reads
  standard input waits for ever, silently, and a plain `timeout` does not
  end it (`timeout -s KILL` does; #1874: two hung runs before
  `--trace-syscalls` showed it). `benches/game/game.nika` sets it for every
  Nikaia row; a program counted by hand needs it too.
* **A criterion over a whole `lower-std` is counted on its parts** (ADR-361
  D15's "at most 5 % more instructions", #1980). Callgrind takes hours on a
  whole `lower-std`, which does not fit a run, and there is no `perf` here.
  Count instead, with a before and an after binary built from the same tree:
  `nikaia check --bounds FILE` under callgrind on the std file of the site the
  change targets, and on each file whose verdicts the change moves (a
  `lower-std --bounds` with each binary, diffed). Give the figure per file and
  the sum. A phase the change adds outside the walk is cut out of the whole
  run with `NIKAIA_LOWER_TIMES=1` and `callgrind_control -d` at its
  `lower-std~ NAME ended` line. The user time of the whole `lower-std`, two
  interleaved rounds (`docs/builder-facts.md`), only checks that the parts
  missed nothing and is never quoted. A report that names the files it
  counted has met the criterion as written; that needs no `Deviation:`.
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

* **A program build's rustc is counted alone, through a `RUSTC` shim** (#2016):
  never callgrind a whole `nikaia build` with `--trace-children=yes` (it
  counts Cargo, every dependency's rustc and the check, prove and lower
  passes; #2005's such run did not end in two hours). Count the top-level
  `nikaia build` with `--trace-children=no`, and rustc with a shim script set
  as `RUSTC`: Cargo hands it to the wrapper, which runs it in rustc's place;
  the shim runs the real `rustc` under `valgrind --tool=callgrind
  --trace-children=no` only when its arguments hold `--crate-name NAME` of
  the program's crate, and plainly otherwise (`-vV` too). About two minutes
  for the solver program. A store hit runs no rustc at all (ADR-021 D14), so
  a cold figure needs Rust that changed or the `rustc/` store moved aside.

* **A JVM caller is counted as a difference, with its JIT flags named**
  (#1337). A JVM's own start costs tens of millions of instructions, more
  than any crossing, so one run means nothing: count the same program twice
  under `valgrind --tool=callgrind --smc-check=all --trace-children=no`, once
  with a loop of N calls and once with N = 0, and divide the difference by N.
  The loop calls a no-op entry point (`pub extern fn nothing() sync {}`)
  through the generated binding, so the figure is the crossing alone. The JIT
  decides what is counted, so its flags are fixed and quoted with the figure:
  `-Xbatch -XX:-TieredCompilation -XX:+UseSerialGC -XX:CICompilerCount=1`
  (compiled before it runs, by C2 only, one thread), and a warm-up loop of
  the same fixed length (20 000 calls) runs before the counted one in both
  runs, so that both count the compiled crossing. The Python and Node
  bindings are counted the same way, with their interpreter's version named.
  The first figure (2026-11-17, OpenJDK 21.0.12 with `--enable-preview`, the
  `java.lang.foreign` that Java 22 makes final; the compiler at
  `--profile fast`): 822,513,063 Ir at N = 0 and 882,972,451 at N = 200,000,
  so about 302 instructions per crossing, the library's own entry wrapper
  included. A figure on Java 22 replaces it when it is counted.

* **Against another solver, count what is solved, then instructions; memory
  with its floor; time only beside** (#1338, ADR-270 D17).
  `nikaia run scripts/solver.nika -- compare` runs ours, z3 and cvc5 under one
  wall-clock limit per file and reports, per family, solved, wrong (against
  the file's status, and disagreements between the solvers) and PAR-2 (a
  file not solved counts twice the limit). PAR-2 there is in seconds, so it
  is a ranking, never a figure to quote: the figure is `--instructions`, the
  same run under callgrind, on the fixed sample `tests/solver/sample`. Peak
  memory is the child's `maxrss`, which Linux never reports below the
  runner's own high-water mark (ADR-368 D3): the header names that floor, and
  a value at the floor reads "at most the floor", not the solver's size.
  Memory under `--instructions` is valgrind's and is not quoted.
* **A wall-clock limit is judged by counts, and with a margin** (#2445).
  On the shared 4-core machine one file's wall time moves by about 0.7 s run
  to run (#2401: `40_40_11_5_unsat` took 9.7-10.7 s against the 10 s limit).
  So each step of the work is judged by callgrind counts (or the rod's case
  count, `run --stats --cases N`) on a fixed budget, never by seconds, and
  the limit itself is checked once at the end with `compare`. A file within
  10 % of the limit is reported as at the limit, with the range of three
  runs, so the supervisor sees how close it is.
* **Instructions per checker analysis** (#2187, item 3 of #1625).
  `nikaia run scripts/analysis_cost.nika` runs `target/fast/nikaia lower-std
  --findings` under callgrind, collecting only inside the check walk of each
  file (`check_walked_walk`), and prints one row per analysis: handles
  (ADR-312), outside data (ADR-396), bounds (ADR-385), proofs, views, lints,
  then *rest of the walk* and *the walk*. It takes hours (valgrind runs the
  whole command); `--read FILE` reads an output kept from an earlier run,
  `--sysroot DIR` checks a cut of the package laid out as the checkout.
  An analysis counts the calls into its functions (each `@inline(never)` in
  the `.nika` source, listed in the script's `analyses()`) from a function
  that is not its own, so nothing is counted twice. Handles and lints run
  woven into the type walk: their rows hold the functions they call, and
  what they record on the way (each read the `.clone()` lint notes) stays in
  *rest of the walk*, with the type walk itself. So read a row as a floor of
  that analysis's cost, and compare one row across two runs or two package
  sizes, not rows against each other. *Per line* divides by the lines of
  the package's `.nika` files (`--lines N` to say otherwise). A function
  added to an analysis needs `@inline(never)` and its name in `analyses()`,
  else its cost lands in *rest of the walk*.
