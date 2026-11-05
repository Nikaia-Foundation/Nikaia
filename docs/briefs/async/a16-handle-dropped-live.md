---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---


# a16: a live `ChildHandle` dropped unjoined is waited for at its group's end (Part I 8.2, ADR-async-5 D11; #987)

Score: Part I 8.1.2 and 8.2 and ADR-async-5 are the record. Rust is allowed (#1214) where the code is Rust; `.nika` stays Nikaia. Read Part I 8.2 on bound handles (around line 3798 of `docs/specification/10-nikaia-light.md`), 8.1.2 on a group's end (around line 3620–3730), ADR-async-5 D11, and the a14 and a15 reports in this folder.

**The defect** (a15 found it; the supervisor decided it is a defect, not a spec question): a pausing `async(handle)` task whose handle is dropped while the task still runs (here after a `Keep` time-out the handler does not take up) is cancelled at its pause. 8.2 says a live child handle is waited for at its group's end. Probe:

```
fn slow() -> i64 { time::sleep(100.millis()); println("end"); return 7 }
fn main() { let h = async(handle) fn { slow() }; let v = h.join(within: 10.millis()) catch { 0 }; println(f"{v}") }
```

prints `0` without `end`; it must print `0`, then `end` (the group of `main` waits for the rest at its end; the value 7 is nobody's and is dropped; a failure of the rest arrives as 8.1.2 says for an unjoined child).

**Where it is**: `ChildHandle::held` and `cancel_here` / the `Drop` of `ChildHandle` in `crates/nikaia-std/src/task/group.rs` (a14). The rest is a `Pin<Box<dyn Future + 'g>>` that borrows the group, so the group cannot own it (self-reference). One way, yours to judge: a store of strays the emitted code declares after the group and polls together with the group's end (`emit_async_task` / the group's end in `emit_expr.nika` and the runtime); the handle's drop moves its live rest there instead of cancelling it. Choose the form that fits the runtime; no new heap or `Arc` beyond the `Box` the rest already has, or name it under `Found:` for a cleanup issue.

**Build early:** `cargo build`, then `lower-std`, after each file you change. A run that ends without a build commits nothing.

**Owns:** what a dropped live `ChildHandle` does (runtime and, if needed, lowering), its tests.

**Not yours:** `join(within:)` itself (a15), `within:` on `async(…)` (a13), `limit:`, `admission:`, `select`, views.

## To build
1. **A dropped live child handle is waited for**: the probe prints `0` then `end`; an explicit `cancel()` and `on_timeout: Cancel` still cancel at the pause; a failure of the dropped rest arrives at the group's end as 8.1.2 says. Tests `tests/compiler/tests/async_handle_dropped_live_*` (the probe; one with a failing rest). Commit `Part of #987`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: fix it in the compiler with a test as its own commit if it fits; otherwise file nothing and report it under `Found:` with its probe. The `else => throw error` defect is #1242, not yours.

`Rs2nika: none`.

## Done when
The probe behaves as 8.2 says; all `async_*`, `task_*`, `spawn_*` and `overlap_*` tests in `tests/compiler` pass.

## Stop and report if
* a form it needs is not built and is not yours (`Result: blocked` or `partial`, `Found:` with a probe, `Needs: supervisor decision`);
* a program in `crates/nikaia-std`, `packages/` or `examples/` that compiles today is refused (list them under `Found:`, do not change them).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a16-handle-dropped-live.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
