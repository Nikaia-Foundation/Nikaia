---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---

# v23: a field of `mut self` and an element of a `mut` list handed to a keeper are taken (#1208)

Score: Part I 6.5 and ADR-views-1 D1-D3 (#949): a part of a `mut` parameter handed to something that keeps it is taken, and assigning the part gives it back; `mut self` is such a parameter too. #949 built it for a `let mut` view. Read #1208 for both probes (re-probed November 5, 2026). Rust is allowed in the checker and the emitter for the views-3 lane (owner 2026-11-03, #1214; list it in `Body:`).

Facts: Checker and views, Lowering and lower-std, Tests and the gate

Probe 1 (`hold(keep ws: Vec[u32])`, `self.trail` in a `mut self` method) gives `NK1131` and `NK2106`, both saying `self` is only borrowed. Probe 2 (`hold(w[1])` with `mut w: Vec[Vec[u32]]`) passes the checker and fails in rustc: *Cannot move out of a shared reference.* Both should print `0 1`.

**Owns:** the take/give-back rule in the checker for a field of `mut self` and an indexed element of a `mut` place, its lowering (`std::mem::take(&mut place)`, or `mem::replace` with D3's stand-in), the messages of NK1131/NK2106 for `mut self`, the new tests.

**Not yours:** #979 (a `let` that moves a field out and gives it back: not decided), v21, v22.

## To build
1. Probe 1: `tests/compiler/tests/a_field_of_mut_self_handed_to_a_keeper_is_taken.nika` (`.stdout` `0 1`), plus a `.stderr` case where the field is not given back before the way out (`NK2108`); NK1131/NK2106 no longer say `self` is only borrowed for a `mut self`. Commit `Part of #1208`.
2. Probe 2: `tests/compiler/tests/an_element_of_a_mut_list_handed_to_a_keeper_is_taken.nika` (`.stdout` `0 1`), the same on `self.w[i]` in a `mut self` method, plus the `NK2108` `.stderr` case. Commit `Fixes #1208`.

`Rs2nika: none`.

## Done when
The four tests pass at both settings; the #949 tests pass.

## Stop and report if
taking an indexed element needs a stand-in for a type without a default that D3 does not cover.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v23-take-from-mut-self.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
