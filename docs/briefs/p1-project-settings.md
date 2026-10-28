---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: P1, the build settings' words of `project.rs` into Nikaia (Part #125)

Score: self-hosting lane, bootstrap week lane 1 (`docs/bootstrap-week.md`, Thu 10-29 row: P1 settings, then K1+K2, P5). Plan row P1 (`docs/selfhosting-plan.md`), ~300 lines, draft share 59 % (the converter's *fewer arguments than holes* TODO in `choices`' 12-hole `format!`; under 60 % the driver prints `CONVERT` and goes on, the converter lane repairs beside you: rewrite that one call by hand). Runs after P4b, whose lines lie below these, so these ranges hold.

Cut: crates/nikaia/src/project.rs:171-302
Cut: crates/nikaia/src/project.rs:441-452
Cut: crates/nikaia/src/project.rs:563-625
Cut: crates/nikaia/src/project.rs:627-644
Convert: --profile check-free

The driver converts these before you start and names the draft (`.draft.nika`, `.notes`, `.rs`) in your prompt: start from it, do not cut or convert again.

## Facts
* Plan row P1 (copied): `refuted_claims`, `unproved_claims`, `optimizations`, `prove_through_loops`, `skip_free_at_exit`, `renamed`, `artifact`, `symbol_prefix`, `cpu_config`, `choices`' key and `panic_strategy`, `encoded_with_cpu` into a new `project_settings.nika` (prefix `project_settings_` / `ProjectSettings` where a name clashes): `ProjectWords` (the strings of `Settings`); `Settings` keeps `build: Build` (Rust, E12-4) and holds the words. `cpu_key` stays Rust (`sysroot::native_cpu`) and is passed in; `Build` is filled in Rust from the Nikaia `BoundsChecks`/`OverflowChecks`.
* Changed by the architect (2026-10-28): **`symbol_prefix` (304-322) stays Rust**: `std` has no `is_ascii_alphabetic` / `is_ascii_alphanumeric` (only `is_ascii_digit`; `is_alphanumeric` is Unicode and would change what ADR-284 D13 accepts). #924 moves it later. **`encoded_with_cpu`** (`project.rs` :3759 at 905bee7, below P4b's ranges: re-grep after P4b, 17 lines, with its doc comment) moves too, by hand, not in the Cut lines; its `#[cfg(test)]` test `cpu_comes_after_the_users_rustflags` (:3777) is moved to a Nikaia test (`tests/compiler/`, `.stdout` beside it, CLAUDE.md: new tests in Nikaia only) or, if the function is not reachable from a `.nika` test, kept calling the lowered function.
* Locations at 905bee7, `crates/nikaia/src/project.rs`: `Settings` :129, free functions :171-302 (`refuted_claims` :173, `unproved_claims` :186, `optimizations` :202, `prove_through_loops` :256, `skip_free_at_exit` :270, `renamed` :283, `artifact` :294), `impl Settings` :324 (`library` :326, `cpu_key` :433, `cpu_config` :444, `choices` :573, `panic_strategy` :632). Callers: :352-355, :366-368, :398-401, :477-479, :536-543 (the free functions), :1040-1041 and :2672 (`choices()`, `Choices::new(key, "rust")` stays Rust: Nikaia returns the key text), :2393/:2437/:2468 (`panic_strategy`), :2809 (`cpu_config`), tests :3929-3934 (`panic_strategy`).
* `BoundsChecks`, `OverflowChecks` (with `parse`, `name`) and `Target` (with `triple`, `is_bare_metal`) are already Nikaia's: `crates/nikaia-std/src/tools/emit_build.nika` :550 ff. and :105 ff.; `crate::bounds::*` in Rust are their lowered forms. `panic_strategy` and `optimizations` call them directly.
* `std` checked: `str::split`, `split_once`, `trim`, `split_whitespace`, `Vec::join`, `chars` exist. The `anyhow!` messages become a thrown `ProjectSettingsRefusal` (P4a's pattern: `ProjectMembersRefusal`, turned back into the same anyhow text in `project.rs`): every message stays byte for byte.
* Known open lowering defects, not yours: #909 (write `.clone()` where rustc asks), #916 (match on the field). Name each use in `Body:`.
* Do not change `crates/rs2nika/` (the converter lane's).

## To build
1. Move the cut functions (all but `symbol_prefix`) and `encoded_with_cpu` into `crates/nikaia-std/src/tools/project_settings.nika` with their doc comments; `Settings`' methods `cpu_config`, `choices`, `panic_strategy` become one-line Rust forwards to the lowered functions (or their callers call those directly: yours, say which in `Body:`); the moved Rust is deleted. `lower-std`: only `project_settings.rs` and its index lines change. Report the draft's `kept N of M` under `Rs2nika:`.
2. In `docs/selfhosting-plan.md` mark P1 built (`project_settings.nika`, `symbol_prefix` left for #924); add its row to `scripts/self_hosting.py`'s file table (beside `project_members.nika` :59); report the self-hosting % under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*, `CLAUDE.md`), except #909 and #916 above; if it is more than one step, stop and report it under `Needs:`.

## Done when
`nikaia --lib project::` passes; `it` tests filtered on `project:: c_library bounds_checks target_has simd assert` plus the guard set of `.claude/agents/builder.md` (not `nikaia_test::the_compiler_test_package_*`); `nikaia test --project tests/compiler` once, no `--both-settings` (no `check_*`/`emit_*` file changes, bootstrap-week L4c); clippy clean; lower-std fixpoint. Commit `project: the build settings' words into Nikaia (P1)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice beyond the ones left above; a `.stderr` or message that would change; a missing `std` call besides the two named; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p1-project-settings.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
