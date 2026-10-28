# The bootstrap week: 2026-10-28 to 2026-11-04

The owner's decision (2026-10-28, binding, a condition of the budget): by
**2026-11-04** self-hosting is complete, as a **complete bootstrap**. At most
two parallel move lanes, one lane for lowering defects, one for compiler bugs;
parallelism that lowers quality is counterproductive, and token cost counts.
This file is the plan; the architect's *State* (`docs/architect.md`) and the
plan rows (`docs/selfhosting-plan.md`) stay where they are and point here.

Counts are code lines of `scripts/self_hosting.py` (no comments, no blanks).
At 2026-10-28 11:30 UTC: **21,035** Rust lines in `crates/nikaia/src`, 59
files, 80.6 % self-hosted; 1,321 of the Rust lines are `#[cfg(test)]` modules.
Pace of the last 24 hours: about 2,200 lines net with one move lane.

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
   if any `.rs` under `crates/nikaia/src` holds a function or a type not on
   the list, or a file holds more code lines than its row allows. Module
   declarations (`mod`, `pub use nikaia_std::tools::...`) are free.

So Stage 2 = Stage 1 (only the adapters below are Rust) + that step green.
Stage 1 alone is what the second check proves; Stage 2 is the first.

**Scope**: `crates/nikaia/src`, what ADR-294 D4 measures. Not in scope (see
*Owner* D-2): `crates/nikaia-std`'s hand-written Rust (the standard library and
the `Span` doors of ADR-294 D6, not the compiler), `crates/orchestrator`
(2,040 lines driving `cargo` and `rustc`: an adapter to those programs by
nature), `crates/rs2nika` (a development tool), and the Rust tests under
`crates/nikaia/tests` (#821 goes on, not in this week). The `#[cfg(test)]`
modules inside `crates/nikaia/src` are in scope: each leaves with the code it
tests, as a `tests/compiler` program where the moved code is not already
covered, else deleted.

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

A process the Nikaia `std::process` could start (no environment, no working
directory, no stdin) is not an adapter: it moves. A missing `std` entry met
on the way is filed (`ready`) and its row's adapter is named in the issue, so
the list shrinks after the deadline rather than growing in it.

## 2. Day targets

Order inside each lane: leaves of the Rust call graph first. Today most of
the remaining Rust is either a **driver** (the run of analyses, the project
loop, `main`) or a **crossing**: a thin Rust function that turns `&Parsed` and
`Ledger` into the Nikaia call (`contracts/order.rs` is 71 lines around
`order.nika`'s 790). A crossing goes when its last Rust caller moves, so each
slice takes a driver together with the crossings only it calls.

| End of | Rust lines | Lane 1 (main checkout, the driver) | Lane 2 (worktree) | Fix checkout (lowering, then compiler) |
| :--- | ---: | :--- | :--- | :--- |
| Wed 10-28 | 20,100 | P4a, P4b (`project.rs`, -660) | L2-1 the crossing `dsl.rs` deleted, `bounds_report.rs`'s walk into Nikaia (-290) | #916 (absorbs #913, #914); #557 steps 1-2 (`span(p)`, `here`) |
| Thu 10-29 | 17,400 | P1 settings, K1+K2 comptime tree, P5 `project.rs` build drivers (-1,400) | L2-2 the `contracts/{order,locks,sharing,maps,throws,touch}.rs` crossings, L2-3 `contracts/{trust,sync,tether,keep,keeps}.rs` (-1,300) | #557 step 3 (`same_line`); #910, #907, #908, #915 (text views) |
| Fri 10-30 | 14,700 | S1 `sysroot` package, M3 `modules.rs` collect, `manifest.rs` (-1,350) | C1 `contracts/mod.rs` + `send.rs` + `ty.rs` (-1,350); parser chunk 1 written beside | #911 (`EitherText`), #917, #918; converter repairs as they come |
| **Sat 10-31** (checkpoint) | **12,000** | K3 `comptime_run.rs` rest, `build_time.rs`, `grammar_run.rs` (-1,300) | `proofs.rs`, `prove.rs`, `bounds.rs`, `bounds_report.rs`, `fold*.rs`, `dsl.rs` (-1,400); parser chunks 2-3 | #909, #898, #900 (checker bugs that block moves) |
| Sun 11-01 | 8,900 | `describe.rs`, `foreign.rs`, `libraries.rs`, `cexport.rs`, `assets.rs`, `specbook.rs`, `fmt.rs`, `run_fast.rs` (-1,000) | parser chunks 4-5 and the switch, `grammar!` deleted (#557 step 5, -2,100) | hands `text_tiers.rs`, `view_changes.rs`, `type_rules.rs`, `one_variant.rs`, `views.rs` to lane 2 |
| Mon 11-02 | 6,200 | `sysroot.rs` driver (`lower_tools_files_cached`' run), `project.rs` drivers (-1,300) | `text_tiers.rs`, `view_changes.rs`, `type_rules.rs`, `one_variant.rs`, `views.rs` (-1,400) | `scripts/bootstrap.nika`, `scripts/bootstrap-adapters.txt`, the CI step (red until done: run it with `continue-on-error` till 11-04) |
| Tue 11-03 | 3,600 | `modules.rs` drivers, `diagnostics/mod.rs`, `main.rs` to the `clap` adapter (-1,300) | `check/*`, `emit/*` cores (`with_contracts`' run, `Emitter`, entry points, -1,300) | defects the last moves meet |
| **Wed 11-04** | **<= 2,800** | the rest of lane 1's rows over the adapter list | the rest of lane 2's rows | the CI step without `continue-on-error`; `promote` brings it to `main` |

That is about 2,600 lines a day against 2,200 with one lane today: each move
lane at 1,300 a day, three to four slices of 350-450 lines. Slices are
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
Worktrees build with `CARGO_INCREMENTAL=0` (smaller `target/`); a lane that
finds less than 4 GB free stops and reports. Three checkouts break CLAUDE.md's
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
block a move: #909, #898, #900, #882. A bug that blocks nothing waits until
after 11-04 (the owner's *bugs before moving* default is overridden by the
deadline, as `docs/priorities.md` allows when the figures say so: they say
2,600 lines a day).

### How the driver is kept off the lane briefs

`scripts/batch.nika` takes briefs only from `docs/briefs/batch.txt`, in its
order. The lane briefs are named `lane2-*`, `lane-lowering-*`,
`lane-bugs-*` and are never written into `batch.txt`; the supervisor starts
them with `nikaia run scripts/agent.nika -- <brief>` from the copy of the
binary in C1's scratch, the builder working in C2 or C3 as its brief says.

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
  (1) the fix queue's items (4) and (5) that block no slice; (2) the
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
* **Disk**: three `target/` with 13 GB free. A lane under 4 GB free deletes
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
needed: C1 (lane 1), C2 (lane 2), C3 (fixes). *Facts:* 13 GB free now, about
17 GB after pruning C1; a `target/fast` is about 5 GB. *Options:* (a) three
checkouts, with the 4 GB stop rule; (b) two, the fixes running in C2 between
lane 2's briefs: about one day lost by Saturday. *Recommendation:* (a), for
this week only.

**D-2. What "complete bootstrap" covers.** *What it is:* ADR-294 D5 speaks
of the compiler's pipeline. *Facts:* besides `crates/nikaia/src` (21,035
lines) there are `crates/nikaia-std` (17,210 hand-written Rust lines: the
standard library), `crates/orchestrator` (2,040: drives `cargo`/`rustc`) and
`crates/rs2nika` (8,086: the converter). *Options:* (a) `crates/nikaia/src`
only, with the adapter list of section 1; (b) also `orchestrator`. Neither
fits `std` into seven days. *Recommendation:* (a); `orchestrator` is an
adapter to two programs and is listed as one.

**D-3 (only if #557 is late).** May the `grammar!` block count as a
`winnow-grammar` adapter at the deadline, the port following in the next
week? *Recommendation:* decide on Friday evening, from #557's state then.

**D-4 (only if Saturday is behind).** A third move stream, in C3 once its
queue is empty (C3 then moves `diagnostics/mod.rs` and `fmt.rs`, which no
other lane touches that day). *Recommendation:* only on Saturday's figures.
