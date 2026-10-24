# A modular compiler: decision basis

The owner's question (2026-10-24): can the compiler be cut into parts, so that
a step rebuilds only the part it changed, prebuilt parts stay (dynamic
linking was the idea), and two builders can work at once?

## The problem

Every self-hosting step runs one cycle: edit a `tools/*.nika` file (and its
Rust adapter in `crates/nikaia`), build `nikaia`, `nikaia lower-std` to
regenerate `crates/nikaia-std/src/tools/lowered/*.rs` (was `package.rs`; split in 8dd21e75), build again, run the
touched tests. The cycle's length decides how fast the work goes, and its
shape decides whether two builders can run side by side.

## Today, checked in the code (develop 7439a1e7)

| Part | Size | Rebuilt by a one-file `.nika` step |
| :--- | ---: | :--- |
| `crates/nikaia/src` (Rust, one crate, 37 top modules) | 48 k lines | yes: it depends on `nikaia-std` |
| `crates/nikaia-std/src`, hand-written runtime | 25 k lines | yes, same crate as the package (incremental reuses most) |
| `tools/*.nika` (213 files, one namespace) | 80 k lines | the one file, through `lower-std` |
| `tools/package.rs`, their lowering | 73 k lines, 3.9 MB, **one module** | all of it, see below |
| `crates/nikaia/tests` | 267 `.rs` = 267 test binaries, 97 k lines | every one relinks |

Facts behind the table:

* **`package.rs` is one compilation unit.** rustc cuts a crate into codegen
  units along its modules; `codegen-units = 256` only merges, it never splits
  a module. All 3 300 functions sit in the root of `tools` (the per-file
  `pub mod x { pub use super::{...} }` are façades with no code), so the
  incremental session of `nikaia_std` (fast profile) has one object of 7.4 MB
  holding 1 832 `tools` functions, beside 186 others that together hold the
  other 6.9 MB. Any change to any `.nika` re-optimises that object at
  `opt-level = 3`, on one core. This is #667 step 4.
* **The Rust crate cannot be split along its modules.** Counting `crate::x`
  references, `parser -> check -> emit -> parser`, `contracts <-> emit`,
  `project <-> comptime_run`: every large module reaches every other.
  Crates must form a graph without cycles. And the Rust is leaving: 68.8 % of
  the toolchain is Nikaia (`scripts/self_hosting.py`), `crates/nikaia`
  becomes the adapter shell.
* **The `.nika` files cannot be split into crates either, yet.** A name-level
  approximation (a file depends on a file whose top-level name it calls or
  names) gives one strongly connected component of 143 files, 60 k of the
  80 k lines; `ast.nika` alone reaches `emit_state`, `ledger`, `views`. The
  approximation overcounts, but not by 140 files.
* **Test binaries.** Each of the 267 is 16-20 MB at the fast profile (it
  links the compiler and `std` statically): about 4.5 GB per profile, and 267
  links after every change. The disk running out (`CLAUDE.md`) is this.
  None of the 267 sets an environment variable or the working directory.
* **Two builders today collide on four things**: `package.rs` (every
  `lower-std` rewrites the whole file, so two branches always conflict in
  it); Cargo's lock on the shared `target/` (their builds queue); the disk
  (a second `target/` is another 1.4 GB plus test binaries); and the
  version lines (the supervisor's, already handled).

## How others do it

* **rustc** splits itself into ~70 crates (`rustc_parse`, `rustc_hir_typeck`,
  ...) along an acyclic graph, with traits and a query system to invert the
  edges that would be cycles; it still rebuilds every dependent crate when a
  crate's interface hash changes. Incremental compilation then reuses
  queries and codegen units inside each crate.
* **Dynamic linking in Rust** (`dylib`, `-C prefer-dynamic`): saves link
  time and disk, not compile time. Rust has no stable ABI; a dependent crate
  is recompiled whenever the dylib's metadata hash changes, as with an rlib.
  Bevy's `dynamic_linking` feature is exactly this trade: faster links only.
* **`cdylib` plugins** (rustc's codegen backends, `proc-macro`s) keep a part
  prebuilt behind a C ABI or a serialised boundary. That needs a stable
  interface (data passed as bytes or `extern "C"`), which the compiler's
  shared `Ast`/`Ty`/`Ledger` types are not.
* **One integration-test binary** (`tests/it/main.rs` with one module per
  file) is the common fix for many test binaries (rust-analyzer, cargo's own
  advice): one link instead of hundreds.
* **sccache** caches whole crate compilations by input hash; it does not
  cache incremental crates, so it helps CI and fresh worktrees, not the edit
  loop. **cargo-nextest** runs tests of all binaries in one parallel pool and
  splits a suite with `--partition count:k/n` across CI machines.

## Options

| Option | What it gives | Cost and risk |
| :--- | :--- | :--- |
| A. One Rust module per `.nika` in `package.rs` (#667 step 4): `mod x_body { use super::*; ... }` and `pub use x_body::*;` | a one-file step re-optimises one unit, not half the crate; 4 cores used | lowering in `sysroot.rs`; items another file reads must lower `pub(crate)`; names are already unique |
| A2. One generated file per `.nika` (`tools/lowered/x.rs`, `include!`d by A's modules) | `git` diffs and merges per file: two builders on two files no longer conflict in generated code | same place as A; CI's freshness check and `self_hosting.py` read the new layout |
| B. One integration-test binary | 267 links -> 1; ~4.5 GB -> ~20 MB per profile, so a second `target/` fits on disk | moves 267 files; `--test a` becomes a filter (`--test it a::`); tests now share a process |
| C. `nikaia-tools` crate beside `nikaia-std` | a program's `std` and the runtime never rebuild for a tools change | path `nikaia_std::tools` moves (ADR-290 D16, `use std::tools`); small gain once A is in |
| D. Split `crates/nikaia` into crates | nothing until the cycles are broken | breaking them is large work on code that is leaving: **not recommended** |
| E. `dylib` / `prefer-dynamic` | links shrink | B gives the same without ABI questions; no compile saved: **not recommended** |
| F. `cdylib` plugin boundary | a part truly prebuilt | needs a serialised interface over the shared types: **not recommended** now |
| G. nextest, sccache | parallel test pool, CI shards; cached cold builds | a tool to install in CI; no change to the edit loop |

## What to measure (counts, not time)

* **Codegen units re-optimised by a one-line change**: in
  `target/fast/incremental/nikaia_std-*/s-*/`, hash each `.o` before and
  after the change; count the changed ones and their bytes. Today: the
  7.4 MB unit for any `.nika` change. Same for `nikaia-*`.
* **Units Cargo rebuilds** (`cargo build --message-format json`, `"fresh":
  false`) and **link steps**, per kind of step: a `.nika` change, a Rust
  adapter change, a test change.
* **Bytes in `target/fast`** after `cargo test --profile fast --no-run`.
* **Instructions** of the `nikaia_std` rustc run where a counter is
  available (`perf_event_paranoid` is 2 here, so user-space counters may
  work with `RUSTC_BOOTSTRAP=1 -Z self-profile-counter=instructions:u`;
  for measurement only, never in a build file).
* **How many files a step touches**: from the last 20 self-hosting commits,
  `.nika` files, Rust files, test files per commit.

## Recommendation

1. **A now**, as #667 step 4, right after its per-file cache keys (step 2):
   the evidence is the single 7.4 MB unit. Keep it if the count of
   re-optimised bytes falls; it will, unless the lowering's `use super::*`
   makes rustc inline across modules into one unit again (measure).
2. **A2 with it or right after**: it is what makes two builders on two
   `.nika` files independent in generated code.
3. **B next**: the disk is the limit on a second `target/`, and every step
   pays 267 links.
4. **Stage 2 (two builders) after A2 and B**, each builder with its own
   worktree and `CARGO_TARGET_DIR`, on disjoint `.nika` files and disjoint
   Rust adapters. Still shared: `crates/nikaia/src/emit/mod.rs`,
   `check/mod.rs`, `project.rs` (the hubs; one builder at a time on each),
   and a change to a type or function another file uses (the #667 key on
   the package interface re-lowers the readers; the second builder rebases).
5. **C, G later**, D, E, F not: the Rust crate's cycles are not worth
   breaking while it shrinks, and dynamic linking saves only what B saves.
   Splitting the `.nika` package into crates waits until the files have
   layers (explicit `use` between tools modules); not decided here.
