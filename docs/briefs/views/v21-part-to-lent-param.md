---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---

# v21: a part of a list handed to a lent or `mut` `Array[T]` parameter (#1209, ADR-views-3 D11)

Score: ADR-views-3 D11 and Part I 2.x (the paragraph after "needs no word."). A lent `xs: Array[T]` is a view of a run, `mut xs: Array[T]` a changeable one; the caller hands an `Array[T, N]`, a `Vec[T]` or a piece `v[a..<b]` of either (of a `mut` place for `mut`: a `let mut`, a `mut` parameter, a field of `mut self`). Rust is allowed in the checker and the emitter for the views-3 lane (owner 2026-11-03, #1214; list it in `Body:`).

Facts: Checker and views, Lowering and lower-std, Tests and the gate

Today (`target/fast/nikaia` at f7fb224f):
```nika
fn sum(xs: Array[u32]) -> u32 {
    let mut s: u32 = 0
    for x in xs { s += x }
    return s
}

fn zero(mut xs: Array[u32]) {
    for i in 0..<xs.len() { xs[i] = 0 }
}

fn main() {
    let mut v: Vec[u32] = [1, 2, 3, 4]
    print(f"{sum(v[1..<4])} ")
    zero(v[1..<3])
    println(f"{sum(v[1..<3])} {sum(v)}")
}
```
Expected `9 0 5`; today `NK1102` on `sum(v[1..<4])` and on `zero(v[1..<3])` (the lent parameter is treated as keeping).

**Owns:** the checker's argument rule for an `Array[T]` parameter, its lowering (`&v[a..b]`, `&mut v[a..b]`), the prover's `len()` of a part, the new tests.

**Not yours:** the methods `sort`/`swap`/`fill`/`reverse`/`sort_by_key` (v22), the migration tool, `split_at_mut` (not decided: stop and report if you need it).

## To build
1. The read and the `mut` part argument: the probe above as `tests/compiler/tests/a_part_of_a_list_is_handed_to_a_lent_run.nika` with its `.stdout`, plus a `.stderr` case handing `v[0..<2]` of a `let` (not `mut`) list to `mut xs` (the refusal names the place that is not `mut`). Commit `Part of #1209`.
2. Prover: inside the callee, `xs[i]` with `i < xs.len()` stays proved when `xs` is a part (its `len()` is `b - a`); an out-of-range part `v[2..<9]` aborts at the call as an index does (a `.stderr` or abort test). Commit `Part of #1209`.

`Rs2nika: none`.

## Done when
Both tests pass at both settings; the checker and prover tests the change touches pass.

## Stop and report if
a part argument needs a new error code (the supervisor reserves it), or the prover cannot see a part's length without a change in how runs are modelled.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v21-part-to-lent-param.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
