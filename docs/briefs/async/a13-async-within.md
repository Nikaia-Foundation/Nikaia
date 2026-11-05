---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---


# a13: `async(within: d)` (ADR-async-5 D16; #986)

Score: the language topics (owner, 2026-11-03), ADR-async-4/5 and Part I 8.1.2 are the record. Rust is allowed (#1214) where the code is Rust (the parser's grammar block, the std runtime `task.rs`); `.nika` stays Nikaia. Read ADR-async-5 D16 and D22, Part I 8.1.2 from "A failure is held until it arrives" (`docs/specification/10-nikaia-light.md`, around line 3718), and the a11 report (`docs/briefs/async/a11-async-fallback.md.report.md`).

Surface today: the fallback `catch` after `let x = async e` and after `async(detached) fn { … }` is built (15e380dd). `TaskHandle::join_within` with `task::OnTimeout` is built (2b93405e). `async(within: d)` is refused (`async_parse_within_is_not_built_yet`). a11 found what D16 needs: a deadline in the runtime's paused and detached starts (`task.rs` `start_paused`, `ChildHandle::held`, `TaskHandle::start`), `task::TimedOut` in a detached task's channel and in the throws sets, and checking on the three forms.

**Owns:** `within:` on `async(…)` (the parser's `OPT_WITHIN` arm, the runtime deadline, checking, lowering), its tests.

**Not yours:** `async(handle)` and `ChildHandle`'s own API (a12; if a12 has committed it, the deadline goes there too), `limit:`, `admission:`, `select`, views.

## To build
1. **D16 on the value form and `async { }`**: `async(within: d) e` and `async(within: d) { … }`: `d` counts from the form's start; on running out the task is cancelled as `h.cancel()` cancels it and ends with `task::TimedOut`, arriving where D16 says (with a fallback, the fallback stands in). Tests `tests/compiler/tests/async_within_*`: the value form timing out (its fallback stands in), the value form finishing in time, `async { }` timing out. Replace `async_parse_within_is_not_built_yet`. Commit `Part of #986`.
2. **D16 on the handle forms built today**: `async(detached, within: d) fn { … }` (and `async(handle, within: d)` if a12's handle is committed): a handle joined after its deadline yields `task::TimedOut`; with a fallback, the fallback's value. Tests `async_within_detached_*`. Commit `Part of #986`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: fix it in the compiler with a test as its own commit if it fits; otherwise file nothing and report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
The forms above run as D16 says; all `async_*`, `task_*`, `spawn_*` and `overlap_*` tests in `tests/compiler` pass.

## Stop and report if
* a form D16 needs is not built and is not yours (`Result: blocked` or `partial`, `Found:` with a probe, `Needs: supervisor decision`);
* where `task::TimedOut` arrives needs a rule D16 and D22 do not give.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a13-async-within.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
