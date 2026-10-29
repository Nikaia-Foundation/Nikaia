---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: M3b, `Program::of` and `Program::emit_reading` of `modules.rs` into `modules.nika` (Part #125)

Score: self-hosting lane, bootstrap week lane 1 (`docs/bootstrap-week.md`). Plan row M3b of `docs/selfhosting-plan.md`, *Large files 3* (draft 100 %, prerequisite M3a built in 1890fc9). S=2, C~3-5 M (comparable: M3a, P4b). Runs after MF (`manifest.rs`); different files, both add to the index `lowered.rs`, so serial.

Cut: crates/nikaia/src/modules.rs:455-554
Cut: crates/nikaia/src/modules.rs:657-828
Convert: --profile check-free --prefix modules_

The driver converts this before you start (`.draft.nika`, `.notes`, `.rs`): start from it. Ranges at 06eaea6; if `git log` shows a later change to `modules.rs` (MF should not touch it), re-grep `fn of(` and `pub fn emit_reading(`. **The draft keeps method shape and Rust names** (m3a: `--prefix` was not applied to functions; P5a kept 6 of 104 lines by keeping methods): the target is free functions `modules_program_of` and `modules_emit_reading` over a host; keep the bodies, redo signatures and calls.

**If `crates/nikaia-compiler/src` exists, R0 (#921) has run**: read `crates/nikaia-std/src/tools` as `crates/nikaia-compiler/src`, `nikaia_std::tools::` as `nikaia_compiler::`.

## Facts
* Read `docs/builder-facts.md`.
* Already Nikaia (don't redo): `Ledger`, `Provenance` (`ty.nika`, re-exported by `crate::contracts`), `dependency_ledgers`, `absorb_own`, `shown_paths`, `modules_names_in`, `emit_needs_abort_table` (`emit_needs.nika`), the M3a hosts `ModulesHost`/`ModulesLedgerHost` (`modules.nika:663`, `:903`; follow their style: `ref self`, `sync`, unit by `i64` index).
* **Stays Rust**: `struct Program` (holds `Vec<Unit>`, `Checked`, `CheckFacts`, `Pause`), `TestCase`, `read`/`read_with`/`read_one`/`read_for_tests` (adapters, unchanged), `with_tests_as_functions` (needs `parse_to_ast`; already forwards), `shown_paths` .. `sources` (already forward), `std_ledger`, `names_in`, `described_beside`, `emit` (calls `emit_reading`). `emit_reading` and `of` keep their Rust signatures and become: build a host struct over `&self.units` (or `&mut units`), call the Nikaia function, assemble `Program`/`Lowered` from its answer and the host's kept state.
* **Host for `of`** (new `pub trait ModulesProgramHost` in `modules.nika`; Rust impl in `modules.rs` with `RefCell` state):
  - `fn drop_tests(ref self) sync` (the `retain` of `Item::Test`) and `fn across_files(ref self, from: i64, to: i64) sync` (`type_rules::across_files`); interior mutability over the units, or do both in Rust before the call (your choice, say which in `Body:`).
  - `fn package_ledger(ref self, from: i64, to: i64) -> Ledger sync` (the existing closure: dependency's own inference against `std_library()`).
  - `fn own_ledger(ref self, from: i64, to: i64, library: ref Ledger) -> Ledger sync`: calls `package_ledger_checked`, keeps `checked` and `noted` (by_code, why with `unit += from`) in host state.
  - Nikaia `modules_program_of(host, described: ref Ledger, packages: ref Vec[String?], renames: ..., reaches: ..., their_renames: ...) -> ModulesProgramParts` with `{ contracts: Ledger, as_its_own: BTreeMap[String, Ledger], own_checked_from: i64? }` (the group start whose `checked` is kept, `null` where `std_alone`/one-group test fails); Rust takes `own_checked`, `paused_by_code`, `pauses` from the host.
* **Host for `emit_reading`** (new `pub trait ModulesEmitHost`): `fn trust(ref self) -> Provenance sync` (the `proofs::with_units` + `analyse_package[_checked]` block), `fn keyed_functions(ref self, package: ref String) -> Vec[String] sync`, `fn own_names(ref self) -> collections::BTreeSet[String] sync` (`names_in(&package)`), `fn barrier_names(ref self, all: bool) -> collections::BTreeSet[String] sync`, `fn preamble(ref self) -> String sync` (the `Needs` fold and `.preamble()`), `fn emit_unit(ref self, at: i64, provenance: Provenance, offset: i64) -> String throws` (`proofs::for_unit` + `emit_module_body_at` + the `refusal_at` rendering; keeps `map.extend(body.map.placed(offset, at))` and `published` in host state, returns `body.rust`), `fn has_main(ref self) -> bool sync`, `fn map_rows(ref self) -> Vec[(i64, i64, i64)] sync`. Nikaia `modules_emit_reading(host, packages: ref Vec[String?], shown: ref Vec[String], sources: ref Vec[String]) -> String throws` holds the trust decision (keyed packages by last `::` segment, both barrier rules), the header lines, the loop skipping dependency units, and the abort table.
* Every refusal and generated text byte for byte (the header comment, `\n` placement).
* Callers (unchanged, signatures kept): `project.rs`, `main.rs`, `parser/mod.rs`, `it` tests `map_ledger`, `map_seeds`, `map_trust`, `package_graph`, `dependency_ledger`, `result_source`, `examples`, `modules`, `accepted_compiles`, `accepted_sweep`.
* Do not touch: `crates/rs2nika/**` (converter lane, uncommitted `convert/nikaia.*`), `contracts/*`, `check/*`, `emit/*` (lane 2), `manifest.rs`/`manifest.nika` (MF), `docs/briefs/*` other than your report.
* Copy `target/fast/nikaia` to your scratch dir before a long test run; right before it, `git status --short` the lowered files (another session may rewrite them: m3a lost a round).
* Tests: `it` filters `modules package map_ trust dependency result_source examples`; `nikaia test --project tests/compiler`; `lower-std` fixpoint. `--lib modules::` has no tests.

## To build
1. The two hosts and `modules_program_of`, `modules_emit_reading` into `modules.nika` with doc comments; `Program::of` and `Program::emit_reading` forward. `lower-std`: only `lowered/modules.rs` (and the index if a name is new) changes. Commit `modules: the program's ledgers and its emission into Nikaia (M3b)`, `Part: #125`; plan row M3b updated (built, what stays Rust), self-hosting share in the commit body.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/m3b-modules-program.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
