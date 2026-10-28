# The bootstrap week: 2026-10-28 to 2026-11-04

The owner's decision (2026-10-28, binding, a condition of the budget): by
**2026-11-04** self-hosting is complete, as a **complete bootstrap**. At most
two parallel move lanes, one lane for lowering defects, one for compiler bugs;
parallelism that lowers quality is counterproductive, and token cost counts.
This file is the plan; the architect's *State* (`docs/architect.md`) and the
plan rows (`docs/selfhosting-plan.md`) stay where they are and point here.

**Owner, 2026-10-28 (answers D-2):** `crates/orchestrator` and `crates/nikaia-std`
are in scope too ("the sponsors want to see this"); the deadline stays
2026-11-04. Section 1 says what that can mean, from the code and the
specification; section 1a gives the figures, plainly. D-1 is still open:
section 2 plans both variants.

Counts are code lines of `scripts/self_hosting.py` (no comments, no blanks).
At 2026-10-28 11:30 UTC: **21,035** Rust lines in `crates/nikaia/src`, 59
files, 80.6 % self-hosted; 1,321 of the Rust lines are `#[cfg(test)]` modules.
Pace of the last 24 hours: about 2,200 lines net with one move lane.
At 12:15 UTC: **20,954** (P4a running). Code lines of the hand-written Rust
outside the compiler, counted the same way (scratch script over
`self_hosting.py`'s `code_lines`; generated `lowered/` files excluded):
`crates/nikaia-std/src` **12,088** plus 4,277 in `#[cfg(test)]` modules;
`crates/orchestrator/src` **1,228** plus 812; `crates/unsafe/*` 2,515 plus 965.

## 1. Definition of done

**Stage 2 is reached, and held by CI, when the `bootstrap_compiler` job of
`.github/workflows/nikaia.yml` passes a step named `Bootstrap (ADR-294 D5,
Stage 2)` on a `develop` commit that `promote` brings to `main`.** That step
runs, after `Build the workspace`:

1. `NIKAIA_CACHE_DIR=$(mktemp -d) target/release/nikaia lower-std` (cold: no
   cached file may stand in for a lowering), then
   `git diff --exit-code -- crates/nikaia-std/src/tools/lowered crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/runtime`.
   The binary was built from the committed lowered Rust plus the adapters, so
   an empty diff is the fixpoint ADR-294 D5 names: the lowered compiler lowers
   its own sources to byte-identical Rust. (A second generation needs no run:
   identical sources build the identical compiler.) The existing test
   `the_committed_rust_is_what_this_compiler_lowers`
   (`crates/nikaia/tests/it/sysroot.rs`) already compares the same bytes for
   the 80 % that is Nikaia today; the step adds the cold cache and the name.
2. `target/release/nikaia run scripts/bootstrap.nika`, a new checker in
   Nikaia (CLAUDE.md: tools in Nikaia): it reads `scripts/bootstrap-adapters.txt`
   (the table below, one `file function max_lines reason` row each) and fails
   if any hand-written `.rs` under `crates/nikaia/src`,
   `crates/orchestrator/src` or `crates/nikaia-std/src` (not `lowered/`, not
   a `.rs` lowered from a `.nika` beside it) holds a function or a type not on
   the list, or a file holds more code lines than its row allows. The `std`
   and orchestrator rows are the kernel tables below; for K-std-1 and K-std-2
   a row is the whole file at its line count today, so it can only shrink. Module
   declarations (`mod`, `pub use nikaia_std::tools::...`) are free.

So Stage 2 = Stage 1 (only the adapters below are Rust) + that step green.
Stage 1 alone is what the second check proves; Stage 2 is the first.

**Scope** (owner, 2026-10-28): `crates/nikaia/src` (what ADR-294 D4
measures), `crates/orchestrator/src` and `crates/nikaia-std/src`'s
hand-written Rust, each down to the fixed lists below. Not in scope:
`crates/unsafe/*` (ADR-218 D1, D4: every `unsafe` topic is a Rust crate of its
own, and Nikaia writes `unsafe` only around a foreign call and `proven-index`),
`crates/rs2nika` (a development tool), and the Rust tests under
`crates/nikaia/tests` (#821 goes on, not in this week). The `#[cfg(test)]`
modules inside the three crates are in scope: each leaves with the code it
tests, as a `tests/compiler` program where the moved code is not already
covered, else deleted; a kernel's own Rust tests stay with the kernel (they
test Rust from Rust, some under Miri).

**The adapters allowed to remain** (the fixed list; at most about 2,800 lines
in all). Each is Rust only because a crate or an OS call stands behind it;
everything it decides is a call into Nikaia.

| File | What may stay | At most | Reason |
| :--- | :--- | ---: | :--- |
| `main.rs` | `Cli`, `Command`, `CacheAction`, `Verb` (the `clap` derives), `main` turning them into one call of a Nikaia entry per subcommand, `std::process::exit` | 200 | `clap` |
| `parser.rs` (was `parser/mod.rs`) | the entry points `parse_to_ast`, `parse_expression_from` calling the lowered grammar; the conversion of a `winnow` error into the Nikaia `ParseError`; `Parsed`'s caches only if a tools file cannot hold a lock | 200 | `winnow`, `winnow-grammar` |
| `ast/mod.rs` | `int_value`, `int_literal` | 70 | `i128` (ADR-294 D6) |
| `manifest.rs` | `Manifest::read`/`parse`: `toml::Value` into the Nikaia manifest view; `rooted` (from `project.rs`) | 150 | `toml` |
| `sysroot.rs` | `Sysroot::resolve`, `COMPILER_IDENTITY`, `rlib_cache` (`env!`, `orchestrator::cache`), `rustc_cpus` (a `rustc` process), `in_parallel` (scoped threads with a 256 MB stack), `sha256_hex`, `Phases` | 250 | `env!`, `sha2`, `std::thread`, `orchestrator` |
| `comptime_run.rs`, `grammar_run.rs`, `build_time.rs`, `run_fast.rs` | compiling and running a generated program (`orchestrator`, `rustc`/`cargo` processes with environment and working directory), the `AHEAD` `thread_local` | 350 together | `orchestrator`, processes `std::process` cannot start yet |
| `project.rs` | the test binaries' spawns with stdin, working directory and exit status, `Sandbox`, `current_exe` | 250 | processes, temporary directories |
| `contracts/mod.rs` | the trait impls `LedgerOps`, `SignatureOps`, `VariantOps`, `SyncOps` (one-line adapters), `std_ledger` (`OnceLock`), `toolchain` (`env!`) | 150 | Rust traits over `nikaia_std` types, `env!` |
| `emit/mod.rs` | `Build` with `host_target`/`default_target`; the `rayon` fan-out in `with_contracts` | 150 | `impl Default` of a `nikaia_std` type (E12-4), `env!`, `rayon` |
| `diagnostics/mod.rs` | the terminal and colour decision, the `serde_json` writer | 80 | `serde_json`, the terminal |
| `describe.rs`, `cexport.rs`, `libraries.rs`, `assets.rs` | the `cargo`/`node-gyp`/`rustc` invocations, `sha2` digests | 150 together | processes, `sha2` |
| any file | `anyhow` context at a crate boundary, `catch_unwind` | in the rows above | `anyhow` |

A process the Nikaia `std::process` can start is not an adapter: it moves.
`process::run` today takes `dir:`, `env:`, `env_remove:`, `env_clear:` and
`within:` and captures both outputs, with stdin closed (`std.contracts`,
`[fn."process::run"]`). What it cannot do yet: hand the child the terminal
(inherited stdout/stderr, as `cargo build` and a test binary need so the user
sees progress as it happens), feed a stdin, or capture one stream and pass the
other through. Only those spawns are adapters. A missing `std` entry met
on the way is filed (`ready`) and its row's adapter is named in the issue, so
the list shrinks after the deadline rather than growing in it.

### `nikaia-std`: what can be Nikaia, and the kernel that stays Rust

**The rule is already decided.** ADR-014 D1: *what a `.nika` file in `std`
may contain is exactly what Stage 0 can lower*, and `std` already holds
Nikaia (`text.nika`, `supervisor.nika`, `runtime/http1.nika`, #723).
`nikaia-std` has no `unsafe` (`#![forbid(unsafe_code)]`, ADR-218 D4), so
`unsafe` is not the line: the line is what Nikaia cannot say, and the
specification names three reasons.

* **The lowering writes into it.** The emitter's output is calls into these
  types (`nikaia_std::index::get`, `either(..)`, `Bytes`, `Map`): Nikaia written
  for them would lower into calls of themselves. They are also built of what
  Part I 6.5 excludes from the language ("Nikaia source contains no lifetime
  annotations. Ever."), of impls of Rust's own traits (`Deref`, `Index`, `Hash`,
  `Display`, `Iterator`, `Future`, `Drop`: 280 `impl … for` in these files) and
  of generic bounds. ADR-282 D14 (`EitherText` is the compiler's per-position
  representation of `String`, over `Cow<str>`), ADR-319 D1 (`fs::Path` is the
  platform's own bytes, `OsStr`), ADR-294 D6 (`Span`'s `usize` doors stand in
  `std`), ADR-296 D42 (a map over `hashbrown`), ADR-373, ADR-375, ADR-377.
* **The concurrency below the language.** Part II 12.2: what `Locked[T]` is in
  the machine (a mutex, an atomic word or a cell) is the compiler's choice per
  value; a task is a Rust `Future` with a `Waker`; the runtime is OS threads,
  `epoll`/`io_uring` (through `polled`, `file-ring`), condition variables. No
  Nikaia source can name an atomic, a waker or a thread's stack.
* **Adapters to the OS and to crates**, the same kind as the compiler's
  table: the call stays Rust, what it decides moves.

| Kernel | Files (code lines, without tests) | Stays | Reason |
| :--- | :--- | ---: | :--- |
| K-std-1, the lowering's target | `index` 581, `map` 650, `seq` 476, `error` 457, `lib.rs` 410 (module tree, prelude), `simd` 384, `byteset` 380, `bytes` 245, `num` 223, `either_text` 206, `hash` 178, `range` 137, `build_time` 128, `grammar` 123, `abort` 90, `fixed` 69, `node_id` 60, `count` 54, `boxed` 52, `target` 49, `func` 43, `text_float` 40, `concat` 37, `foreign` 30, `list` 16, `arm` 15, `tether` 1 | 5,134 | the emitter writes into them; lifetimes, Rust trait impls (Part I 6.5, ADR-282 D14, ADR-294 D6) |
| K-std-2, concurrency and runtime | `task` 1,187, `rt/mod` 739, `cleanup` 405, `rt/exec` 315, `lock` 301, `par` 284, `rt/pool` 272, `rt/readiness` 240, `rt/worker` 177, `channel` 106, `rt/library` 61, `rt/timer` 20, `rt/exit` 12 | 4,119 | Part II 12.2; `Future`/`Waker`, atomics, threads |
| K-std-3, OS and crate adapters (after their logic moves) | `fs` ~444 of 644 (mapping via `checked-text`, `OsStr`, the `Root` calls), `process` ~214 of 364 (the spawn), `cli` ~113 of 363 (`args_os`), `supervision` ~182 of 332 (`dyn` over tasks), `io` ~159 of 199, `net` 145, `regex` 116 (`regex-automata`), `http1` ~82 of 142 (the socket loop), `search` ~74 of 104 (`memchr`, ADR-320 D6), `time` ~81 of 101, `rt/config` ~36 of 136, `text_normal` 33, `html` ~10 of 70, `graphemes` 9 | ~1,700 | OS calls, crates; ADR-319 D1 |
| already Nikaia | `text.rs` 60, `supervisor.rs` 17 (lowered) | 0 | - |

So **of `std`'s 12,088 hand-written lines about 1,060 can move** (±400: the
per-file split above is read off the code, not converted; std-1 measures it),
and **about 10,950 stay Rust by the language's design**, not for want of time.
The share that moves: `cli` argument parsing ~250, `fs` path and `Root` checks
~200, `process` env checks and `PATH` search (`invalid_entries`, `merged`,
`found_on_path`) ~150, `supervision` restart policy ~150, `rt/config` the file's
parse ~100, `html` escaping ~60, `http1` glue ~60, `io` ~40, `search` ~30,
`time` ~20.

**Where `std`'s moved Nikaia goes, and what the ledger allows.**
1. Into `src/runtime/<m>.nika`, the unpublished package (`http1.nika`'s
   pattern): the Rust surface function stays as a one-line forward, **its
   hand-written `std.contracts` entry unchanged**. A program's checker sees
   nothing new.
2. **Not** as a published `src/<m>.nika` (`text.nika`'s pattern) this week:
   that turns the module's entries from written to *inferred*
   (`crates/nikaia/tests/contracts.rs` regenerates them), and an inferred
   `sync`, `touches`, `keeps` or `throws` that differs from the written
   promise changes what programs may do: one review per entry, after 11-04.
3. The runtime package is lowered into every program's `std`, at both
   `user_parallelism` settings (Part III, *`std` ships as sources*), and
   `lower-std` lowers it in the same run as the tools (`sysroot.rs:1139`). So
   a runtime `.nika` must lower at both settings, and section 3's rule for
   generated files covers `crates/nikaia-std/src/runtime/lowered*` (already in
   the Stage 2 diff).
4. Runtime and tools are separate packages: neither calls the other, and a
   runtime body calls only what `std.contracts` publishes. Whether the runtime
   package can read a `described.contracts` for an unpublished kernel helper,
   as the tools do (ADR-294 D9.3), is **not verified**: std-1 checks it first;
   if not, that is a finding, filed and fixed, not routed around.
5. A body the lowering itself writes a call to (K-std-1) never moves: it
   would lower into itself.

A move of `std`'s Rust changes `crates/`, so it raises the version like a
compiler move; that is the supervisor's.

### `crates/orchestrator`: what can be Nikaia now

1,228 code lines (812 more in tests). Its Nikaia goes into the tools package
(`tools/orchestrator_*.nika`, the prefix rule); `orchestrator` gains a
dependency on `nikaia-std` (no cycle: `nikaia-std` does not depend on it) and
its kept Rust calls `nikaia_std::tools::...`.

| Moves (~790) | Stays (~440) | Reason it stays |
| :--- | :--- | :--- |
| `cache.rs`: `Choices`, `Layout` and its paths, `limit`, `parse_size`, `show_size`, `prune`'s choice of entries, `entries`, `size_of`, `Artifacts`, `Store`'s paths | `sha256_hex`, `KeyBuilder` (~60) | `sha2` |
| `project.rs`: `CargoProject`, `Workspace` (the `Cargo.toml` text), `write_if_changed`, `toolchain_is_new_enough`, `toolchain_can_build_for`, `rustc_version`, `version_parts` (captured `rustc` runs with `dir:`), `Invocation`'s argument lists, `record_extra_dependencies`, `escape_make` | `Lockfile::load`/`save` (~80) | `toml`/`serde` |
| | `mod lock`, `HELD`, `hold`, `open_lock`, `first_link`, `cargo_holds` (~120) | `flock` (`rustix`), inode numbers, a process-wide hold |
| | `Cargo::run`, `Cargo::json`, `Invocation`'s spawn (~90) | inherited stdio: not in `std::process` yet (file it, `ready`) |
| | `resolved_versions` (~50) | `serde_json` over `cargo metadata` |
| | `lib.rs` (36: `clap` `Cli`, `LanguageFrontend`) | `clap`; delete if no caller is left |

### What remains Rust on 11-04, in all

Compiler adapters <= 2,800; orchestrator ~440; `std` kernel ~10,950;
`crates/unsafe` 2,515 (out of scope by ADR-218). About **16,700** lines of
Rust in the shipped toolchain against about 105,000 of Nikaia (87,400 today
plus the moves): about 86 %. Every one of those Rust lines is on a list with
its reason, and `scripts/bootstrap.nika` holds CI to the lists.

## 1a. Feasibility, in figures

Today's measured pace (one move lane, the driver): 7 moves from 03:39 to
10:08 UTC removed 1,135 net lines (P2 -318, M1 -172, S2 -119, P3a -214, M2
-129, `shown_path` -9, P3b -174); 24 hours: **-2,387**. A move takes about
56 minutes of wall time, of which the builder runs 27.5 (P3b) to 33.6 (M1);
the rest is the ESCALATE stop, the converter repair (30-50 minutes) and the
restart. Inside a builder 77 % (P3b) to 93 % (M1) of the wall time is tools:
`cargo build`, `lower-std`, the tests, clippy. The machine has **4 cores**.

Time left: from 10-28 12:15 to 11-04 16:00 (the last push that CI, about 45
minutes, and `promote` can still carry that day): **7.15 days**.

| Scope | To move | Lines a day | Against 2,387 measured |
| :--- | ---: | ---: | ---: |
| S-A: the compiler to its adapters | 18,150 | 2,540 | 1.06x |
| **S-B: the owner's, with the kernel lists above** | **20,000** (18,150 + 790 + 1,060) | **2,800** | **1.17x** |
| S-C: every Rust line of the three crates and their tests | 36,600 | 5,100 | 2.1x, and ~11,400 of it cannot be Nikaia (section 1) |

**Is S-B reachable with four lanes? Not without the levers of section 3a, and
not with margin even with them.** The reasons, plainly:

1. **The CPU, not the lanes, is the ceiling.** 77-93 % of a builder's time is
   compiling and testing on 4 cores. A second move lane that builds at the
   same time stretches both lanes' tool phases: estimated 1.3x the single
   lane's lines, not 2x (an estimate; nothing has measured two building lanes
   here). The two fix lanes take turns in C3 and add a third builder. So four
   lanes at today's way of working give about **3,100 lines a day on easy
   slices**: 1.1x what S-B needs, and the late slices are not easy.
2. **The pace so far was measured on leaves** (`project.rs`, `modules.rs`).
   The parser (2,280 lines behind #557 steps 1-3) and the `check/*`, `emit/*`
   cores (recursive, not yet mapped) come last, when the margin is gone.
3. **Half of today's wall time is the stop after each move** (ESCALATE,
   repair, restart), not building. That is the one large reserve, and it is a
   process change (lever L1), not more lanes.

With levers L1-L4 live by Thursday morning the single lane should reach
3,500-4,000 lines a day on leaves, two lanes about 4,500 (estimated; L5 makes
it measurable). Then S-B is reachable **if** #557 step 3 is green by Friday
evening and the `check`/`emit` cores are at most 2,000 lines of moves.
Without the levers, at 2,387 a day: S-A ends about 11-05 03:00 and S-B about
11-05 22:00.

**The strictest scope reachable by 11-04 with confidence** is S-A plus the
orchestrator (18,940 lines, 2,650 a day) plus the `std` kernel list fixed and
held by CI, with std-1 (`cli`, `html`, about 310 lines) as the first `std`
move. `std`'s other ~750 movable lines then follow by 11-06 at one lane. S-B
in full on 11-04 needs the levers, D-1 (variant A) and the parser on time;
the Saturday checkpoint decides (section 5, D-6).

## 1b. Notes for the sponsors (what "in Nikaia" honestly means)

The compiler is Nikaia except a fixed list of adapters to Rust crates and to
the OS. `std` is Nikaia where its logic is, and Rust where it is the machine
under the language: the types the compiler lowers to, the scheduler, the
locks, the system calls. Self-hosted toolchains keep such a layer: GHC's
runtime system is C, Go's runtime has assembly, Rust's `core` has compiler
intrinsics. What makes it a bootstrap is the fixpoint (section 1) and that
the layer is small, listed with a reason per row, and checked by CI.

## 2. Day targets

Order inside each lane: leaves of the Rust call graph first. Today most of
the remaining Rust is either a **driver** (the run of analyses, the project
loop, `main`) or a **crossing**: a thin Rust function that turns `&Parsed` and
`Ledger` into the Nikaia call (`contracts/order.rs` is 71 lines around
`order.nika`'s 790). A crossing goes when its last Rust caller moves, so each
slice takes a driver together with the crossings only it calls.

Variant A (D-1 granted: three checkouts). Column *orch + std* is what is left
of the orchestrator's ~790 and `std`'s ~1,060 movable lines.

| End of | Compiler Rust | orch + std | Lane 1 (main checkout, the driver) | Lane 2 (worktree) | Fix checkout C3 (lowering, then compiler; std moves when its queue is empty) |
| :--- | ---: | ---: | :--- | :--- | :--- |
| Wed 10-28 | 20,100 | 1,850 | P4a, P4b (`project.rs`, -660) | L2-1 the crossing `dsl.rs` deleted, `bounds_report.rs`'s walk into Nikaia (-290) | #916 (absorbs #913, #914); #557 steps 1-2 (`span(p)`, `here`) |
| Thu 10-29 | 17,400 | 1,850 | levers L1-L2 (section 3a) first, between two briefs; P1 settings, K1+K2 comptime tree, P5 `project.rs` build drivers (-1,400) | L2-2 the `contracts/{order,locks,sharing,maps,throws,touch}.rs` crossings, L2-3 `contracts/{trust,sync,tether,keep,keeps}.rs` (-1,300) | #557 step 3 (`same_line`); #910, #907, #908, #915 (text views) |
| Fri 10-30 | 14,700 | 1,850 | S1 `sysroot` package, M3 `modules.rs` collect, `manifest.rs` (-1,350) | C1 `contracts/mod.rs` + `send.rs` + `ty.rs` (-1,350); parser chunk 1 written beside | #911, #917, #918; `scripts/bootstrap.nika` + `scripts/bootstrap-adapters.txt` with all three crates' rows, the CI step with `continue-on-error`; converter repairs as they come |
| **Sat 10-31** (checkpoint) | **12,000** | **1,540** | K3 `comptime_run.rs` rest, `build_time.rs`, `grammar_run.rs` (-1,300) | `proofs.rs`, `prove.rs`, `bounds.rs`, `bounds_report.rs`, `fold*.rs`, `dsl.rs` (-1,400); parser chunks 2-3 | #909, #898, #900; **std-1** `cli.rs` parsing, `html.rs` escaping into `runtime/*.nika`, and the `described.contracts` question of section 1 (-310) |
| Sun 11-01 | 8,900 | 840 | `describe.rs`, `foreign.rs`, `libraries.rs`, `cexport.rs`, `assets.rs`, `specbook.rs`, `fmt.rs`, `run_fast.rs` (-1,000); **O1** `orchestrator/cache.rs` (-450) | parser chunks 4-5 and the switch, `grammar!` deleted (#557 step 5, -2,100) | hands the five lowering files to lane 2; **std-2** `process.rs` env and `PATH`, `rt/config.rs` parse (-250) |
| Mon 11-02 | 6,200 | 150 | `sysroot.rs` driver (`lower_tools_files_cached`' run), `project.rs` drivers (-1,300); **O2** `orchestrator/project.rs` (-340) | `text_tiers.rs`, `view_changes.rs`, `type_rules.rs`, `one_variant.rs`, `views.rs` (-1,400) | **std-3** `fs.rs` path and `Root` checks, `supervision.rs` policy (-350) |
| Tue 11-03 | 3,600 | 0 | `modules.rs` drivers, `diagnostics/mod.rs`, `main.rs` to the `clap` adapter (-1,300) | `check/*`, `emit/*` cores (`with_contracts`' run, `Emitter`, entry points, -1,300) | **std-4** `http1.rs`, `search.rs`, `io.rs`, `time.rs` (-150); defects the last moves meet |
| **Wed 11-04** | **<= 2,800** | **0** | the rest of lane 1's rows over the adapter list | the rest of lane 2's rows | the CI step without `continue-on-error`; `promote` brings it to `main` |

**Variant B (D-1 not granted: two checkouts).** C3's work runs in C2 between
lane 2's briefs, and lane 1 takes nothing extra. Lane 2 loses about a day by
Saturday (section 5), so the compiler column slips by about 1,300 lines from
Saturday on unless the levers make it up; O1 and O2 stay in lane 1 (Sunday,
Monday, as above); std-1 to std-4 do not run before 11-04 and follow on
11-05 and 11-06 in one lane. Under B, 11-04 is S-A plus the orchestrator plus
the `std` kernel list in CI: the strict scope of section 1a.

That is about 2,800 lines a day against 2,387 measured with one lane: each
compiler move lane at 1,300 a day, plus about 300 a day of orchestrator and
`std` from Saturday on; with the levers, two to three slices of 400-600 lines
a lane and day. Slices are
re-measured each morning by the architect (`rs2nika --score` drafts), and the
day's row is corrected here, not elsewhere.

**Critical path: the parser.** `parser/mod.rs` (2,280 lines, 358 rules) waits
on #557 steps 1-3 (grammar language features in the compiler), then is ported
in five chunks of about 70 rules into `tools/parser_grammar.nika`, beside the
Rust grammar, each chunk compared on every `tests/compiler` program, and
switched in one commit. **#557 starts today** in the fix checkout. Decided
here (architect, within ADR-339 and ADR-294 D7): the compiler's own grammar
runs over the source as `Bytes`, so `span(p)` gives the `Span` of two `u32`
the span table needs; the text form (`text::Match`, `text::Pos`) waits for
#743 and is not on this path. If #557 step 3 is not green by Friday 10-30
evening, see *Risks*.

**Second path: `check/*` and `emit/*` cores** (Tuesday). They are the top of
the analyses and are last in lane 2 by the call graph; their crossings fall
away with them. The architect writes their briefs on Saturday from the
call graph of that day.

## 3. The lanes

### Checkouts and binaries

| Checkout | Path | Branch | Lanes | `target/` |
| :--- | :--- | :--- | :--- | :--- |
| C1 | `/home/user/Nikaia` | `develop` | lane 1 (the driver, `scripts/drive.nika`); architect and supervisor docs edits (no builds) | yes |
| C2 | `/home/user/nikaia-lane2` | detached `HEAD` | lane 2 | yes |
| C3 | `/home/user/nikaia-fix` | detached `HEAD` | lowering lane and compiler lane, **one agent at a time** | yes |

Why three: every move and every fix changes `tools/*.nika`, and `lower-std`
lowers the whole package. A half-written `.nika` of one agent makes the next
agent's `cargo build` or `lower-std` fail in a shared checkout, so two agents
that build cannot share one. The worktrees are `git worktree add --detach
<path> origin/develop` (CLAUDE.md forbids only a second checkout with `develop`
checked out). Disk: 13 GB free at 11:30; deleting `target/debug` in C1
(2.2 GB) and pruning `target/nikaia-project-tests` to `NIKAIA_CACHE_LIMIT=2G`
(2.2 GB) leaves about 17 GB for two more `target/fast` of about 5 GB each.
Worktrees build with incremental compilation on (`profile.fast` sets it):
the 20-second rebuild after a one-file change rests on it, and turning it
off (as this plan first said) made every build of `nikaia` a full one. Only
a lane that finds less than 6 GB free sets `CARGO_INCREMENTAL=0`; one under
4 GB stops and reports. (16 GB free at 12:15.) Three checkouts break CLAUDE.md's
*at most two clones with `target/`*: owner decision D-1; without it C3's
lanes run in C2 between lane 2's briefs and the day targets slip by about a
day (see *Risks*).

**The binary:** each checkout's `target/fast/nikaia` is used only by the agent
in that checkout. Nobody runs another checkout's binary. The driver in C1
runs `target/fast/nikaia` (`drive_exe`), which only lane 1's builder rebuilds,
between briefs or in its own cycle; a converter run or a probe by another
agent copies a binary into its scratchpad first.

### File ownership

Every Rust file has one owning lane for the week; a lane edits only its own.

| Lane | Rust files owned (`crates/nikaia/src/`) | Nikaia files owned (`crates/nikaia-std/src/tools/`) |
| :--- | :--- | :--- |
| 1 (moves) | `project.rs`, `comptime_run.rs`, `build_time.rs`, `grammar_run.rs`, `run_fast.rs`, `sysroot.rs`, `modules.rs`, `manifest.rs`, `describe.rs`, `foreign.rs`, `libraries.rs`, `cexport.rs`, `assets.rs`, `specbook.rs`, `fmt.rs`, `main.rs`, `diagnostics/mod.rs` | the ones its running brief names (`project_*`, `comptime_*`, `sysroot_*`, `modules.nika`, `manifest.nika`, `describe.nika`, `foreign.nika`, `libraries.nika`, `cexport*.nika`, `assets.nika`, `specbook.nika`, `fmt.nika`, new files) |
| 2 (moves) | `contracts/*`, `proofs.rs`, `prove.rs`, `bounds.rs`, `bounds_report.rs`, `fold.rs`, `fold_bound.rs`, `dsl.rs`, `check/*`, `emit/*`, `types.rs`, `traits.rs`, `ast/mod.rs`, every other file not named in this table; `parser/mod.rs` from #557 step 3 on; the five lowering files from Sunday 11-01 on | the ones its running brief names (`order`, `locks`, `sharing`, `maps`, `throws`, `touch`, `trust`, `sync`, `tether`, `keep*`, `ty`, `ledger_*`, `prove*`, `prover_*`, `bounds_*`, `fold*`, `dsl`, `parser_grammar`, new files) |
| Lowering | `text_tiers.rs`, `view_changes.rs`, `type_rules.rs`, `one_variant.rs`, `views.rs` until Sunday 11-01 | `emit_*` (not `emit_grammar`), `text_tiers`, `tiers`, `views`, `lends`, `type_rules`, `check_reads`, `check_facts`, `check_narrow` |
| Compiler | `parser/mod.rs` until #557 step 3 is pushed | `check_grammar`, `emit_grammar`, `grammar_decode`, `parse_*`, `parser_leaves`, `node_numbers`, the `check_*` file its bug names |
| Converter repair | `crates/rs2nika/` | none |
| Lane 1, from Sunday 11-01 | `crates/orchestrator/src/*` | `orchestrator_*` (new) |
| C3, std moves (variant A) | `crates/nikaia-std/src/{cli,html,process,fs,supervision,http1,search,io,time}.rs`, `rt/config.rs` | `crates/nikaia-std/src/runtime/*.nika` (new files only; `http1.nika` with std-4) |
| nobody this week | the K-std-1 and K-std-2 files of section 1, `std.contracts` | - |

**Each brief names its files under `Owns:`**, and the supervisor starts a
brief only if its `Owns:` is disjoint from every running brief's. A fix that
needs a file a move lane owns waits for that slice to land, or the
supervisor reassigns the file for one brief. `docs/selfhosting-plan.md`,
`docs/architect.md`, `docs/priorities.md` and `scripts/self_hosting.py` are
edited in C1 only: lane 2 and the fix lanes put plan rows and new
`self_hosting.py` rows into their report (`Body:`), and the supervisor or the
architect writes them.

### The rule for `lower-std` and the generated files

`tools/lowered/*.rs` and `tools/lowered.rs` belong to no lane: they are the
output of the compiler over the committed `.nika`, and CI's
`the_committed_rust_is_what_this_compiler_lowers` makes every pushed commit
hold the fixpoint. So:

1. **Every lane runs `lower-std` in its own checkout and commits every
   lowered file that changed**, its own and others'. A compiler change that
   re-lowers another lane's file is committed with it; that is correct,
   because in its checkout that file is exactly what `develop` holds.
2. **Before each push a worktree lane rebases** (`git fetch origin develop &&
   git rebase origin/develop`), rebuilds, runs `lower-std` once more, and, if
   a lowered file changed, commits it as `lowered: regenerate after rebase`;
   then `git push origin HEAD:develop`. Rejected: rebase again, at most three
   times, then report `Pushed: rejected`.
3. **A conflict in a generated file is never merged by hand**: `git checkout
   --theirs -- <file>` (or `--ours`, either), finish the rebase, rebuild,
   `lower-std`, commit the result. The `.nika` decides; the `.rs` follows.
4. In C1 the driver's builder does not rebase (CLAUDE.md, shared checkout).
   When its push is rejected, the supervisor rebases C1 between briefs and
   applies 3. The other lanes push small and often, so C1 sees each of their
   commits as one rebase step.
5. The lowering and compiler lanes push a compiler change **together with**
   its regenerated files, in one commit (no tree on `develop` where the
   compiler and its output disagree).

### Converter repair

The 60 % rule stays as `scripts/drive.nika:85` has it. When the driver
ESCALATEs on it, the supervisor starts the converter repair as the **next
brief in C3** (ahead of the queue there; `crates/rs2nika/` only, so it
touches no lane's files) and restarts the driver on the next move, which
does not wait for the repair. The move that escalated is re-briefed after the
repair lands. `drive.nika` itself is not changed this week.

### The fix checkout's queue

Priority in C3, one agent at a time: (1) a converter repair after an
ESCALATE; (2) #557 steps 1-3 (critical path); (3) a lowering defect that
blocks a briefed slice; (4) the lowering queue: #916 (absorbs #913, #914),
#910, #907, #908, #915, #911, #917, #918, #899, #896; (5) compiler bugs that
block a move: #909, #898, #900, #882; (6) in variant A, the `std` moves std-1
to std-4 (section 2), one at a time, each behind a free queue. A bug that blocks nothing waits until
after 11-04 (the owner's *bugs before moving* default is overridden by the
deadline, as `docs/priorities.md` allows when the figures say so: they say
2,600 lines a day).

### How the driver is kept off the lane briefs

`scripts/batch.nika` takes briefs only from `docs/briefs/batch.txt`, in its
order. The lane briefs are named `lane2-*`, `lane-lowering-*`,
`lane-bugs-*`, `lane-std-*` and are never written into `batch.txt`; the supervisor starts
them with `nikaia run scripts/agent.nika -- <brief>` from the copy of the
binary in C1's scratch, the builder working in C2 or C3 as its brief says.

### 3a. Throughput levers (proposals; the supervisor decides and briefs them)

Where a move's time goes is estimated, not measured (L5). The figures below
come from the run records (`docs/briefs/*.run.json`) and builder.md's own
build costs.

| Lever | The change | Expected saving |
| :--- | :--- | :--- |
| **L1 The converter's 60 % rule records, it does not stop** | `scripts/drive.nika`'s `drive_kept` (line 77) turns its stop into a line in the cycle's output (`CONVERTER <brief> kept N of M`), and the supervisor queues one converter repair a day in C3 from those lines (#877's list), instead of one repair per move with the pipeline standing. The draft stays the start of every move. | Today about 25 of a move's 56 minutes are the stop, the repair and the restart: up to **~45 % of lane 1's wall time**, the largest single reserve. Risk: the converter's share drifts lower during the day; the daily repair caps it. |
| **L2 A pre-step writes the draft** | Before `claude -p`, `scripts/agent.nika` (or the driver) reads the brief's `Cut:` line (the functions to move; a field the architect adds to every move brief from now on), runs `crates/rs2nika/convert --only NAME` on each, `rs2nika --lint`, and `nikaia run scripts/where.nika -- NAME...`, and writes `draft.nika`, `notes.txt`, `where.txt` into the brief's scratch dir; the brief points at them. A cut the converter cannot take is reported before any model runs. | 5-10 turns of 40-75 and 2-4 minutes per move; at ~75k context a turn, **0.4-0.8 M cache tokens a move**. |
| **L3 Larger slices: 400-600 lines, not 1,000** | The architect cuts 400-600 lines a brief (a driver with its crossings), `max_turns` 120, `budget_usd` 25. Not 1,000: a move of 170 lines already took 75 turns at 75k context (P3b); 1,000 lines would cross 120 turns and the 150k reset rule, and a red push then costs a whole day's slice. | The fixed cost a brief pays once (orientation, a cold `lower-std`, the `it` binary build, the guard set, clippy's own build in `target/debug`, pre-push, commit and push, the driver's turnaround) is estimated at 12-15 minutes. At 170 lines it is about half of the builder's time, at 500 about a quarter: **+40-60 % lines an hour**. |
| **L4 Cheaper gates for a move** | (a) The guard set and the touched `it` tests run **once**, at the end, beside clippy (builder.md's *Clippy beside the last test run*), not after every cycle; cycles in between run only the probes and `nikaia test --project tests/compiler` on the moved subcommand's programs. (b) The brief names the touched tests (the architect greps them from the moved module), so the builder does not search. (c) A move that touches no `check_*`/`emit_*` file never runs `--both-settings` over `tests/compiler` (already the rule; state it in each brief). (d) Incremental builds on in every checkout (section 3). | (a) one `it` binary build and one guard run per brief instead of two to three: **5-10 minutes**; (b) 2-4 turns; (d) up to 50 seconds per build in C2/C3. |
| **L5 Keep the transcripts** | Drop `--no-session-persistence` at `scripts/agent.nika:64` for this week; delete transcripts older than three days at the start of each driver run. A transcript is a few MB. | Saves nothing by itself; without it `docs/tokenspeed.md` has had no row since 10-25 and L1-L4 stay estimates. **Recommended: drop the flag.** |
| L6 Briefs ahead | The architect's morning run leaves two cut briefs ready per lane, so neither the driver nor lane 2 waits for an architect run. | the wait between briefs; not measured |
| L7 One cache limit | builder.md says `NIKAIA_CACHE_LIMIT=4G`, the owner's brief rule 2G: builder.md is corrected to 2G (a process defect at its source; the supervisor's file). | disk, not time |

Two building lanes on four cores share the CPU (section 1a): L1, L3 and L4
cut tool time and so scale; more lanes do not.

## 4. Cost

Measured (`docs/briefs/cost.txt`): 10-26 39 runs 20 USD, 10-27 57 runs 58 USD,
10-28 to 11:30 19 runs 36 USD; a run costs 0.6-5.9 USD, about 1.9 on average.
The cap is 1,000 USD a day; the plan stays far below it on purpose, since
the binding limit is quality, not money.

| Lane | Runs a day | Per run (`budget_usd`) | A day, at most |
| :--- | ---: | ---: | ---: |
| Lane 1 (driver) | 4-6 | 25 | 80 USD |
| Lane 2 | 4-6 | 25 | 80 USD |
| C3 (lowering + compiler + converter) | 5-8 | 20 | 80 USD |
| Architect (morning re-plan, briefs) | 2-3 | - | 40 USD |
| Main session (supervision, rebases, CI) | - | - | 60 USD |
| **In all** | | | **340 USD**, reserve to 1,000 |

**Stop rule, per lane and day.** A worktree lane goes back to serial (its
next brief waits for lane 1's driver, in C1) when any of these holds:
two red `develop` runs caused by its commits; two conflicts the supervisor had
to clean up by hand; its net lines under 40 % of its day target by 18:00 UTC;
its cost over twice its row above. Serial again for at least one day; the
architect re-plans the rows here. If lane 1 itself stalls (two ESCALATEs in a
day), lane 2 becomes the only move lane until the converter repair lands.

## 5. Risks and fallbacks

* **#557 late** (grammar features not green by Friday evening). Fallback,
  in order: (a) the compiler lane takes #557 step 4's first chunk itself
  over the weekend; (b) owner decision D-3: the `grammar!` block counts as an
  adapter to `winnow-grammar` for this deadline, and the port follows.
* **Day 4 (Saturday) behind by more than 1,500 lines.** Cut, in this order:
  (0) std-2 to std-4 move to 11-05/11-06 (D-6); (1) the fix queue's items (4) and (5) that block no slice; (2) the
  `#[cfg(test)]` modules move as deletions where `tests/compiler` already
  covers the code, and new Nikaia tests only for what nothing covers; (3)
  style passes in moves (iterator house style, doc comment rewrites) become a
  later issue; (4) the slices are made larger (600-800 lines), the 30-minute
  limit of the driver permitting; (5) only then a third move stream, which
  needs the owner (D-4).
* **A converter defect under 60 % on many slices at once**: the repair is
  one brief per form (#877's list), in C3, before any other C3 work.
* **Rebase churn in C1** (lowered files re-lowered by other lanes): the cost
  shows in the supervisor's turns; if it exceeds two manual cleanups a day,
  the stop rule applies to the lane that caused them.
* **CPU**: four cores, and 77-93 % of a builder's time compiles or tests.
  Two lanes building at once are estimated at 1.3x one lane, three at about
  1.4x. If Thursday's two-lane figure is under 1.2x lane 1's Wednesday, the
  lanes do not pay: lane 2 then takes only slices whose build is short, and
  the levers come before any further lane.
* **The `std` moves** change what every program runs (the runtime package
  is lowered into each program's `std`). A red test from a `std` move is
  reverted at once, and the remaining std-N waits for 11-05: `std` is the
  first thing cut (D-6).
* **Disk**: three `target/` with 16 GB free. A lane under 4 GB free deletes
  its `target/` and stops; the supervisor prunes caches before restarting.
* **CI time**: about 45 minutes a run, and one run per push. Not a blocker:
  `promote` takes the newest green commit.
* **`check/*` and `emit/*` on Tuesday** are the largest unknowns (the
  `Emitter` and the `CheckHost`/`EmitHost` traits). The architect maps their
  call graph on Saturday; if it is more than 2,000 lines of moves, lane 1
  takes `emit/*` after `main.rs`, and the owner hears about it the same day.

## 6. First briefs

* Lane 2: [`briefs/lane2-l21-dsl-bounds-report.md`](briefs/lane2-l21-dsl-bounds-report.md)
  (`dsl.rs`, a crossing, deleted; `bounds_report.rs`'s walk into a new `bounds_report.nika`). The crossings of `contracts/` follow as L2-2/L2-3: their callers in `project.rs` and `modules.rs` (`order::overlap_report`, `sharing::report`, `maps::keyed_functions`) are lane 1's, so those three keep a one-line Rust forward until lane 1 moves the caller.
* Lowering: [`briefs/lane-lowering-916-ref-bytes-option.md`](briefs/lane-lowering-916-ref-bytes-option.md)
  (#916, absorbs #913 and #914), then the queue of section 3.
* Compiler: [`briefs/lane-bugs-557a-span-here.md`](briefs/lane-bugs-557a-span-here.md)
  (#557 steps 1-2), then #557 step 3.

In C3 the compiler brief runs first (critical path), the lowering brief
second.

## 7. For the owner (decision proposals, `docs/supervisor.md`)

**D-1. A third checkout with `target/`.** *What it is:* CLAUDE.md says at
most two clones with `target/` on this machine, because the disk is small.
*Why it comes up:* this plan runs four lanes; any two agents that build need
their own checkout (a half-written `.nika` breaks the other's build). With
the lowering and compiler lanes taking turns in one checkout, three are
needed: C1 (lane 1), C2 (lane 2), C3 (fixes). *Facts:* 16 GB free at 12:15 UTC, about
20 GB after pruning C1; a `target/fast` is about 5 GB. *Options:* (a) three
checkouts, with the 4 GB stop rule; (b) two, the fixes running in C2 between
lane 2's briefs: about one day lost by Saturday, and `std`'s moves after
11-04 (section 2, variant B). *Recommendation:* (a), for this week only.

**D-2. What "complete bootstrap" covers.** *Answered by the owner,
2026-10-28:* the compiler, `crates/orchestrator` and `crates/nikaia-std`.
Section 1 turns that into kernel lists; D-5 asks the owner to confirm them.

**D-5. Which Rust `std` keeps, for the sponsors.** *What it is:* `std` is the
library every Nikaia program runs on; 12,088 of its lines are hand-written
Rust. Example: `m[k]` in a Nikaia program becomes `nikaia_std::index::get(&m,
k)` in the Rust the compiler writes, and `index::get` is Rust with lifetimes
and impls of Rust's traits. *Why it comes up:* the owner's D-2 answer puts
`std` in scope; the specification already says what a `.nika` in `std` may
hold (ADR-014 D1: what Stage 0 can lower) and that Nikaia has no lifetime
annotations, ever (Part I 6.5). *Facts:* about 1,060 lines are logic that can
move (section 1); about 10,950 are the types the compiler lowers into
(5,134), the runtime and concurrency below the language (4,119), and the
calls to the OS and crates (~1,700). Writing the first two in Nikaia would
need lifetimes, impls of Rust traits and atomics in the language, which
Part I excludes on purpose: a language change of months, not a week.
*Risk against the recommendation:* a sponsor who reads "std in Nikaia" as
"no Rust in std" sees 10,950 lines and calls it unfinished. *Options:* (a)
the kernel lists of section 1 are the definition, fixed in
`scripts/bootstrap-adapters.txt` and held by CI, each row with its reason;
(b) the lists plus a later ADR on which K-std-3 adapters the language could
absorb (`std::process` with inherited stdio, a `sha256` in `std`); (c) the
literal reading, no Rust in `std`: not reachable, by design. *Recommendation:*
(a) now, (b) after 11-04.

**D-6. What is cut first if Saturday is behind.** *What it is:* the
checkpoint of section 2 (compiler at 12,000, orch + std at 1,540 by Saturday
evening). *Facts:* S-B needs 2,800 lines a day, 1.17x the best day measured,
with the parser and the `check`/`emit` cores last (section 1a). *Options:*
(a) cut `std`'s moves first (they change what every program runs, the
riskiest to hurry), then the orchestrator, never Stage 2 of the compiler;
(b) cut the orchestrator first; (c) move the deadline for all of S-B.
*Recommendation:* (a); the owner hears the Saturday figures that evening.

**D-3 (only if #557 is late).** May the `grammar!` block count as a
`winnow-grammar` adapter at the deadline, the port following in the next
week? *Recommendation:* decide on Friday evening, from #557's state then.

**D-4 (superseded by variant A).** The third move stream is C3's `std`
moves of section 2 once its fix queue is empty; it needs D-1, and nothing
beyond it.
