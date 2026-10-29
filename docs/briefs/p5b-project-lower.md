---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: P5b, `lower_reading` of `project.rs` into a new `project_lower.nika` (Part #125)

Score: self-hosting lane, bootstrap week lane 1 (`docs/bootstrap-week.md`). Plan row P5b of `docs/selfhosting-plan.md`, *Large files 3* (draft 36 %; prerequisites M3b built in 093923c, P5d before it in this batch). S=2, C~4-6 M (comparable: M3b). Narrowed from the plan row: `lower`, `reads_for`, `reads_for_file`, `read_file`, `packages_of_file` are adapters already, `project_for_file`'s name rule needs `is_ascii_alphanumeric` (#924, after 11-04) and `foreign_library` needs `std_library` (Rust): all stay.

Cut: crates/nikaia/src/project.rs:730-1108
Convert: --profile check-free --prefix project_lower_
Shape: schematic host-calls (R47/R244 `[forms project_lower_]` rows and R295's host parameter, learned: P4b, P5d)

The driver converts this before you start. Ranges at 02b36d6; P5d (same file, lines below) runs first, so re-grep `^pub fn lower_reading(` and the `Ok(Lowered {` that ends it, and cut again if they moved.

**If `crates/nikaia-compiler/src` exists, R0 (#921) has run**: read `crates/nikaia-std/src/tools` as `crates/nikaia-compiler/src`, `nikaia_std::tools::` as `nikaia_compiler::`.

## Facts
* Read `docs/builder-facts.md` (*Host traits*: a `throws` host method, an anyhow error kept in a `RefCell` and handed back unchanged, as M3b's `ModulesEmitHost`, `modules.nika`).
* Already Nikaia (do not redo): `Ledger`, `Contract` (`requires`, `ensures`, `from`, `inferred`), `Published` (`prover_results.nika:85`, Rust `crate::prove::Published`), `changed_contracts` (`contract_changes.nika:17`, takes `mine` and `implies` callbacks; Rust `prove::changed_contracts` wraps it), `modules_program_of`/`modules_emit_reading` (M3b), `project_check_check` (P5d).
* **Stays Rust**: the `Lowered` struct and `lower_reading`'s signature (callers: `lower`, `project_for_file`, `main.rs`, `sysroot.rs`, `it` tests: `nikaia run scripts/where.nika -- lower_reading lower`); the `choices` assembly before the program is read (`Layout`, `reads_for`, `build_for_cpu`, `Choices`, the proofs digest: 737-765); `Program` (held by the host); `Cache`, `Artifacts`, `Book`, `with_book`, `cexport`; the export assembly after the call (1070-1094, the digest replaced in header, python, node).
* **Step 1, the draft's host calls**: add `[forms project_lower_]` and `[hosts project_lower_]` sections to `crates/rs2nika/convert/rs2nika.names` for the calls below, re-convert (`C=crates/rs2nika/convert/target/nikaia/bin/rs2nika; $C --cut crates/nikaia/src/project.rs:A-B > D.rs; $C --profile check-free --prefix project_lower_ --file D.rs > D.nika 2> D.notes`), build from that draft, `--score` against it.
* **`pub trait ProjectLowerHost`** (Rust impl `LowerHost` in `project.rs`, built after the `choices` with `layout`, `settings`, `reads`, `packages`, `input`, `no_cache`; `RefCell` for `program`, `cache`, `choices`, `exported`, `foreign`, `checked_facts`, `proofs`, `published`, and a kept `anyhow::Error`):
  - `fn read_program(ref self) throws` (`defaults_compiled_in` + the three `read_*`, `entry_shown_as`; then `choices.naming(shown_paths)`)
  - `fn open_cache(ref self) sync` (the `Cache::open` and its warning, unless `no_cache`)
  - `fn key_source(ref self) -> String sync`, `fn source_paths(ref self) -> Vec[String] sync`, `fn could_promise(ref self) -> String sync`, `fn dependencies(ref self) -> collections::BTreeSet[String] sync`
  - `fn committed_ledger(ref self) -> Ledger? sync` (in a project, `nikaia.contracts` read and parsed; `null` otherwise or on any failure)
  - `fn lost_sync(ref self, committed: ref Ledger) -> String sync`
  - `fn export(ref self) throws` (the `settings.library()` branch, kept for after the call)
  - `fn cached(ref self) -> ProjectLowerArtifacts? sync` (lookup + `has_all`), `fn record(ref self, made: ref ProjectLowerArtifacts) sync` (record + save, its warning)
  - `fn in_the_wrapper(ref self) -> bool sync`, `fn eprint(ref self, text: ref String) sync`
  - `fn check_units(ref self) -> String throws` (886-974: foreign, `newly`, `Around`, `promised`, the book, the per-unit `check`; returns the warnings, keeps book and facts)
  - `fn emit(ref self) -> String throws` (975-990: the second `with_book`, `described`, `checked_facts`, `emit_reading`; keeps `published` and the program, `proofs` in a project; returns the Rust)
  - `fn contracts(ref self) -> Ledger sync`, `fn published(ref self) -> collections::BTreeMap[String, Published] sync`, `fn render(ref self, ledger: ref Ledger) -> (String, String) sync` (`render`, `render_derived`)
  - `fn unused_reads(ref self) -> Vec[String] sync`
  - `fn changed_contracts(ref self, now: ref Ledger, committed: ref Ledger) -> String sync` (with the `dependencies` filter), `fn parse_ledger(ref self, text: ref String) -> Ledger? sync`
* Nikaia `project_lower_reading[H: ProjectLowerHost](host: ref H, in_project: bool) -> ProjectLowerOut throws` with `pub struct ProjectLowerArtifacts { rust: String, ledger: String, derived: String, warnings: String }` and `pub struct ProjectLowerOut { rust: String, ledger: String, derived: String, sources: Vec[String], reused: bool, notes: String }`: the order of the steps, the cache hit's replay, the published-contract merge (994-1015, over `Ledger` in Nikaia), the allowlist warning, the notes. Rust builds `Lowered` from it plus `proofs` and the export.
* Every warning, note and refusal byte for byte and in the same order on stderr (`tests/compiler` `.stderr`).
* Do not touch: `crates/rs2nika/**` except `rs2nika.names`, `contracts/*`, `check/*`, `emit/*` (lane 2), `modules.*`, `manifest.*`, `project_check.*` (P5d's), `docs/briefs/*` other than your report, and the uncommitted files `git status` shows at start.
* A new `tools/*.nika` file: one `lower-std` run with the old binary before `cargo build`, and a row in `scripts/self_hosting.py` (`builder-facts.md`, *A new `.nika`*).
* Disk: about 3.7 GB free at 02b36d6; `agent.nika` escalates under 3 GB. Copy `target/fast/nikaia` to your scratch dir before a long test run; right before it, `git status --short` the lowered files.
* Tests: `--lib project::`; `it` filters `project cache lower ledger prove proofs export library described foreign allowlist reads warnings` + guard set; `nikaia test --project tests/compiler`; `lower-std` fixpoint (only `lowered/project_lower.rs` and the index change).

## To build
1. The `rs2nika.names` rows, the host, the two structs and `project_lower_reading` into a new `project_lower.nika` with doc comments; `lower_reading` forwards. Commit `project: a lowering's steps, its cache and its ledger into Nikaia (P5b)`, `Part: #125`; plan row P5b updated (built, narrowed, what stays Rust), self-hosting share in the commit body.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p5b-project-lower.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
