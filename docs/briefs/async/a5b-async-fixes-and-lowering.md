---
agent: builder
model: opus
max_turns: 120
budget_usd: 22
effort: medium
---

# a5b: fix the two defects a5 met, then finish a5 from the tree (#988 steps 1-3)

Score: the language topics (owner, 2026-11-03), ADR-async-5 is the record. Rust is allowed (#1214) where the code is Rust; `.nika` stays Nikaia. a5 (`a5-async-value-lowering.md`, read it whole: its *To build*, tests and *Done when* are yours) ran out of turns twice. Read its retrospective `a5-async-value-lowering.debrief.md` before anything else.

Surface today: a5 left its work **uncommitted in the shared checkout**: `crates/nikaia-std/src/task.rs`, `task/group.rs`, and `emit_core_args`, `emit_expr`, `emit_function`, `emit_function_body`, `emit_state` and `emit_stmt` (`.nika` + `lowered/*.rs`). `git diff` on those files is your starting point. Steps 1 and 2 of a5 are built there and their probes pass, except the unread-failure test. Its 9 test programs were run from a scratchpad copy that is gone, so you write them again (5 new, 4 converted from a4's `tests/compiler/tests/async_*`).

Facts (from a5's retrospective):
* An async task needs a channel with `From<Crashed>` + `Joined`. Only `nikaia_std::task::Crashed`, Box, and generated throw sums `__NikaiaThrows_…` that hold `task::Crashed` have them. `emit_sums.nika` writes those impls only when `task::Crashed` is in the throw set. `emit_call_tasks_channel` (`emit_core_args.nika`) picks the channel.
* The checker puts `task::Crashed` into an inferred throw set for `task::scope`, but **not for `async`**. That is defect 1.
* The parser refuses a statement after `async { … }` used as a statement. Probe: `fn main() { async { async work(1) } println("x") }`. That is defect 2.
* `CheckAsyncWait`: the group id is encoded as `-1 - stmts[0].span.start`. `id.get()` is u32 and the facts are keyed i64.
* The pool at `user_parallelism: yes` is not built, so both settings run in sequence. Say so in the commit message, not as a deviation.
* `Ok({ 4 })` trips `unused_braces`.
* A test file inside the project cannot be run with `nikaia run`. Run it with `nikaia test tests/compiler --filter <name>`.

**Owns:**
* Everything a5 owns.
* For the two defects only: the checker's throw-set inference and the parser's statement rule for `async { }`.

**Not yours:** as in a5.

## To build
1. **The two defects** (each its own commit, each with a test under `tests/compiler/tests/async_*`, shown failing on the binary before your fix):
   (a) a function whose body starts async tasks gets `task::Crashed` in its inferred throw set, as `task::scope` does (Part II 12.7, ADR-async-5 D10);
   (b) `async { … }` as a statement ends the statement like any block statement, so the next statement needs no `;`.
   Commit `Part of #988`.
2. **Finish a5**: complete a5's steps 1 and 2 on top of the tree's diff and fix 1, with all of a5's tests including the unread failure arriving at the group's end. Run a5's verification list. Commit `Part of #988` (one or two commits).

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: no third fix, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
As a5's *Done when*, plus the two defect tests.

## Stop and report if
an existing `task::scope`, `spawn` or `overlap` program changes output; fix 1 changes an existing program's throw set beyond adding `task::Crashed` where async tasks start.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a5b-async-fixes-and-lowering.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
