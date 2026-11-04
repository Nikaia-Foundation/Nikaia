---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# a5: lower `async e`, `async e` as a statement and `async { }` onto the group runtime (#988 steps 1-3)

Score: the language topics (owner, 2026-11-03), ADR-async-5 is the record (#988's text still cites async-4/async-2). Rust is allowed here (#1214) where the lowering is Rust; where the code you change is `.nika` (`emit_*.nika`, `emit_core_args.nika`), it stays Nikaia. Read `docs/specification/adr/adr-async-5.md` D1, D6, D9, D10, D20, D21, ADR-292 D6 and Part II 12.7 first. S=1, C~3 M.

Surface today: a4 put one node for `async e` / `async fn { … }` and one for `async { … }` into the AST, with each task's wait point and group recorded by the checker, and the lowering refuses both as not built yet. The runtime is `crates/nikaia-std/src/task/group.rs` (6331c7ab): `Group`, `Scope::{handle, deferred}`, `PausedScope::*` are the entry points meant for this lowering (their doc comments); pool scopes have no handle entry yet (a3 deviation: build it here if a test needs it). A program test must exit 0; a secondary failure shows only in an uncaught error (Part I 7.1), so a case about secondaries is an `it` test on `process_exit` (see `SCOPE_JOINS`). Tasks cannot assign an outer `mut` (NK2107); a test that collects results uses the results or `SharedMut`.

**Owns:**
* The lowering of the two nodes (the emit files that write `spawn`/`task::scope` code today, Rust or `.nika`), the runtime's entry points if one is missing, the lowering's refusal a4 added (removed), and new tests under `tests/compiler/tests/async_*` (run tests with `.stdout`).

**Not yours:**
* The parser and checker (a4), handles and options (`handle`, `deferred`, `limit:`, `within:`, `admission:`), `par_iter`, `--overlaps`, touch sets.
* `contracts/sync.rs`, `keeps.rs` (another lane).

## To build
1. **Tasks and wait points** (#988 steps 1, 2): start each task where it is written, left to right; join it at the wait point a4 recorded (before the statement, the call or the operation); hold its outcome until then; the callee gets a plain `T`. A pausing task is polled by its group's future (`emit_core_args.nika`'s scope code is the start), a non-pausing one runs on the pool per ADR-292 D6's start order. Tests (both `user_parallelism` settings): a value waited for at first use with the window's statement printing first; `f(async g(), async h())`; `async a() + async b()`; `let (a, b) = (async g(), async h())`. Commit `Part of #988`.
2. **Groups** (#988 step 3, D9, D21): `async { }` and the innermost block join every unjoined task at their end, in written order, before an early exit (`return`, `?`, a throw) continues; a statement task's value is dropped; a failure nobody read is raised at the group's end as group.rs does for `task::scope`. Tests: a group waits on an early `return`; two statement tasks both finish before the line after `async { }`; an unread failure arrives at the group's end. Commit `Part of #988`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A form Nikaia, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*. A further defect: no second fix in this brief, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both steps hold with tests. `nikaia test tests/compiler --both-settings` gives every count as before plus the new tests; the `it` tests filtered on `task`, `overlap`, `spawn`, `scope`, `async`; `cargo test -p nikaia-std` for `task`; the guard set of `.claude/agents/builder.md`; the `lower-std` fixpoint if you changed a `.nika` the lowering reads. Clippy clean. No changelog line.

## Stop and report if
an existing `task::scope`, `spawn` or `overlap` program changes output; a4's recorded wait point or group is missing or wrong for a case (report the case, `Found:`, do not patch the checker).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a5-async-value-lowering.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
