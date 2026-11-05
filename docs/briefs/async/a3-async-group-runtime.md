---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# a3: async-5 group runtime and `task::ChildHandle` in `std` (#987 steps 1, 2)

Score: the language topics (owner, 2026-11-03), #987 against ADR-async-5 (the issue text still cites async-4/async-2; async-5 is the record). Rust is allowed here (#1214): the std runtime `crates/nikaia-std/src/task.rs` is Rust. Read `docs/specification/adr/adr-async-5.md` D1-D4, D9, D11-D14, D20, D21 and D34, Part II 12.7 and ADR-292 D6 first. S=1, C~3 M.

**Owns:**
* `crates/nikaia-std/src/task.rs` (and a new std module beside it if the group grows past one file), the std declarations of `task` that the checker reads (the `.nika` or registry entry that names `task::ChildHandle`, `start`, `cancel`, `join`), and new tests under `tests/compiler/tests/` that call the runtime through the existing `task::scope` / `async(detached)` surface where it reaches it.

**Not yours:**
* The parser (a1, a2), the checker (#986), lowering of the new forms onto this runtime (#988: it calls what you build; you make the entry points it will call, documented with doc comments).
* `contracts/sync.rs`, `keeps.rs` (another lane).

## To build
1. **The group runtime** (#987 step 1, D1, D3, D9, D20, D21): a group that starts tasks, lives until every task in it has ended, gives each task's end (value, failure, cancelled, `task::OverBudget` reserved for later) to its receiver, and keeps every failure nobody read until the group's end, then raises them in the order D21/D22 say. The existing `task::scope` runs on it, unchanged in behaviour. Tests: a scope of three tasks whose results are read in order; a task failure nobody reads raised at the group's end; two failures both kept. Commit `Part of #987`.
2. **Handles** (#987 step 2, D11-D14): `task::ChildHandle[T]` bound to its group (`join`, `cancel`; the group waits for it if it is never joined), and `TaskHandle.start()` for a deferred handle (runs only from `start()`; a second `start()` panics at run time, the compile-time check is NK2121 in #986). `cancel` is the only way to cancel (D14). Tests through `std` calls where the surface reaches them; where only the parser's new forms reach a piece, its doc comment names the #988 entry point and the test waits for #988. Commit `Part of #987`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A form Nikaia, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*. A further defect: no second fix in this brief, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both steps hold with tests. `nikaia test tests/compiler --both-settings` gives every count as before plus the new tests; the `it` tests filtered on `task`, `overlap`, `spawn`, `scope`; `cargo test -p nikaia-std` for `task`; the guard set of `.claude/agents/builder.md`. Clippy clean. No changelog line.

## Stop and report if
an existing `task::scope`, `spawn` or `overlap` program changes output; a piece needs the lowering of #988 to be testable at all (build the runtime, say which test waits, `Result: partial`).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a3-async-group-runtime.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
