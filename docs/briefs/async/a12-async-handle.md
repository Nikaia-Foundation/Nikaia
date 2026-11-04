---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---


# a12: `async(handle)` as `task::ChildHandle`, then `join(within:, on_timeout:)` on it (Part I 8.1.2, 8.2, ADR-async-5 D17; #986, #987)

Score: the language topics (owner, 2026-11-03), ADR-async-4/5 and Part I 8.1.2/8.2 are the record. Rust is allowed (#1214) where the code is Rust; `.nika` stays Nikaia. Read Part I 8.1.2 on handles (around line 3620 of `docs/specification/10-nikaia-light.md`: a child handle has `join()`, `join(within: d)` and `cancel()`, no `detach()`, and is waited for at its group's end when left live), the 8.2 rules on bound handles and `NK2109`, ADR-async-5 D17, issue #986 step 6, and the a10 and a11 reports in this folder.

Surface today: `async(handle)` is not built (a10 report); `TaskHandle::join_within(on_timeout)` with `task::OnTimeout` and the `NK2109` Live path for `TaskHandle` are (2b93405e). If a11's fallback is on `develop`, `async(handle) fn { … } catch { … }` comes with step 1; if not, leave the fallback out of your tests and say so in the commit message.

**Owns:** `async(handle)` (parser, checking, lowering), `task::ChildHandle` in `std`, its `join`, `join(within:, on_timeout:)` and `cancel`, waiting for a live child handle at its group's end, `NK2109` for a child handle where 8.2 gives it, their tests.

**Not yours:** `within:` on `async(…)` and the fallback (a11), `limit:`, `admission:`, `select`, views.

## To build
1. **`async(handle)`**: the form gives a `task::ChildHandle[T]` bound to its group: `join()`, `cancel()`, no `detach()`; a handle left live is waited for at the group's end; it cannot leave its group (8.1.2). Tests `tests/compiler/tests/async_handle_*`: joined, cancelled, left live (waited for at the group's end), and refused where it would leave its group, with its `.stderr`. Commit `Part of #986`.
2. **D17 on `ChildHandle`**: `join(within: d, on_timeout:)` as on `TaskHandle`: `Keep` by default (the task runs on, the handle stays live, joined again in the test), `Cancel` cancels it and uses the handle up. Tests `async_join_within_child_*`. Commit `Part of #987`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: fix it in the compiler with a test as its own commit if it fits; otherwise file nothing and report it under `Found:` with its probe. The `else => throw error` defect of a10 is #1242, not yours.

`Rs2nika: none`.

## Done when
The forms above run as 8.1.2, 8.2 and D17 say; all `async_*`, `task_*`, `spawn_*` and `overlap_*` tests in `tests/compiler` pass.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a12-async-handle.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
