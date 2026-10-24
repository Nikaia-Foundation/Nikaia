# Long jobs: who runs them, how often, how long

The owner's question (2026-10-24): measuring and verifying take too long, and
every builder runs the long jobs itself. Should one agent, or a shared
script or queue, run them once for everybody? Companion to
[`modular-compiler.md`](modular-compiler.md), which covers making the
build itself smaller.

## The jobs today (develop ccd7bb47, 4 cores, shared machine)

Durations are observed wall clock of build tools, not program measurements
(`CLAUDE.md`: they move 1.4-1.9x by day); counts where there are any.

| Job | Observed | Where the time goes | Who needs it |
| :--- | ---: | :--- | :--- |
| `cargo build --profile fast -p nikaia` | ~20 s after a Rust change; longer after a `.nika` change | one module per `.nika` since 8dd21e75 (was one 7.4 MB unit) | every step |
| `nikaia lower-std` | ~40 s; under 1 s when the whole-package cache hits | 213 files checked and lowered; the cache key is package plus binary hash, so an edit always misses | every `.nika` step |
| Touched tests (`--test a --test b`) | ~10 min | 267 test binaries, each relinked (16-20 MB each) | every step |
| Full suite, locally | ~45 min | the same 267 binaries, all run | nobody locally |
| Full suite, CI (`bootstrap_compiler`) | 8 min tests + 1.5 min sweep, ~17 min the whole run | off this machine; arm64 runs in parallel | every push, automatically |
| Oracle (`scripts/c6/oracle.py`) | ~35 s | 142 files lowered by two binaries | moves of `check`/`emit` |
| callgrind of one small file (`lower overlaps.nika`) | seconds; 42.6 M instructions | valgrind runs the program 50-100x slower | tasks whose point is a number |
| callgrind of the whole `lower-std` | over 25 min and still running (the #667 run of today) | the same slowdown on ~40 s of work | nobody |

This machine has **no hardware counters** (`/sys/bus/event_source/devices`
has no `cpu`; no `perf`), so callgrind is the only instruction counter, and
it is slow by construction.

## Findings

* **The verify service exists: it is CI.** Every push to `develop` runs the
  full suite on GitHub's machines in about 17 minutes, off the builder's
  critical path and off this machine's 4 cores. A local agent or queue
  running the same suite would compete with the builder for the same cores,
  the same `target/` lock and the same small disk.
* **What is missing is a reader of CI.** `develop` has been red since
  9f090392 (U-d2, 10:18 UTC: badge check, arm64 tests) and nobody's next order
  was the repair. `CLAUDE.md` says a red `develop` is fixed forward; no role
  had the duty to look.
* **In stage 1 there is nothing to batch.** One builder at a time hands in one
  change at a time. A measuring agent would cost its own context per turn
  and save no job. It may pay in stage 3 (many builders, many changes); not
  decided here.
* **Most measuring is better done by counting work, not instructions.** A
  cache is judged by *files re-lowered of 213*, a module split by *codegen
  units and bytes re-optimised* (method in `modular-compiler.md`). These are
  exact, deterministic and cost nothing. Instructions are for the program's
  own speed, and then on the smallest input that shows the effect.

## Recommendation

1. **No measuring agent, no queue now.** CI is the verifier; the main session
   reads it. After each builder's push it runs
   `gh run watch <id> --exit-status` with `run_in_background` (one call, it
   wakes on the end; no polling) while the next builder starts; a red run
   makes the repair the next order (`architect.md`, *The loop*).
2. **Builders never run the full suite**, only the touched tests; CI runs the
   rest. Already the rule; now also in the work order's *Done when*.
3. **Measuring only in a task whose point is a number**, named in the work
   order's *Measure* line (default: none). Prefer a work count the tool
   prints; callgrind only on one file or one small program, never on a whole
   `lower-std` or a test run.
4. **Make the jobs shorter**, in this order: #667 per-file cache keys (an
   edit re-lowers one file and its readers), #667 step 4 (one Rust module per
   `.nika`: one small unit re-optimised), one test binary (#793 step 3: one
   link instead of 267), then nextest in CI if CI's 17 minutes become the
   limit. CI's own run and its caches: [`ci-cache.md`](ci-cache.md).
5. **Revisit** a shared measuring tool (a `tools/*.nika` behind `nikaia`,
   per `CLAUDE.md`) only in stage 2 or 3, if two builders' jobs queue on the
   machine; it would serialise their `cargo` and callgrind runs and hand back
   counts per commit.
