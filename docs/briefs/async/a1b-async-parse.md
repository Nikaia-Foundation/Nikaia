---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# a1b: async-5 prefix forms in the parser; unbuilt forms refused at parse (#984 steps 2, 4)

Score: the language topics (owner, 2026-11-03), #983 / #984 against ADR-async-5 (the issue text still cites async-4; async-5 is the record). Rust is allowed here (#1214, CLAUDE.md *Exception: the specification first*): the grammar block of `crates/nikaia/src/parser/mod.rs` and `crates/nikaia/src/ast/mod.rs` are Rust. Read `docs/specification/adr/adr-async-5.md` D5, D6, D9-D13, D15, D22 and D34, Part I 8.1.2, and the `NK21xx` row of Part III first. S=1, C~3 M.

**Owns:**
* `crates/nikaia/src/parser/mod.rs` (the reserved words near `KW_ASYNC`, `KW_OVERLAP`, `KW_SPAWN`; `spawn_expr`, its use and `overlap_expr`), `crates/nikaia/src/ast/mod.rs`, the places that must match on the new AST nodes so the build stays whole (a node the checker or lowering does not handle yet is refused there with the existing "not built yet" path, not silently dropped), and new tests `tests/compiler/tests/async_parse_*.nika`.

**Not yours:**
* The checker rules (#986: NK2110-NK2122 except what step 3 of #984 says, which is a2), lowering (#988), the runtime in `crates/nikaia-std/src/task.rs` (a3).
* `tools/fmt.nika` (a2), the specification text.

## To build
1. Step 1 of a1 (reserved words) is on develop (656a44f6); do not redo it.
2. **The prefix forms** (#984 step 2 and 4): parse `async e` as a value (D6), `async e` as a statement (D9), `async { … }` as the group (D9), `async fn { … }` run as the task (D10; a lambda after `async` that names an argument is `NK2103`, which exists), `async(handle|deferred|detached, limit: e, within: e, admission: e) fn { … }` (D11-D13, D15), and a `catch { … }` after any form (D22). Supervisor decision (2026-11-03, after a1 found no "not built yet" path in the checker): a form the existing AST nodes and lowering already express parses into them; every other form is refused **in the parser** with a `fail`, as `immutable_item` is (ADR-389 D4): the message names the form and says it is not built yet (#986). No new `Expr` variant and no new NK code in this brief; the nodes come with #986, form by form, where the checker handles them. `spawn fn { … }` parses as `async(detached) fn { … }`. Tests: one program per form; a built form parses and runs, an unbuilt form gives its parse refusal in `.stderr`. Commit `Part of #984`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A form Nikaia, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*. A further defect: no second fix in this brief, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Step 2 holds with tests. `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before plus the new tests; the `it` tests filtered on `overlap`, `spawn`, `task`, `async`; the guard set of `.claude/agents/builder.md`. Clippy clean; the lower-std fixpoint holds. No changelog line.

## Stop and report if
an existing program using `spawn`, `overlap` or `task::scope` stops parsing or changes output; the grammar of a form is ambiguous against an existing expression (report the two readings under `Needs:`).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a1b-async-parse.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
