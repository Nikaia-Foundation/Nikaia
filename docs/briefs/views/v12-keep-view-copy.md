---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---

# v12: a whole parameter handed back is a view; arithmetic on a lent `Copy` value is owned (#1239, #1240)

Score: the language topics (owner, 2026-11-03), ADR-views-3 and Part I, 6.5 are the record: "A whole parameter handed back is a view the result points into (6.6)", so it is not keeping; and an operator's result is a new value that points into nothing. Read issues #1239 and #1240 whole (each has its probe and its *To build*) and the report `v11-keep-cross-file.md.report.md` beside this brief (its `Found:` (1) and (2) are this brief). Code that is Rust is changed in Rust (#1214); where it is Nikaia, in Nikaia.

Probe 1 (#1239, `nikaia run`, gives NK2302 "`hand` keeps `x`", should print `3`):
```nika
struct Box[T] {
    item: T,
}

impl Box[T] {
    fn get(keep self) -> T {
        return self.item
    }
}

fn hand[T](x: T) -> T {
    return x
}

fn main() {
    let b = Box { item: 3 }
    println(f"{hand(b.get())}")
}
```

Probe 2 (#1240, `nikaia run`, gives NK1104 "returning `ref time::Duration`", should print `6`):
```nika
use std::time

trait Doubled { fn doubled(self) -> time::Duration }

impl Doubled for time::Duration {
    fn doubled(self) -> time::Duration { return self * 2 }
}

fn main() {
    println(f"{time::Duration::from_millis(3).doubled().as_millis()}")
}
```

Owns: the keep inference and the lowering of a lent operand in an operator, their tests, the spec baselines. Not yours: the use-after-`keep self` check (#1241), the spec page text.

## To build
1. **A whole parameter handed back is a view, not keeping** (#1239): probe 1 prints `3`; spec block #56 (Part I, 4.6) lowers again; regenerate `tests/specification/EXPECTED.txt` and `COMPILES.txt` (how: `docs/builder-facts.md`). Tests in `tests/compiler/`: the probe, and a function that takes a moving field out of its parameter and returns it, still NK2302. Commit `Fixes #1239`.
2. **An operator on a lent `Copy` operand gives an owned value** (#1240): probe 2 prints `6`; spec block #62 (Part I, 4.7) lowers again; regenerate the baselines. Tests in `tests/compiler/`: the probe, and the same for an integer field read through a lent `self`. Commit `Fixes #1240`.

List under `Body:` every Rust function added or changed, for #557 / #125.

A further defect: no third fix, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
both probes behave as said, blocks #56 and #62 compile in the regenerated baselines, the tests pass, the gate is green.

## Stop and report if
either fix needs a decision ADR-views-3 or Part I, 6.5-6.6 does not make.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v12-keep-view-copy.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
