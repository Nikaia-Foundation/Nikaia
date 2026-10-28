---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: M3a, `modules.rs`' collection of units into `modules.nika` (Part #125)

Score: self-hosting lane, bootstrap week lane 1 (`docs/bootstrap-week.md`, Fri 10-30 row: M3 `modules.rs` collect, taken a day early as the plan's second row). Plan row M3a of `docs/selfhosting-plan.md`, *Large files 3: lane 1's drivers* (draft 95 %). S=2, C~3-5 M (comparable: M1, M2). Different file from P5a (`project.rs`), so the two do not shift each other's lines.

Cut: crates/nikaia/src/modules.rs:37-480
Cut: crates/nikaia/src/modules.rs:1066-1088
Convert: --profile check-free --prefix modules_

The driver converts these before you start and names the draft (`.draft.nika`, `.notes`, `.rs`) in your prompt: start from it, do not cut or convert again. Ranges are at a4b33a0; re-grep if `git log` shows a later change to `modules.rs`.

**If `crates/nikaia-compiler/src` exists, R0 (#921) has run**: read every `crates/nikaia-std/src/tools` below as `crates/nikaia-compiler/src`, and `nikaia_std::tools::` as `nikaia_compiler::`.

## Facts
* Plan row M3a (copied): `Unit::qualify`, `collect`, `collect_with`, `package_at`, `collect_one`, `Standing`, `read_unit`, `one_namespace`, `declared_once`, `declared_names`, `check_imports`, `sources_of`, `groups`, `packages_of`, `package_ledger*`, `shipped_ledger` (37-480), `foreign_beside`, `described_beside` (1066-1088) into `tools/modules.nika`, where `declared_names` :14, `a_use_refused` :32, `declared_once` :266, `groups` :289 already are (the Rust ones forward to them: no new Nikaia for those, only the loops around them).
* **`Unit` is not reshaped** (plan's known risk): it holds `Parsed` and is read about 150 times in about 30 files (25 of them `crates/nikaia/tests/it/`). Nikaia returns the file list, the packages and the refusals; Rust builds the `Unit`s from arena indices. `collect` has 630 call sites in 161 files: its Rust signature stays, so does `collect_with`'s and `collect_one`'s, as adapters.
* **Host callbacks** (the M2/K1 pattern; `Parsed` stays Rust until lane 2's parser rows): `parse_unit: fn(ref String, ref String, ref String?) -> i64 sync throws` (path, source, package: Rust runs `parser::parse_to_ast`, renders a refusal with `diagnostics::refused_finding`/`render_finding` exactly as today :194-200 of the cut, keeps the `Parsed` in an arena and returns its index); `infer_package` for `package_ledger*` (`contracts::Ledger::infer_package_noted`, lane 2's: a callback over indices, Rust keeps the `Noted`/`Checked` it returns in the arena beside the `Parsed`); `read_beside` (`contracts::Ledger::read_beside`, `shipped_ledger`, `described_beside`); `sha256_hex` (`sources_of`, Rust `orchestrator::cache::sha256_hex`, until #922 step 1); `manifest_of` for `foreign_beside` (`manifest::Manifest::read`, MF's row: returns the foreign names as `Vec[String]`). `std::fs::read`, `read_dir` and file tests are Nikaia's.
* **`read_dir`'s order**: Rust sorted `PathBuf`s (:111 of the cut); Nikaia's `read_dir` gives sorted names. Check both orders agree on names with `.`, `_` and digits, and say so in `Body:`; if they can differ, sort the way Rust did.
* Every refusal text and its path display stays byte for byte (`tests/compiler` `.stderr`; `crates/nikaia/tests/it/` tests on modules, packages, `use`).
* `std` has no `Seq.chain` (#924): write loops. Known open lowering defects, not yours: #909, #916.
* Do not touch `crates/rs2nika/` (the converter lane's uncommitted `convert/nikaia.{contracts,derived,proofs}`), `contracts/*`, `check/*`, `emit/*` (lane 2), nor `project.rs` (P5a is on develop, 2e13458; its rest waits on #933/#934), nor `tools/text_tiers.nika`/`lowered/text_tiers.rs` and the untracked `tests/compiler/tests/a_view_list_lent_to_a_ref_list_parameter_beside_its_own_text_is_refused.*` (other agents' uncommitted files; that test may fail `tests/compiler`: name it under `Tests:`).
* Tests that exercise this code: `it` tests filtered on `modules package use_ import namespace collect`; `--lib modules::`.

## To build
1. Start from the draft (its `kept N of M` under `Rs2nika:`). Write the functions into `tools/modules.nika` with doc comments, in the shape above; the Rust ones become adapters (signatures unchanged) or go where no Rust caller is left. `lower-std`: only `modules.rs` of `lowered/` (and the index if a name is new) changes. Every message byte for byte.
2. In `docs/selfhosting-plan.md` mark M3a built; report the self-hosting % (`python3 scripts/self_hosting.py`) under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*), except #909 and #916; if it is more than one step, stop and report it under `Needs:`.

## Done when
`nikaia test --project tests/compiler` once, no `--both-settings`; the `it` filters above plus the guard set of `.claude/agents/builder.md`, once, at the end, beside clippy; `--lib modules::` passes; clippy clean; lower-std fixpoint. Commit `modules: collecting the units into Nikaia (M3a)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice beyond the ones left above; a `.stderr` or message that would change; a missing `std` call other than the callbacks named; an edit needed in a lane 2 file; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/m3a-modules-collect.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
