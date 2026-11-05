---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# d1: the guide's ownership chapter and the new syntax (owner, 2026-11-03)

Score: the owner wants the guide (`guide/`, beside `syntax.md`) to explain the new syntax and the ownership rules. `guide/syntax.md` has a short section *Memory: lending and keeping*; there is no chapter that teaches the rules whole. Docs only; no compiler change. S=1, C~3 M.

**Owns:**
* `guide/ownership.md` (new), `guide/syntax.md` (its memory section, links, and sections for syntax it lacks), the *Where next* lists of the other `guide/*.md` pages.

**Not yours:**
* `docs/specification/` and its ADRs (the views lane writes ADR-views-3 into the specification now: `docs/briefs/views/`); the compiler, `std`, examples. A contradiction between guide and specification goes under `Found:`.

## To build
1. `guide/ownership.md`: the ownership rules as the compiler checks them today, taught from small programs up: one owner and cleanup at the end of the block; lending (read) and handing over; `mut` and a changed value; keeping (a value that outlives the call), views into a value and why a view cannot outlive its source; the contract ledger (`nikaia.contracts`) as what the reader sees; the common errors with the message the compiler gives and how to fix each; where the model ends (shared data across tasks, `extern "C"`). Every block marked as runnable compiles and runs with `target/fast/nikaia` as shown (check each one; paste the real output). Each section ends with a **Specification:** link into Part I ch. 6 like `syntax.md` does. `syntax.md`'s memory section keeps a short summary and links the chapter. Commit `Part of #1213`.
2. The new syntax: (a) a section of `ownership.md`, *What changes next*, explaining ADR-views-3 (`docs/specification/adr/adr-views-3.md`): `keep` on parameters, `self` / `mut self` / `keep self`, `ref` leaving the language outside `extern "C"`, inferred views, a result's source `-> T from a | b`. It is not built yet: say so plainly at the top of the section, mark every block `does not compile yet`, and show the old and the new spelling side by side. (b) Compare `syntax.md` with the language as built (CHANGELOG entries since `syntax.md` was last changed, `git log -1 guide/syntax.md`): a built form the tour does not show gets a short section or a line in the matching one, checked with the compiler. Commit `Part of #1213`.

Write by `docs/writing-guide.md`: lead with safety, no absolute claims, natural prose without bold lead-ins, nothing shown as runnable before it is built.

## Done when
Both steps committed; every runnable block checked with the compiler; links resolve; `nikaia run scripts/pre-push.nika` clean. No changelog line (docs only).

## Stop and report if
the compiler's behaviour contradicts the specification on a rule the chapter must teach (report under `Found:` with a probe; teach neither until decided).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/docs/d1-ownership-guide.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
