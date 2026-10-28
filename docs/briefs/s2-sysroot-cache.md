---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: S2, the paths and caches of `sysroot.rs` into Nikaia (Part #125)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: moves first, largest Rust files first; #125, `docs/selfhosting-plan.md` *Large files 2*, row S2). S=2 (~445 Rust lines, converter draft 90%; also builds the `std::fs` bytes probe P3 and P4 wait for), C~4-6 M (comparable: P2, e51e3e2).

## Facts
* Plan row S2 (copied): `sysroot.rs`: `Sysroot` paths, `std_modules`, `tool_modules`, `nika_in`, `cpu_levels` and the parsing half of `native_cpu`, `tools_sources`, `Package` with `committed`, `inlined`, `contains`, `drift`, `stale`, `pack`, `unpack`, `LoweredBefore` `key`, `get`, `put`, the `interface`/`file_key` keys of `lower_tools_files_cached`. Target: new `crates/nikaia-std/src/tools/sysroot_cache.nika` (`std::fs` reads, writes, `read_dir`, as `sources.nika`). Host added: `sha256_hex(ref Vec[u8]) -> String` (`std` has no SHA-256), `rustc_cpus(triple, args) -> String throws`, `use_entry(dir)` and `user_cache_dir()` (`orchestrator::cache`), `unique_beside(path)` (`comptime_run`); the `ASKED` memo of `native_cpu` stays Rust around the Nikaia parse.
* Locations at fa57fcc, `crates/nikaia/src/sysroot.rs`: `struct Sysroot` :90, `std_modules` :135, `tool_modules` :162, `nika_in` :171, `cpu_levels` :227, `native_cpu` :250, `tools_sources` :460, `lower_tools_files_cached` :544 (`interface` :599-619, `file_key` :620-627), `struct Package` :783-878, `struct LoweredBefore` :1331, `key` :1373, `get` :1390, `put` :1398, `mod tests` :1420 on (Rust tests of `LoweredBefore`: they keep calling the Rust wrappers and stay passing; they are the key-stability check).
* Prerequisite (plan): a probe that `std::fs` in `tools/*.nika` reads and writes bytes. Build it here first: a small `tests/compiler` program, and if it fails, that is a compiler/`std` fix as its own commit before the move.
* **Cache keys do not change.** The keys hash Rust `Debug` text (`{build:?}{provenance:?}{facts_reusable}`, `{own:?}`): Rust computes those strings and passes them in as `String`; Nikaia builds the same bytes in the same order (`part`: u64 little-endian length, then the bytes; `key`: parts ended by `0`). The draft wrote `interface` as one line of closures over a `mut Vec[u8]`: write a nested or module `fn` instead. `without_bodies` is S1's and stays Rust: pass its results in.
* Stays Rust (plan): `lower_std*`, `lower_tools*` runs, `in_parallel`, `Phases`, `Sysroot::resolve`, `rlib_cache`, `COMPILER_IDENTITY`, `rustc_cpus`, `tools_build`, `impl Display for Package`, `mod tests`.

## To build
1. The `std::fs` bytes probe (above), committed as its own test.
2. Run the converter on a scratch file holding exactly the moved Rust (`rs2nika --profile check --file`), report `--score` under `Rs2nika:`. Move the listed code into `sysroot_cache.nika` (prefix `sysroot_` where `where.nika` finds a clash), with the doc comments. `sysroot.rs` calls the lowered functions; the moved Rust is deleted. `lower-std`: only the new lowered file and its index lines change; then a second `lower-std` reuses every file from the cache (report `reused` under `Tests:`).
3. Add `sysroot_cache.nika`'s row to `scripts/self_hosting.py` (as `project_check.nika`'s), mark S2 built in the plan's table (one line); report the self-hosting % under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*, `CLAUDE.md`); if it is more than one step, stop and report it under `Needs:`.

## Done when
`nikaia test tests/compiler --both-settings`; `cargo test -p nikaia --lib sysroot::`; `it` tests filtered on `sysroot lower_std native cpu package drift`; the guard set of `.claude/agents/builder.md`; clippy clean; lower-std fixpoint. Commit `sysroot: paths, package and cache keys into Nikaia (S2)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice; a cache key that would change; a `.stderr` that would change; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/s2-sysroot-cache.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
