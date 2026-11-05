---
agent: builder
model: opus
max_turns: 60
budget_usd: 8
effort: medium
---


# a17: the checker refuses an integer where a `time::Duration` is expected (#1246)

Score: Part II (async, `select`, around line 1143 of `docs/specification/20-nikaia-advance.md`) is the record: `5.seconds()` is a `std::time::Duration`, made by a `std` extension on the integers; there is no suffix literal. An integer is never a Duration. `.nika` stays Nikaia; the checker is `crates/nikaia-std/src/tools/check_*.nika`.

**The defect** (a13 found it). Probe:

```
fn f(d: time::Duration) { println("ok") }
fn main() { f(20) }
```

`nikaia check` passes; `nikaia run` stops in rustc with *Mismatched types*. `async(within: 20) e` is accepted the same way. Both must be refused by the checker with its own argument or option mismatch code (the one it already gives for, say, a `String` passed as an `i64`), naming `time::Duration`; the help points to `.seconds()` / `.millis()`.

**Where it is**: find where an integer literal or integer value is taken as compatible with a `std` struct type. Read `check_types.nika` and the argument check first; a13 checks `within: d` against `time::Duration` in `check_core_async_deadline_fails`. Fix the compatibility rule, not each call site.

**Build early:** `cargo build`, then `lower-std`, after each file you change. A run that ends without a build commits nothing.

**Owns:** integer-to-`std`-struct compatibility in the checker, its tests.

**Not yours:** what `within:` does at run time (a13), handles (a16), views.

## To build
1. **The checker refuses an integer as a Duration**: the probe and `async(within: 20)` are refused with the existing mismatch code; `f(20.millis())` and `async(within: 20.millis())` still pass. Tests `tests/compiler/tests/a_duration_is_not_an_integer_*` (refused parameter, refused `within:`, accepted `.millis()`). Commit `Fixes #1246`.

A further defect: fix it in the compiler with a test as its own commit if it fits; otherwise file nothing and report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
The probe is refused by the checker; all `async_*`, `task_*`, `time_*` and `a_duration_*` tests in `tests/compiler` pass.

## Stop and report if
* a program in `crates/nikaia-std`, `packages/` or `examples/` that compiles today is refused (list them under `Found:`, do not change them);
* the rule turns out to be on purpose somewhere (an ADR allows it): `Result: blocked`, `Found:` the ADR, `Needs: supervisor decision`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a17-duration-integer.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
