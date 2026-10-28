# The bootstrap week: 2026-10-28 to 2026-11-04

The owner's decision (2026-10-28, binding, a condition of the budget): by
**2026-11-04** self-hosting is complete, as a **complete bootstrap**. At most
two parallel move lanes, one lane for lowering defects, one for compiler bugs;
parallelism that lowers quality is counterproductive, and token cost counts.
This file is the plan; the architect's *State* (`docs/architect.md`) and the
plan rows (`docs/selfhosting-plan.md`) stay where they are and point here.

**The goal, as the owner corrected it (2026-10-28, two messages; the first D-2
hand-over was mis-transmitted):**

1. The sponsors want to see that **the Nikaia compiler is written in Nikaia**.
   Glue that only makes sense in Rust (bindings to Rust crates, the OS, the C
   library) stays Rust.
2. **`crates/orchestrator` moves completely**, or each part that stays Rust has
   a concrete reason (what Rust-only thing it needs, why `std` cannot offer it,
   what adding it would cost). A missing `std` entry is no reason: it is work to
   plan.
3. **`crates/nikaia-std` is out of scope.** Nothing in it moves, not even the
   logic that is neither glue nor a wrapper: moving single parts complicates
   things for little gain. If `std` is ever taken up, then whole modules in one
   piece.
4. **The compiler's Nikaia leaves the `std` crate.** Its 242 `.nika` stand in
   `crates/nikaia-std/src/tools` and read from outside as part of the standard
   library; they move to `crates/nikaia-compiler/src` (R0, #921, ADR-294's new D21),
   with the thin Rust adapter `crates/nikaia` beside it.

Counts are code lines of `scripts/self_hosting.py` (no comments, no blanks).
At 2026-10-28 12:45 UTC (P4a in flight): **87,503** Nikaia lines, **80.7 %**,
so about **20,930** Rust lines in `crates/nikaia/src`; 1,321 of them are
`#[cfg(test)]` modules. Pace of the last 24 hours: about 2,200-2,400 lines net
with one move lane. `crates/orchestrator/src`: **1,228** code lines plus 812 in
tests. After R0 `self_hosting.py` also counts the hand-written
`crates/nikaia-compiler/src/lib.rs` (about 120 code lines without its tests;
inside `std` it was not counted): the Rust column rises once by that much,
honestly.

## 1. Definition of done

**Stage 2 is reached, and held by CI, when the `bootstrap_compiler` job of
`.github/workflows/nikaia.yml` passes a step named `Bootstrap (ADR-294 D5,
Stage 2)` on a `develop` commit that `promote` brings to `main`.** That step
runs, after `Build the workspace`:

1. `NIKAIA_CACHE_DIR=$(mktemp -d) target/release/nikaia lower-std` (cold: no
   cached file may stand in for a lowering), then
   `git diff --exit-code -- crates/nikaia-compiler/src/lowered crates/nikaia-compiler/src/lowered.rs crates/nikaia-std/src/runtime`.
   The binary was built from the committed lowered Rust plus the adapters, so
   an empty diff is the fixpoint ADR-294 D5 names: the lowered compiler lowers
   its own sources to byte-identical Rust. (A second generation needs no run:
   identical sources build the identical compiler.) The existing test
   `the_committed_rust_is_what_this_compiler_lowers`
   (`crates/nikaia/tests/it/sysroot.rs`) already compares the same bytes; the
   step adds the cold cache and the name.
2. `target/release/nikaia run scripts/bootstrap.nika`, a new checker in
   Nikaia (CLAUDE.md: tools in Nikaia): it reads `scripts/bootstrap-adapters.txt`
   (the table below, one `file function max_lines reason` row each) and fails
   if any hand-written `.rs` under `crates/nikaia/src` or
   `crates/nikaia-compiler/src` (not `lowered/`) holds a function or a type not
   on the list, or a file holds more code lines than its row allows, or
   `crates/orchestrator` exists. Module declarations (`mod`,
   `pub use nikaia_compiler::...`) are free. `std` is not scanned (point 3).

So Stage 2 = Stage 1 (only the adapters below are Rust) + that step green.
Stage 1 alone is what the second check proves; Stage 2 is the first.

**Scope:** `crates/nikaia/src` (what ADR-294 D4 measures),
`crates/nikaia-compiler/src`'s hand-written `lib.rs`, and `crates/orchestrator`
(to zero: the crate is deleted, #923). Not in scope: `crates/nikaia-std` (point
3; it gains only the new entries #920 and #922 the orchestrator needs, see
D-7), `crates/unsafe/*` (ADR-218 D1, D4), `crates/rs2nika` (a development
tool), and the Rust tests under `crates/nikaia/tests` (#821 goes on, not in
this week). The `#[cfg(test)]` modules inside the compiler and the
orchestrator are in scope: each leaves with the code it tests, as a
`tests/compiler` program where the moved code is not already covered, else
deleted.

**The adapters allowed to remain** (the fixed list; at most about 2,950 lines
in all). Each is Rust only because a crate or an OS call stands behind it;
everything it decides is a call into Nikaia.

| File | What may stay | At most | Reason |
| :--- | :--- | ---: | :--- |
| `main.rs` | `Cli`, `Command`, `CacheAction`, `Verb` (the `clap` derives), `main` turning them into one call of a Nikaia entry per subcommand, `std::process::exit` | 200 | `clap` |
| `parser.rs` (was `parser/mod.rs`) | the entry points `parse_to_ast`, `parse_expression_from` calling the lowered grammar; the conversion of a `winnow` error into the Nikaia `ParseError`; `Parsed`'s caches only if a tools file cannot hold a lock | 200 | `winnow`, `winnow-grammar` |
| `ast/mod.rs` | `int_value`, `int_literal` | 70 | `i128` (ADR-294 D6) |
| `manifest.rs` | `Manifest::read`/`parse`: `toml::Value` into the Nikaia manifest view; `rooted` (from `project.rs`); the same reading for `nikaia.lock` and `Cargo.lock` (from the orchestrator, #923) | 190 | `toml`; a TOML reader in Nikaia would be a grammar of 400-600 lines after #557, not this week, and `std` gains no TOML module (point 3) |
| `sysroot.rs` | `Sysroot::resolve`, `COMPILER_IDENTITY`, `rlib_cache` (`env!`), `rustc_cpus` (a `rustc` process), `in_parallel` (scoped threads with a 256 MB stack), `sha256_hex` until #922 step 1, `Phases` | 250 | `env!`, `std::thread` |
| `comptime_run.rs`, `grammar_run.rs`, `build_time.rs`, `run_fast.rs` | compiling and running a generated program (`rustc`/`cargo` processes with environment and working directory), the `AHEAD` `thread_local` | 350 together | processes until #920; `thread_local` |
| `project.rs` | the test binaries' spawns with stdin, working directory and exit status (until #920 step 2), `Sandbox`, `current_exe` | 250 | processes, temporary directories |
| `contracts/mod.rs` | the trait impls `LedgerOps`, `SignatureOps`, `VariantOps`, `SyncOps` (one-line adapters), `std_ledger` (`OnceLock`), `toolchain` (`env!`) | 150 | Rust traits over `nikaia_std` types, `env!` |
| `emit/mod.rs` | `Build` with `host_target`/`default_target`; the `rayon` fan-out in `with_contracts` | 150 | `impl Default` of a `nikaia_std` type (E12-4), `env!`, `rayon` |
| `diagnostics/mod.rs` | the terminal and colour decision, the `serde_json` writer | 80 | `serde_json`, the terminal |
| `describe.rs`, `cexport.rs`, `libraries.rs`, `assets.rs` | the `cargo`/`node-gyp`/`rustc` invocations, `sha2` digests | 150 together | processes, `sha2` |
| `crates/nikaia-compiler/src/lib.rs` | `Span`'s `usize` doors, the generated parser's entry `file`, `impl Display for Ty`, the imports the lowered files need (`sysroot::HAND_WRITTEN`) | 150 | `usize` (ADR-294 D6), `winnow`, Rust traits |
| any file | `anyhow` context at a crate boundary, `catch_unwind` | in the rows above | `anyhow` |

A process the Nikaia `std::process` can start is not an adapter: it moves.
`process::run` today takes `dir:`, `env:`, `env_remove:`, `env_clear:` and
`within:` and captures both outputs, with stdin closed. What it cannot do yet
is #920 (inherited streams, one stream captured and the other passed through,
a stdin text); only those spawns are adapters, and each row says until when.
A missing `std` entry met on the way is filed (`ready`) and its row's adapter
is named in the issue, so the list shrinks rather than grows.

### `nikaia-std`: out of scope, and why that is honest

Nothing in `crates/nikaia-std` moves (point 3). `std` is the library every
Nikaia program runs on: the types the emitter writes calls into
(`nikaia_std::index::get`, `Bytes`, `Map`; Part I 6.5 excludes the lifetimes
they are built of), the scheduler and locks below the language (Part II 12.2),
the calls to the OS and to crates. Its Nikaia half (`text.nika`,
`supervisor.nika`, `runtime/http1.nika`) stays where it is. The only `std`
change this week is **new** thin entries over Rust that the orchestrator's
move needs (#920 spawns, #922 sha256, environment, file locks, file times):
glue by design, the kind point 1 keeps in Rust (D-7).

### `crates/orchestrator`: moves completely, the crate is deleted (#923)

1,228 code lines (812 more in tests). Its Nikaia goes into the compiler's
package as `orchestrator_cache.nika` and `orchestrator_project.nika` (the
prefix rule), after R0 in `crates/nikaia-compiler/src`; the Rust callers in
`crates/nikaia` call `nikaia_compiler::orchestrator_*`. **No part has to stay
Rust inside it.** What it needs that Nikaia cannot say is a `std` entry
(#920, #922) or the compiler's `toml` adapter row; nothing is left for a crate.

| Part (`crates/orchestrator/src/`) | Verdict | Needs first |
| :--- | :--- | :--- |
| `lib.rs:1-64` `Cli`, `LanguageFrontend`, `Orchestrator` | deleted: no caller | - |
| `cache.rs:32-62` domains, `sha256_hex` | moves; the digest itself is `hash::sha256_hex` | #922 step 1 |
| `cache.rs:64-154` `Choices` | moves | - |
| `cache.rs:156-259` `UnitRecord`, `Lockfile` (`new`, `save` render TOML text in Nikaia, as `CargoProject::render` does) | moves; `load`'s TOML parse goes through `manifest.rs`'s `toml` row | - |
| `cache.rs:261-418` `Key::build`, `Key::sysroot`, `KeyBuilder` | moves | #922 step 1 |
| `cache.rs:420-614` `Artifacts`, `Store`, `Cache` (`lookup`, `record`, `save`) | moves; the store's JSON object of texts (`:559`, `:587`) is our own format, written and read in Nikaia (about 50 lines) | - |
| `cache.rs:616-794` `Layout` (`user_cache_dir`, `where_tools_start`, `resolve`, `unit_name`), `limit`, `parse_size`, `show_size` | moves | #922 step 2 (`env::var`, `env::temp_dir`) |
| `cache.rs:796-856` `HELD`, `ADDED`, `use_entry`, `hold`, `open_lock`, `settle` | moves; the two process-wide statics become a value the driver keeps and hands to `settle` | #922 step 3 (`fs::lock`) |
| `cache.rs:858-960` `Pruned`, `prune`, `Entry`, `entries`, `size_of` | moves | #922 step 4 (`modified`) |
| `cache.rs:962-973` `first_link` | moves | #922 step 4 (`links`, `file_id`) |
| `cache.rs:975-1031` `cargo_holds`, `mod lock` | moves; `mod lock` *is* the `std` entry | #922 steps 3-4 |
| `cache.rs:1033-1154` `clean`, `touch`, `sweep_programs`, `profile_dirs`, `used_at` | moves (`fs::remove`/`create_dir` take `recursive:`) | #922 step 4 |
| `cache.rs:1156-1774` tests | `tests/compiler` programs where `it/cache.rs` does not cover them, else deleted | with O1 |
| `project.rs:35-333` `Package`, `Profile`, `CrateKind`, `CargoProject`, `Workspace`, `string`, `key`, `write_if_changed` | moves | - |
| `project.rs:335-460` `Cargo`, `toolchain_is_new_enough`, `toolchain_can_build_for`, `installed_rustc`, `rustc_version`, `version_parts` | moves (captured `rustc` runs with `dir:`); the once-per-process memo of `rustc_version` becomes the caller's | - |
| `project.rs:461-560` `Cargo::run`, `Cargo::messages` | moves | #920 step 1 (inherited; stdout captured with stderr inherited) |
| `project.rs:562-571` `cargo_binary` | moves | #922 step 2 |
| `project.rs:573-724` `Invocation` (`parse`, `source_path`, `replace_source`, `dep_info`, `is_a_probe`, `run`) | moves | #920 steps 1-2 (`run`: inherited, stdin closed for a probe) |
| `project.rs:726-770` `resolved_versions` | moves; the TOML parse of `Cargo.lock` through `manifest.rs`'s row | - |
| `project.rs:772-813` `record_extra_dependencies`, `escape_make` | moves | - |
| `project.rs:815-1230` tests | as `cache.rs`'s | with O2 |

The one piece of the orchestrator's work that stays Rust anywhere is the TOML
parse (about 40 lines in `manifest.rs`): the `toml` crate, a format the
compiler reads but does not define; a Nikaia TOML grammar is 400-600 lines and
waits for #557's grammar features, so it is filed after 11-04 if the owner
wants it, not done this week.

### What remains Rust on 11-04, in all

Compiler adapters <= 2,950 (with `crates/nikaia-compiler/src/lib.rs`);
orchestrator 0, the crate deleted; about 150 new lines of `std` glue (#920,
#922). Out of scope and unchanged: `crates/nikaia-std` (12,088 hand-written
code lines), `crates/unsafe` 2,515 (ADR-218). By `self_hosting.py`'s measure
the compiler is then about **97 %** Nikaia (about 106,000 lines against at most
2,950), every remaining Rust line on a list with its reason, and
`scripts/bootstrap.nika` holds CI to the list.

## 1a. Feasibility, in figures

Measured pace (one move lane, the driver): 7 moves from 03:39 to 10:08 UTC on
10-28 removed 1,135 net lines; 24 hours: **-2,387**. A move takes about 56
minutes of wall time, of which the builder runs 27.5 (P3b) to 33.6 (M1); the
rest is the ESCALATE stop, the converter repair (30-50 minutes) and the
restart. Inside a builder 77 % (P3b) to 93 % (M1) of the wall time is tools:
`cargo build`, `lower-std`, the tests, clippy. The machine has **4 cores**.

Time left: from 10-28 13:00 to 11-04 16:00 (the last push that CI, about 45
minutes, and `promote` can still carry that day): **7.1 days**.

| Scope | To move | Lines a day | Against 2,387 measured |
| :--- | ---: | ---: | ---: |
| S-A: the compiler to its adapters | 17,980 | 2,530 | 1.06x |
| **S-B: the owner's: S-A + the orchestrator** | **19,210** (17,980 + 1,228) | **2,710** | **1.13x** |
| plus R0 (#921) | 0 lines, one brief | about 2 hours of a quiet C1 | - |
| plus #920, #922 (new `std` entries) | about 150 lines of Rust written, not moved | C3, Thursday to Saturday | - |

Dropping `std`'s moves (the earlier 1,060 lines and four C3 briefs) lowers
S-B from 2,800 to about 2,710 a day and frees C3 for #920 and #922. **11-04
holds on the same conditions as before, with a little more margin:** levers
L1-L4 (section 3a) live by Thursday morning, #557 step 3 green by Friday
evening, the `check`/`emit` cores at most 2,000 lines of moves. Without the
levers, at 2,387 a day, S-B ends about 11-05 15:00.

1. **The CPU, not the lanes, is the ceiling.** 77-93 % of a builder's time is
   compiling and testing on 4 cores; a second building lane is estimated at
   1.3x, not 2x (nothing has measured two building lanes here).
2. **The pace so far was measured on leaves** (`project.rs`, `modules.rs`).
   The parser (2,280 lines behind #557 steps 1-3) and the `check/*`, `emit/*`
   cores come last, when the margin is gone.
3. **Half of the wall time is the stop after each move** (ESCALATE, repair,
   restart): lever L1, a process change, not more lanes.

## 1b. Notes for the sponsors (what "in Nikaia" honestly means)

The compiler is Nikaia, in its own crate `crates/nikaia-compiler`, except a
fixed list of adapters to Rust crates and to the OS in `crates/nikaia`. The
orchestrator (caches, Cargo projects, the `rustc` wrapper) is Nikaia too. The
standard library is not part of this claim: it is the layer under every Nikaia
program (the types the compiler lowers to, the scheduler, the locks, the
system calls), and self-hosted toolchains keep such a layer: GHC's runtime
system is C, Go's runtime has assembly, Rust's `core` has compiler intrinsics.
What makes it a bootstrap is the fixpoint (section 1), and that the compiler's
remaining Rust is small, listed with a reason per row, and checked by CI.

## 2. Day targets

Order inside each lane: leaves of the Rust call graph first. Today most of
the remaining Rust is either a **driver** (the run of analyses, the project
loop, `main`) or a **crossing**: a thin Rust function that turns `&Parsed` and
`Ledger` into the Nikaia call (`contracts/order.rs` is 71 lines around
`order.nika`'s 790). A crossing goes when its last Rust caller moves, so each
slice takes a driver together with the crossings only it calls.

Variant A (D-1 granted: three checkouts). Column *orch* is what is left of the
orchestrator's 1,228 lines. **R0** (#921, `briefs/r0-compiler-crate.md`) runs
once, in C1, at the first quiet moment after P4a and #557 steps 1-2 are pushed
(Wednesday evening, at the latest Thursday before the levers): every later
brief then names `crates/nikaia-compiler/src`.

| End of | Compiler Rust | orch | Lane 1 (main checkout, the driver) | Lane 2 (worktree) | Fix checkout C3 (lowering, then compiler, then the new `std` entries) |
| :--- | ---: | ---: | :--- | :--- | :--- |
| Wed 10-28 | 20,100 | 1,228 | P4a, P4b (`project.rs`, -660); **R0** at the quiet moment | L2-1 the crossing `dsl.rs` deleted, `bounds_report.rs`'s walk into Nikaia (-290) | #916 (absorbs #913, #914); #557 steps 1-2 (`span(p)`, `here`) |
| Thu 10-29 | 17,500 | 1,228 | levers L1-L2 (section 3a) first, between two briefs; P1 settings, K1+K2 comptime tree, P5 `project.rs` build drivers (-1,400) | L2-2 the `contracts/{order,locks,sharing,maps,throws,touch}.rs` crossings, L2-3 `contracts/{trust,sync,tether,keep,keeps}.rs` (-1,300) | #557 step 3 (`same_line`); #910, #907 |
| Fri 10-30 | 14,800 | 1,228 | S1 `sysroot` package, M3 `modules.rs` collect, `manifest.rs` (-1,350) | C1 `contracts/mod.rs` + `send.rs` + `ty.rs` (-1,350); parser chunk 1 written beside | **#920** (spawns); #908, #915; `scripts/bootstrap.nika` + `scripts/bootstrap-adapters.txt`, the CI step with `continue-on-error`; converter repairs as they come |
| **Sat 10-31** (checkpoint) | **12,100** | **1,228** | K3 `comptime_run.rs` rest, `build_time.rs`, `grammar_run.rs` (-1,300) | `proofs.rs`, `prove.rs`, `bounds.rs`, `bounds_report.rs`, `fold*.rs`, `dsl.rs` (-1,400); parser chunks 2-3 | **#922** steps 1-4 (sha256, `env`, `fs::lock`, file times); #909, #911 |
| Sun 11-01 | 9,000 | 530 | `describe.rs`, `foreign.rs`, `libraries.rs`, `cexport.rs`, `assets.rs`, `specbook.rs`, `fmt.rs`, `run_fast.rs` (-1,000); **O1** `orchestrator/cache.rs` (-700, #923) | parser chunks 4-5 and the switch, `grammar!` deleted (#557 step 5, -2,100) | hands the five lowering files to lane 2; #917, #918, #898, #900 |
| Mon 11-02 | 6,300 | 0 | `sysroot.rs` driver (`lower_tools_files_cached`' run), `project.rs` drivers (-1,300); **O2** `orchestrator/project.rs` (-530), the crate deleted | `text_tiers.rs`, `view_changes.rs`, `type_rules.rs`, `one_variant.rs`, `views.rs` (-1,400) | defects the moves meet; the spawns of the adapter rows that #920 freed |
| Tue 11-03 | 3,700 | 0 | `modules.rs` drivers, `diagnostics/mod.rs`, `main.rs` to the `clap` adapter (-1,300) | `check/*`, `emit/*` cores (`with_contracts`' run, `Emitter`, entry points, -1,300) | defects the last moves meet |
| **Wed 11-04** | **<= 2,950** | **0** | the rest of lane 1's rows over the adapter list | the rest of lane 2's rows | the CI step without `continue-on-error`; `promote` brings it to `main` |

(The compiler column carries R0's one-time +120 from Thursday on.)

**Variant B (D-1 not granted: two checkouts).** C3's work runs in C2 between
lane 2's briefs, and lane 1 takes nothing extra. Lane 2 loses about a day by
Saturday (section 5), so the compiler column slips by about 1,300 lines from
Saturday on unless the levers make it up; #920 and #922 run in C2 on Friday
and Saturday ahead of lane 2's briefs, since O1 and O2 (lane 1, Sunday and
Monday) wait for them.

That is about 2,710 lines a day against 2,387 measured with one lane: each
compiler move lane at about 1,300 a day, plus the orchestrator's 1,228 on
Sunday and Monday; with the levers, two to three slices of 400-600 lines a
lane and day. Slices are re-measured each morning by the architect
(`rs2nika --score` drafts), and the day's row is corrected here, not
elsewhere.

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

| Lane | Rust files owned (`crates/nikaia/src/`) | Nikaia files owned (`crates/nikaia-compiler/src/` after R0, `crates/nikaia-std/src/tools/` before) |
| :--- | :--- | :--- |
| 1 (moves) | `project.rs`, `comptime_run.rs`, `build_time.rs`, `grammar_run.rs`, `run_fast.rs`, `sysroot.rs`, `modules.rs`, `manifest.rs`, `describe.rs`, `foreign.rs`, `libraries.rs`, `cexport.rs`, `assets.rs`, `specbook.rs`, `fmt.rs`, `main.rs`, `diagnostics/mod.rs` | the ones its running brief names (`project_*`, `comptime_*`, `sysroot_*`, `modules.nika`, `manifest.nika`, `describe.nika`, `foreign.nika`, `libraries.nika`, `cexport*.nika`, `assets.nika`, `specbook.nika`, `fmt.nika`, new files) |
| 2 (moves) | `contracts/*`, `proofs.rs`, `prove.rs`, `bounds.rs`, `bounds_report.rs`, `fold.rs`, `fold_bound.rs`, `dsl.rs`, `check/*`, `emit/*`, `types.rs`, `traits.rs`, `ast/mod.rs`, every other file not named in this table; `parser/mod.rs` from #557 step 3 on; the five lowering files from Sunday 11-01 on | the ones its running brief names (`order`, `locks`, `sharing`, `maps`, `throws`, `touch`, `trust`, `sync`, `tether`, `keep*`, `ty`, `ledger_*`, `prove*`, `prover_*`, `bounds_*`, `fold*`, `dsl`, `parser_grammar`, new files) |
| Lowering | `text_tiers.rs`, `view_changes.rs`, `type_rules.rs`, `one_variant.rs`, `views.rs` until Sunday 11-01 | `emit_*` (not `emit_grammar`), `text_tiers`, `tiers`, `views`, `lends`, `type_rules`, `check_reads`, `check_facts`, `check_narrow` |
| Compiler | `parser/mod.rs` until #557 step 3 is pushed | `check_grammar`, `emit_grammar`, `grammar_decode`, `parse_*`, `parser_leaves`, `node_numbers`, the `check_*` file its bug names |
| Converter repair | `crates/rs2nika/` | none |
| Lane 1, from Sunday 11-01 | `crates/orchestrator/src/*` | `orchestrator_*` (new) |
| C3, new `std` entries (#920, #922) | `crates/nikaia-std/src/{process,hash,fs,time}.rs`, a new `env.rs`, their entries in `std.contracts` and Part III | none |
| R0 (#921), once, at a quiet moment | everything its brief names: no other brief runs | the whole package (a `git mv`) |
| nobody this week | every other file of `crates/nikaia-std` (point 3) | - |

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
blocks a briefed slice; (4) #920 then #922, the `std` entries O1 and O2 wait
for (done by Saturday 10-31 evening); (5) the lowering queue: #916 (absorbs
#913, #914), #910, #907, #908, #915, #911, #917, #918, #899, #896; (6)
compiler bugs that block a move: #909, #898, #900, #882. A bug that blocks
nothing waits until after 11-04 (the owner's *bugs before moving* default is
overridden by the deadline, as `docs/priorities.md` allows when the figures
say so: they say 2,710 lines a day).

### How the driver is kept off the lane briefs

`scripts/batch.nika` takes briefs only from `docs/briefs/batch.txt`, in its
order. The lane briefs are named `lane2-*`, `lane-lowering-*`,
`lane-bugs-*`, `lane-std-*` (the #920/#922 entries) and are never written into `batch.txt` (nor is `r0-compiler-crate.md`); the supervisor starts
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
  (1) the fix queue's items (5) and (6) that block no slice; (2) the
  `#[cfg(test)]` modules move as deletions where `tests/compiler` already
  covers the code, and new Nikaia tests only for what nothing covers; (3)
  style passes in moves (iterator house style, doc comment rewrites) become a
  later issue; (4) the slices are made larger (600-800 lines), the 30-minute
  limit of the driver permitting; (5) O2 and then O1 (#923) move to
  11-05/11-06, the orchestrator crate staying for those days (D-6); never R0
  and never the compiler's Stage 2; (6) only then a third move stream, which
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
* **The new `std` entries** (#920, #922) are additions: nothing a program
  already calls changes. A red test from one is reverted at once; O1/O2
  wait for it and slip a day before anything else does.
* **R0 meets a half-written `.nika`** in the old directory: it runs only at a
  quiet moment (no brief running in any checkout, all pushed); the other
  checkouts rebase onto it before their next brief, and git's rename
  detection carries an edit of a moved file. A brief written before R0 names
  the old path: the supervisor runs it after a `sed` of the path, not as is.
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
lane 2's briefs: about one day lost by Saturday (section 2, variant B). *Recommendation:* (a), for this week only.

**D-2. What "complete bootstrap" covers.** *Answered by the owner,
2026-10-28 (corrected the same day):* the compiler and `crates/orchestrator`
completely, the compiler's Nikaia in its own crate; `crates/nikaia-std` out of
scope (section 1, points 1-4).

**D-5 (dropped).** It asked which Rust `std` keeps; the owner took `std` out
of scope (point 3), so there is nothing left to confirm.

**D-6. What is cut first if Saturday is behind.** *What it is:* the
checkpoint of section 2 (compiler at 12,100, orchestrator still 1,228 by
Saturday evening, #920 and #922 pushed). *Facts:* S-B needs 2,710 lines a day,
1.13x the best day measured, with the parser and the `check`/`emit` cores
last (section 1a); the orchestrator is 1,228 of the 19,210 lines. *Options:*
(a) cut the process items first (section 5: fix-queue items that block
nothing, tests as deletions, style passes, larger slices), then the
orchestrator (O2, then O1, to 11-05/11-06), never R0 or the compiler's
Stage 2; (b) cut the orchestrator first, keeping the process items; (c) move
the deadline for all of S-B. *Recommendation:* (a): the process items cost
quality nobody sees on 11-04, the orchestrator is what the owner asked for by
name; the owner hears the Saturday figures that evening.

**D-7. May `std` gain new entries while nothing in it moves?** *What it
is:* the orchestrator calls six things Nikaia's `std` does not offer: a
child with the terminal (#920), sha256, environment variables, a file lock,
file times and file identity (#922). *Why it comes up:* the owner said both
"a missing `std` entry is not a reason, it is work to plan" and "`std`
stays Rust as it is". *Facts:* the entries are new thin wrappers (about 150
lines of Rust over `std::process`, `sha2`, `std::env`, `rustix::fs::flock`,
`std::fs::Metadata`), glue by design (point 1); nothing existing changes;
four specification amendments and version raises (the supervisor's).
*Options:* (a) add them to `std` (#920, #922, as filed): programs can use
them too; (b) put the same calls as host functions into the compiler's
adapter (`crates/nikaia`, about 150 lines on the adapter list, reason "std
frozen"): `std` untouched, programs gain nothing; (c) leave those orchestrator
parts in Rust (about 350 lines): against point 2. *Risk against the
recommendation:* a `std` change in the deadline week; it is additions only.
*Recommendation:* (a); C3 starts #920 on Friday unless the owner says (b).

**D-3 (only if #557 is late).** May the `grammar!` block count as a
`winnow-grammar` adapter at the deadline, the port following in the next
week? *Recommendation:* decide on Friday evening, from #557's state then.

**D-4 (superseded by variant A).** No third move stream is planned: C3's
free capacity after its queue goes to #920 and #922. A third stream needs
D-1, and nothing beyond it.
