---
agent: builder
model: opus
max_turns: 70
budget_usd: 12
effort: medium
---

# a9: an `async` task that changes an outer name is NK2107 (#1229)

Score: the language topics (owner, 2026-11-03), ADR-async-5 is the record. Rust is allowed (#1214) where the code is Rust; `.nika` stays Nikaia. Read issue #1229 whole: its probe and *To build* are yours.

Surface today: a8 (`a8-*.md`, its report, commit ceabe4ac) refuses NK2107 only when two tasks of one group touch the same name. ADR-async-5 D7 (`docs/specification/adr/adr-async-5.md`, line "A task that changes a name bound outside it is `NK2107`") is stricter: any change of an outer name inside a task is refused, whatever runs beside it. The ADR wins.

Facts:
* A change is an assignment or a method that changes what it is called on (as for a `par_iter()` lambda, Part III's NK2107 row).
* Reading an outer name in a task stays accepted; changing a `SharedMut` through `update` stays accepted.
* A test file inside the project cannot be run with `nikaia run`. Run it with `nikaia test tests/compiler --filter <name>`.

**Owns:** the checker's NK2107 for `async` tasks, its tests, Part III's NK2107 row.

**Not yours:** the lowering, the parser, keeps, views.

## To build
1. **The rule**: NK2107 for every `async` task that changes a name bound outside it; tests `tests/compiler/tests/async_outer_write_*` for #1229's probe (refused, shown accepted on the binary before your change) and for a task that only reads (accepted). a8's `async_conflict_*` tests that change an outer name keep passing with their messages updated if they change. Commit `Part of #1229`.
2. **Part III**: the NK2107 row of `docs/specification/30-nikaia-tooling.md` names the `async` task beside the `par_iter()` lambda, in one sentence, with the same ways out. Commit `Fixes #1229`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: no third fix, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
#1229's probe is refused; the read-only case runs; all `async_*`, `task_*`, `spawn_*` and `overlap_*` tests in `tests/compiler` pass.

## Stop and report if
a program in `crates/nikaia-std`, `packages/` or `examples/` that compiles today is refused by the new rule (list them under `Found:`, do not change them).
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a9-async-outer-write.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
