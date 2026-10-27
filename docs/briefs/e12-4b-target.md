---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: E12-4b, `Target` and the two check switches into Nikaia (Part #125)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: moves first; #125, `docs/selfhosting-plan.md` *C-7 rest and E12*, row E12-4, the last E12 row). S=2 (a slice, ~250 Rust lines), C~3-5 M (comparable: e12-4a). Architect's decision (2026-10-27, after d6318de): **`Build` stays Rust** for now. Its `Default` names `Target::default()`, which is `Target::host()` = `env!("NIKAIA_HOST_TRIPLE")`, a value of the compiler's own build, not a build-time value a Nikaia field default (ADR-331 D5) can hold; and `impl Default` for a type of `nikaia_std` cannot be written in `crates/nikaia` (orphan rule). So `e12-4b0` (lowered `impl Default`) is not needed by this move and waits.

## Facts
* `crates/nikaia/src/emit/mod.rs`: `enum Target` (:51, derives `Debug, Clone, Copy, PartialEq, Eq`; variants `X86_64Linux`, `Aarch64Linux`, `Wasm32Unknown`, `Rust(&'static str)`), `impl Default for Target` (:61, stays Rust as a free `fn`, see step 1), `impl Target` (:218-345: `parse` -> `Result`, `is_bare_metal`, `name`, `triple`, `host`, `has_threads`, `has_runtime`, `unbuildable` -> `Option<&'static str>`). 8 `Target::Rust(` sites in `crates/`.
* `crates/nikaia/src/bounds.rs`: `BoundsChecks` (:43), `OverflowChecks` (:72), each `Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord` with `#[default]`, `parse(word) -> Option`, `name() -> &'static str`; 28 uses in `crates/nikaia/src`.
* `crates/nikaia-std/src/tools/emit_build.nika` (426 lines, from E12-4a): `UserParallelism`, `ReentrancyCheck`, `emit_build_rust_targets() -> Vec[ref String]`, lowered to `Vec<&'static str>`; `Target::parse` reads it for `Target::Rust(known)`. How 4a kept the `Default` of its enums (a hand `impl Default for Build` in Rust) is the model.
* Errors: `parse` `throws` in Nikaia with the `refused!` text byte for byte; the Rust adapter maps it back (`fn switch`, emit/mod.rs:135).

## To build
1. `Target` into `emit_build.nika` with `parse`, `is_bare_metal`, `name`, `triple`, `has_threads`, `has_runtime`, `unbuildable`; `emit` re-exports it (`pub use`, as 4a did). `Target::Rust` keeps a `'static` triple and `Copy`: probe first how `Rust(ref String)` lowers; if it does not lower to `Rust(&'static str)` without a lifetime parameter on `Target`, carry an index into `emit_build_rust_targets()` instead and let `triple`/`name` read the list (your choice, say which in `Body:`). `host()` and the `Default` stay in Rust as free functions in `emit/mod.rs` (`host_target()`, and `Build::default()` calls it); the lowered `Target` gets no `Default`, and callers of `Target::default()` / `Target::host()` (grep them) call the free functions. A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*).
2. `BoundsChecks` and `OverflowChecks` with `parse`/`name` into `emit_build.nika`; `bounds.rs` re-exports them. Their `Default`: as 4a did for its enums (callers name the variant; `Build`'s hand `impl Default` already does). If `PartialOrd`/`Ord` are used (grep `<`, `max`, `cmp` on them) and the lowering does not derive them for a data-less enum, that is a compiler step: stop and report it under `Needs:` rather than writing the comparison by hand.
3. `lower-std`: only `emit_build`'s lowered file and the index line change. Run the converter on the moved Rust first (`rs2nika --profile emit --file` on a file holding the Rust you move) and report `--score` under `Rs2nika:`. Update `docs/selfhosting-plan.md` row E12-4 (4b built; `Build` stays Rust and why, one line) and run `python3 scripts/self_hosting.py` (report the % under `Tests:`).

## Done when
`it` tests filtered on `target parallelism bounds overflow optimization`; `nikaia --lib emit:: bounds::`; `nikaia test tests/compiler --both-settings`; the guard set of `.claude/agents/builder.md`; clippy clean. Commits `emit: the build target into Nikaia (E12-4b)` and `emit: the bounds and overflow switches into Nikaia (E12-4b)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice beyond the one left to you in step 1; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/e12-4b-target.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
