---
agent: builder
model: opus
max_turns: 80
budget_usd: 12
effort: medium
---

> **Retired (2026-11-05):** the builder crashed without a report; its work is in `v18-matched-lent-param.partial.patch`, the account in `v18-matched-lent-param.debrief.md`. Split into `v18a-method-param-lent.md` and `v18b-matched-lent-param.md`.

# v18: a lent parameter a `match` takes apart stays lent; a lent `String` argument passes as `&str` (#1213, found by v17)

Score: ADR-views-3 (D1: a plain parameter is lent, D4: `ref` leaves the language outside `extern`). Read `v17-migrate-docs-packages.md.report.md` beside this brief, `Found:` (1) and (4).

Facts: Checker and views, Lowering and lower-std, Tests and the gate

**(1) Probe** (refused NK2105 today; checks with `value: ref Json`; must check and print `2` twice without `ref`):
```nika
enum Json {
    Text(String),
    Array(Vec[Json]),
    Num(i64),
}

fn longest(value: Json) -> i64 {
    match value {
        Json::Array(items) => return items.len() as i64
        else => return 0
    }
}

fn main() {
    let j = Json::Array([Json::Text("ab"), Json::Num(3)])
    println(longest(j))
    println(longest(j))
}
```
Cause (v17): `keeps.nika` `Expr::Match` counts a parameter a `match` takes apart as kept unless it is in `context.lent` (the #173 rule, written when a plain parameter was by value). Under views-3 a plain parameter is lent: a `match` that only reads it (binds by view, `items.len()`) keeps nothing. A match arm that moves a payload out (returns `items`) still keeps.

**(4) Probe**: `examples/escaping` with its `ref` dropped: `built.push_str(html::escape(name))` fails in rustc with *Mismatched types*. A lent `String` parameter passed on to a lent `str`/`String` parameter lowers to the borrow, as it did with `ref`.

**Owns:** `crates/nikaia-std/src/tools/keeps.nika` (and the checker files the rule needs), the lowering file the call needs, the lowered files `lower-std` changes, the new tests in `tests/compiler/`, `examples/escaping` (drop its `ref` once it works).

**Not yours:** the migration tool (`migrate_views3.nika`, `migrate.rs`: v19), the rest of the examples and tests (a later brief reruns v17).

## To build
1. **(1)**: the keep rule for a matched lent parameter, with the probe as a `tests/compiler/` test (and one where an arm moves the payload out and stays kept). The keep column of std's and the tools' matched parameters changes: re-lower, rebuild to the fixpoint, commit every lowered file. Commit `Part of #1213`.
2. **(4)**: the lowering of a lent `String` argument at a lent `str` parameter, with a test; then drop the `ref` in `examples/escaping` and its tests pass. Commit `Part of #1213`.

`Rs2nika: none`.

## Done when
Both probes check and run without `ref`; the touched tests and `examples/escaping` pass.

## Stop and report if
the keep change makes a std or tools signature refuse a caller that the rule cannot tell apart (a real move through a match): stop with the probe.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v18-matched-lent-param.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
