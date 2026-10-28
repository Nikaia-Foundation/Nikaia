---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: MF, `manifest.rs`' `Manifest` and its readers into `manifest.nika` (Part #125)

Score: self-hosting lane, bootstrap week lane 1 (`docs/bootstrap-week.md`, Fri 10-30 row: `manifest.rs`). Plan row MF of `docs/selfhosting-plan.md`, *Large files 3* (draft 100 %, no prerequisite). S=2, C~3-5 M (comparable: P4a, M2). Runs after M3a (`modules.rs`); different files.

Cut: crates/nikaia/src/manifest.rs:20-358
Convert: --profile check-free --prefix manifest_

The driver converts this before you start and names the draft (`.draft.nika`, `.notes`, `.rs`): start from it, do not cut or convert again. **The draft writes `impl Manifest` methods: the target is free functions over a Nikaia struct; keep the bodies, redo the signatures** (P5a kept 6 of 104 lines by keeping the method shape). Range at 3ba472a; re-grep if `git log` shows a later change to `manifest.rs`.

**If `crates/nikaia-compiler/src` exists, R0 (#921) has run**: read `crates/nikaia-std/src/tools` as `crates/nikaia-compiler/src`, `nikaia_std::tools::` as `nikaia_compiler::`.

## Facts
* `manifest.nika` already holds every decision (`build_key` -> `Switch`, `codegen_table`, `codegen_key`, `codegen_units`, `dependency` -> `DependencyShape`, `crate_name`); its names are unprefixed, new ones take `manifest_`.
* **Shape** (plan row): a Nikaia `ManifestFile { build: Map[String, String], package: Map[String, String], codegen: Map[String, Map[String, String]], libraries: Map[String, String], dependencies: ..., notes: Vec[String], root: fs::Path? }`, every TOML value held as its text (`toml::Value::to_string`); a dependency as a record of its shape, its text (`type` removed for Rust) and its path. The `toml` crate stays Rust: Rust `Manifest::parse` walks `toml::Value` once and hands Nikaia plain words (keys, is-table, text, `as_integer`, `as_str`); Nikaia builds the `ManifestFile` and every refusal (the build-key loop, `libraries`' four refusals, `codegen`, `dependencies`' shapes).
* **`Target::parse` in `codegen`** (`emit/mod.rs`, lane 2's): pass its answer in as a `bool` from Rust, do not call it from Nikaia.
* **Readers keep their Rust signatures** as adapters (`root`, `notes`, `package_name`, `package_version` (`0.0.0` default in Nikaia), `dependencies`, `foreign_crates`, `libraries`, `codegen_for`, `setting`, `has_build_section`, `find`, `read`). Callers, all unchanged: `project.rs` (codegen_for 3, dependencies 4, package_name 8, `Manifest::read` 7, `find` 4, `Manifest {` 2), `describe.rs` (read 3, dependencies 2), `modules.rs` (1-2 each), `main.rs` (find), `foreign.rs`, `contracts/mod.rs` (`Dependency::`, lane 2: do not edit), `build.rs`, `it` tests `described`, `reentrancy_switch` (parse), `overflow`, `project`, `map_ledger`, `map_seeds`, `modules`, `package_graph`, `result_source`, `system_libraries`, `tests/common/mod.rs`; `tools/lowered/project_members.rs` builds a `Manifest {`. Rust keeps `enum Dependency` (matched in 5 files) and `codegen_for`'s `toml::Value`, rebuilt from the text in the adapter.
* `project.rs`' `rooted` stays Rust (a `toml::Value` in and out, `canonical`); not in this brief.
* Manifest's `mod tests` (16 `#[test]`, 359-599) stay Rust and keep passing unchanged (they call `Manifest::parse`, which stays). Every refusal text byte for byte.
* Do not touch: `crates/rs2nika/**`, `crates/nikaia-std/src/tools/text_tiers.nika` and `lowered/text_tiers.rs` (other lanes' uncommitted files), the untracked `tests/compiler/tests/a_view_list_lent_to_a_ref_list_parameter_beside_its_own_text_is_refused.*` (another agent's; it may fail `tests/compiler`: name it under `Tests:`), `contracts/*`, `check/*`, `emit/*`.
* Copy `target/fast/nikaia` to your scratch dir before a long test run: other agents rebuild it in the shared checkout (P5a lost two runs).
* Tests: `--lib manifest::`; `it` filters `manifest project describe package foreign reentrancy library codegen`; `nikaia test --project tests/compiler`; `lower-std` fixpoint.

## To build
1. `ManifestFile` and the parse logic into `manifest.nika` with doc comments; Rust `Manifest` holds a `ManifestFile` (plus the `Dependency` map it derives) and its methods forward. `lower-std`: only `lowered/manifest.rs` (and the index if a name is new) changes. Commit `manifest: Manifest and its readers into Nikaia (MF)`, `Part: #125`; plan row MF updated (built, what stays Rust).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/mf-manifest-file.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
