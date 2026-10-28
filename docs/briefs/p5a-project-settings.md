---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: P5a, `Settings` and `Project`'s paths of `project.rs` into `project_settings.nika` (Part #125)

Score: self-hosting lane, bootstrap week lane 1 (`docs/bootstrap-week.md`, Thu 10-29 row: P5 `project.rs` build drivers). Plan row P5a of `docs/selfhosting-plan.md`, *Large files 3: lane 1's drivers* (first in its order; draft 87 %). S=2, C~3-5 M (comparable: P1, P4b). K1+K2 (7e3f8eb) on `develop`, self-hosting 80.9 %+.

Cut: crates/nikaia/src/project.rs:185-460
Cut: crates/nikaia/src/project.rs:1782-1929
Cut: crates/nikaia/src/project.rs:2740-2780
Cut: crates/nikaia/src/project.rs:3354-3392
Convert: --profile check-free --prefix project_settings_

The driver converts these before you start and names the draft (`.draft.nika`, `.notes`, `.rs`) in your prompt: start from it, do not cut or convert again. Ranges are at a4b33a0; `project.rs` is unchanged since (re-grep if `git log` shows a later change).

**If `crates/nikaia-compiler/src` exists, R0 (#921) has run**: read every `crates/nikaia-std/src/tools` below as `crates/nikaia-compiler/src`, and `nikaia_std::tools::` as `nikaia_compiler::`.

## Facts
* Plan row P5a (copied): `Settings`' `resolve`, `refutations`, `choose_cpu`, `cpu_key`, `cpu_config`, `cpu_rustflags`, `optimize`, `as_env`, `from_env`, `choices`, `panic_strategy` (185-460); `Project`'s `open`, `entry`, `build_dir`, `own_dir`, `gen_dir`, `switches_path`, `ledger_path`, `packages`, `members` (1782-1929); `program_name`, `root_tag`, `own_copy` (2740-2780); `switches_beside`, `trace` (3354-3392), into `tools/project_settings.nika` (`ProjectWords` :24, `optimizations` :88, `cpu_config` :153, `choices_key` :163, `panic_strategy` :188, `encoded_with_cpu` :205 are there from P1: the Rust `cpu_config`, `choices`, `panic_strategy`, `cpu_rustflags` already forward to them, so those need no new Nikaia).
* **Both structs stay Rust, not reshaped** (plan's known risk): `Settings` holds `build: Build` (`emit/mod.rs`, lane 2) and `Project`'s fields are read 86 times (`project.rs` 56, `main.rs` 18, `sysroot.rs` 8). So the Nikaia functions take and return words and paths, not the structs: free functions over `ProjectWords` and a new `ProjectPaths { root: String, variant: String }` (or what the fields of `build_dir`/`own_dir`/`gen_dir` need; name its shape in `Body:`); Rust keeps the methods as one-line adapters with unchanged signatures, so no caller changes (`resolve`, `choose_cpu`, `optimize` are called from `main.rs` only; `Project::open` from `main.rs` and `project.rs`). Rust still calls `Build::parse` on the words Nikaia returns.
* **Host callbacks** (`std` lacks them until #922 steps 1-2, which are not built): `env_var: fn(ref String) -> String? sync` (for `from_env`, `cpu_rustflags`' `CARGO_ENCODED_RUSTFLAGS`/`RUSTFLAGS`, `program_name`'s `CARGO_TARGET_DIR`, `trace`'s `TRACE_VAR`); `native_cpu: fn(ref String) -> String sync throws` (Rust `crate::sysroot::native_cpu`, `sysroot.rs` :182); `sha256_hex` (`root_tag`, Rust `orchestrator::cache::sha256_hex`); `hard_link: fn(ref String, ref String) -> bool sync` (`own_copy`; the copy fallback and `create_dir`/`remove` are `std::fs`). The precedent for a host trait with `sha256_hex` is `SysrootHost` (`tools/sysroot_cache.nika` :52) and P4b's (`tools/project_members.nika` :353): a trait or plain `fn` callbacks, your choice, say which in `Body:`. `cpu_levels` is `sysroot_cpu_levels` (`sysroot_cache.nika` :126). `as_env` returns `Vec[(String, String)]`; Rust turns it into `OsString`.
* `trace` appends a line to a file (`OpenOptions::append`): if `std::fs` has no append, `trace` stays Rust (8 lines) and you say so in `Body:`; do not add a `std` entry (std is out of scope this week, `docs/bootstrap-week.md` section 1).
* Every refusal text of `resolve`/`refutations`/`choose_cpu`/`optimize` stays byte for byte (`tests/compiler` `.stderr`, `crates/nikaia/tests/it/` tests on `--cpu`, `--optimize`, `--refuted-claims`).
* `std` has no `Seq.chain` (#924): write loops. Known open lowering defects, not yours: #909, #916 (name each use in `Body:`).
* Do not touch `crates/rs2nika/` (the converter lane's uncommitted `convert/nikaia.{contracts,derived,proofs}` are in the checkout) nor `emit/*`, `check/*`, `contracts/*` (lane 2).
* Tests that exercise this code: `it` tests filtered on `project cpu optimize settings switches`; `crates/nikaia/src/project.rs` `mod tests` (`cargo test --profile fast -p nikaia --lib project::`).

## To build
1. Start from the draft (its `kept N of M` under `Rs2nika:`). Write the functions into `tools/project_settings.nika` with doc comments, in the shape above; the Rust methods become one-line adapters or go where no Rust caller is left. `lower-std`: only `project_settings.rs` (and the index if a name is new) changes. Every message byte for byte.
2. In `docs/selfhosting-plan.md` mark P5a built; report the self-hosting % (`python3 scripts/self_hosting.py`) under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*), except #909 and #916; if it is more than one step, stop and report it under `Needs:`.

## Done when
`nikaia test --project tests/compiler` once, no `--both-settings`; the `it` filters above plus the guard set of `.claude/agents/builder.md`, once, at the end, beside clippy; `--lib project::` passes; clippy clean; lower-std fixpoint. Commit `project: settings and paths into Nikaia (P5a)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice beyond the ones left above; a `.stderr` or message that would change; a missing `std` call other than the four callbacks and `append`; an edit needed in a lane 2 file; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p5a-project-settings.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
