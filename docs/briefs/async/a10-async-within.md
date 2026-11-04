---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---

# a10: `async(within: d)` and `h.join(within: d, on_timeout:)` (ADR-async-5 D16, D17; #986, #987)

Score: the language topics (owner, 2026-11-03), ADR-async-5 is the record. Rust is allowed (#1214) where the code is Rust; `.nika` stays Nikaia. Read ADR-async-5 D14, D16, D17 and the `NK2109` paragraph (`docs/specification/adr/adr-async-5.md`), and issues #986 and #987 for what is theirs.

Surface today: the parser refuses `async(within: …)` with "`within:` on `async` isn't built yet (#986)." (`crates/nikaia/src/parser/mod.rs`, the `OPT_WITHIN` arm). The runtime has `TaskHandle::join_within` (`crates/nikaia-std/src/task.rs`, from ADR-346) that always cancels on time-out; D17 says the default is `Keep`: the task runs on and the handle stays live. `task::TimedOut` exists.

Probe (refused today at 3:19):
```nika
fn slow() -> i64 { time::sleep(2.seconds()); return 1 }
fn main() {
    let a = async(within: 1.seconds()) slow()
    match a { ok(v) => print(v), err(e) => print("timed out") }
    let h = async(detached) slow()
    let r = h.join(within: 1.seconds(), on_timeout: task::OnTimeout::Cancel)
}
```
Write the probe in the form D16/D17 show (`catch` where the ADR uses it); where a form it needs (a `catch` after an `async` form, also refused as #986) is not built, that is the stop below, not a way around.

**Owns:** the parser's `within:` option on `async`, its checking and lowering, `task::OnTimeout` and `join(within:, on_timeout:)` in `std`, `NK2109`, their tests.

**Not yours:** `limit:` and `admission:` (D15's other options), `select`, keeps, views.

## To build
1. **D16**: `async(within: d)` on the value form, the handle forms and `async { }`: `d` counts from the form's start; on running out the task is cancelled as `h.cancel()` cancels it and ends with `task::TimedOut`, arriving where D16 says. Tests `tests/compiler/tests/async_within_*`: the value form timing out (the fallback stands in), finishing in time (the value), and a handle form joined after its deadline. Commit `Part of #986`.
2. **D17**: `task::OnTimeout` (`Keep | Cancel`, `Keep` by default) and `h.join(within: d, on_timeout:)` on `ChildHandle` and `TaskHandle`: by default the task keeps running and the handle stays live (joined again in the test); with `Cancel` the task is cancelled, its cleanups run, the handle is used up. `NK2109` for a `TaskHandle` left live on the `task::TimedOut` path as the ADR's paragraph says, with a refused and an accepted test (`async_join_within_*`). Commit `Part of #987`.

The existing always-cancel `join_within` callers in `crates/nikaia-std` and `packages/` keep their behaviour by passing `Cancel` where they relied on it; list them under `Body:`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: fix it in the compiler with a test as its own commit if it fits; otherwise file nothing and report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
The probe runs as D16/D17 say; all `async_*`, `task_*`, `spawn_*` and `overlap_*` tests in `tests/compiler` pass.

## Stop and report if
* a form D16 or D17 needs is not built and is not yours (`Result: blocked` or `partial`, `Found:` with a probe, `Needs: supervisor decision`);
* a program in `crates/nikaia-std`, `packages/` or `examples/` that compiles today is refused by `NK2109` (list them under `Found:`, do not change them).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a10-async-within.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
