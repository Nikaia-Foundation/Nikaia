---
agent: builder
model: opus
max_turns: 70
budget_usd: 10
effort: medium
---

# v14: a cloned parameter is a fresh value, not a keep (#1243)

Score: the language topics (owner, 2026-11-03). Read issue #1243 whole and the report `v13-view-result.md.report.md` beside this brief (its `Found:` is this brief). Code that is Rust is changed in Rust (#1214); where it is Nikaia, in Nikaia.

Supervisor decision: a defect. ADR-views-3 D12 and D8 say a result keeps or views only what its ways out reach; `x.clone()` returns an owned value that reaches nothing of `x`. So the keep inference and the D12 hand-back (b28ac489) treat the result of `clone()`, and of any call whose result type holds no view of its receiver or arguments, as fresh. `NK1126` on an unbounded `T` stays (no `Clone` bound); with `T: Clone` the program is accepted.

Probe (in a keep file; today NK2302 "`id` keeps `x`"; with the bound it should print `3`):
```nika
fn id[T: Clone](x: T) -> T {
    return x.clone()
}

fn main() {
    let a = 3
    let b = id(a)
    println(f"{b}")
}
```

Owns: the keep inference, the D12 hand-back in the checker, their tests. Not yours: the use-after-`keep self` check (#1241), the spec page text.

## To build
1. **A cloned or freshly made result is not a keep**: the probe prints `3`; without the bound only `NK1126` is given, no `NK2302`; a function that hands back the parameter itself is still a view (v13's tests stay green). Tests in `tests/compiler/`: the probe, the unbounded case (`.stderr`), and a keep file where `clone()` of a lent field is returned. Commit `Fixes #1243`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v14-clone-not-keep.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
