---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 10
---
# Brief: #1077, the tools preamble lacks the prelude's method traits

Blocks: k3b-comptime-bundle.md

Score: compiler bug, a lowering rustc refuses (E0599) in every `tools/*.nika` that calls a trait method of the prelude; it blocks the K3b move (`Duration.in_nanos()` on `fs::Metadata::modified()`). Small and traced.

Facts: Where things live, Traps already met, Commands

## Issue #1077 (summary)
A `tools/*.nika` call to a prelude method that lives on a trait (`d.in_nanos()`, `5.seconds()`, `b.since(a)`, `x.low_bits(n)`, a `ListExt` method) lowers to the bare method call; the tools package's preamble does not import the trait, so rustc gives E0599. A plain `nikaia run` program works: its prelude (`crates/nikaia-std/src/lib.rs` :720-737) brings `DurationExt`, `DurationIn`, `InstantSince`, `LowBits as _`, `ListExt`.

Probe: a tools function such as `pub fn stamp(p: fs::Path) -> i64 throws { fs::metadata(p).modified().in_nanos() }` in a `crates/nikaia-std/src/tools/*.nika`, `lower-std`, then `cargo check -p nikaia-std --features compiler`: E0599 `no method named in_nanos`. (Remove the probe function before committing.)

## Location (traced by reading, sure)
`crates/nikaia/src/sysroot.rs`, `lower_tools_files_cached`, the preamble at :768-775: it imports only `Full`, `panic`, `Bytes`, `ByteCopy as _`, `ByteReads as _`. Each `mod X_lowered` reaches them by its glob of the parent.

## To build
1. Reproduce the probe on the unfixed binary; keep the result under `Tests:`.
2. The preamble imports every method trait of the program prelude: `nikaia_std::time::{DurationExt as _, DurationIn as _, InstantSince as _}`, `nikaia_std::num::LowBits as _`, `nikaia_std::list::ListExt as _` (check each path against `lib.rs`), with a comment naming Part I 1.3 like the `Bytes` line. Name in `Body:` any other trait of `lib.rs`'s prelude you find the tools lack. Then `lower-std` (every lowered file changes only in what the preamble is: `lowered.rs`) and `cargo check -p nikaia-std --features compiler` with the probe function present, then without.
3. Test: a `tests/compiler/tests/tools_prelude_traits.nika` + `.stdout` that writes a one-file tools package calling `in_nanos()` and `since(..)` into a temp dir and has the toolchain lower and build it through `process::run` (as `a_trait_column_change_warns.nika` drives `nikaia build`), if a command lowers a tools dir that is not the sysroot. If none does, there is no Nikaia path to the preamble: commit step 2 alone, and say `Found: no Nikaia test reaches the tools preamble` (that is the defect to file, not yours to work around).
Commit `sysroot: the tools preamble brings the prelude's method traits (Fixes #1077)`.

## Done when
The probe compiles through `cargo check -p nikaia-std --features compiler`; `lower-std` fixpoint clean; the test (or the Found of step 3); build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line.

## Stop and report if
An edit is needed outside `crates/nikaia/src/sysroot.rs`, the lowered files `lower-std` rewrites, and the new test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b17-1077-tools-preamble-traits.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
