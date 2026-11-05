---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---


# a14: `async(handle)` as `task::ChildHandle` (Part I 8.1.2, ADR-async-5 D11; #986)

Score: the language topics (owner, 2026-11-03), ADR-async-4/5 and Part I 8.1.2/8.2 are the record. Rust is allowed (#1214) where the code is Rust; `.nika` stays Nikaia. Read Part I 8.1.2 on handles (around line 3620 of `docs/specification/10-nikaia-light.md`: a child handle has `join()`, `join(within: d)` and `cancel()`, no `detach()`, is waited for at its group's end when left live, and leaving its group is `NK2122`), ADR-async-5 D11, issue #986 step 6, the a11 report and the a12 debrief (`a12-async-handle.debrief.md`) in this folder.

Surface today: `async(handle)` is not built. The fallback `catch` after `async(detached) fn { … }` is (15e380dd): `async(handle) fn { … } catch { … }` comes with this step the same way.

**Where an `async(…)` option goes** (the a12 run traced it; a12 ran out of turns before its first build, so nothing of it is on `develop`): the parser rule `async_expr` (`crates/nikaia/src/parser/mod.rs`); `ast.nika` and `node_numbers.nika`; the Spawn and AsyncGroup arms of `check_core_expr.nika`; `check_async_block` / `check_async_in` and `CheckAsyncWaitAt` with its number in `check_walks.nika`; `check_crossings_spawned` (`check_crossings.nika`); `emit_async_kind` and the Spawn arm of `emit_expr.nika`, `emit_async_task` (picks `start_here`, `start_paused` or `spawn_*`); the runtime in `crates/nikaia-std/src/task/` (`group.rs`, `ChildHandle`). `join(within:)` is keyed on `"TaskHandle"` in two predicates (`check_core_method`, `check_core_call_on`) and in the `std.contracts` keys. `ChildHandle::join` takes `&mut self` and its `held` is not a `Cell`. a12's unbuilt edits (213 lines, never compiled) are in `docs/briefs/async/a12-async-handle.partial.patch`: read it as a map, not as code that works; apply nothing blind.

**Build early:** build (`cargo build`, then `lower-std`) right after the parser and AST edit, before the checker; then after each further file. A run that ends without a build commits nothing.

**Owns:** `async(handle)` (parser, checking, lowering), `task::ChildHandle` in `std` with `join` and `cancel`, waiting for a live child handle at its group's end, `NK2122`, their tests.

**Not yours:** `join(within:)` on `ChildHandle` (a15), `within:` on `async(…)` (a13), `limit:`, `admission:`, `select`, views.

## To build
1. **`async(handle)`**: the form gives a `task::ChildHandle[T]` bound to its group: `join()`, `cancel()`, no `detach()`; a handle left live is waited for at the group's end; leaving its group (returned past it, stored in a value that outlives it) is `NK2122` (8.1.2, D11). Tests `tests/compiler/tests/async_handle_*`: joined, cancelled, left live (waited for at the group's end), with a fallback, and `NK2122` with its `.stderr`. Commit `Part of #986`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: fix it in the compiler with a test as its own commit if it fits; otherwise file nothing and report it under `Found:` with its probe. The `else => throw error` defect is #1242, not yours.

`Rs2nika: none`.

## Done when
The form runs as 8.1.2 and D11 say; all `async_*`, `task_*`, `spawn_*` and `overlap_*` tests in `tests/compiler` pass.

## Stop and report if
* a form `async(handle)` needs is not built and is not yours (`Result: blocked` or `partial`, `Found:` with a probe, `Needs: supervisor decision`);
* a program in `crates/nikaia-std`, `packages/` or `examples/` that compiles today is refused (list them under `Found:`, do not change them).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a14-async-handle.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
