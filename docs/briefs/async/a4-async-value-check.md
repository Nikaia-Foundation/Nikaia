---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# a4: `async e`, `async e` as a statement and `async { }` reach the checker; wait points, groups, NK2110, NK2111 (#984 rest, #986 steps 1, 2)

Score: the language topics (owner, 2026-11-03), ADR-async-5 is the record (#986's text still cites async-4/async-2; read it through async-5). Rust is allowed here (#1214): the parser's grammar block and the AST are Rust; the checker's walks are `.nika` (`crates/nikaia-std/src/tools/check_*.nika`, `touch.nika`) and stay Nikaia. Read `docs/specification/adr/adr-async-5.md` D5-D10, D30, D33 and Part I 8.1.2 first. S=1, C~3 M.

Surface today: `async(detached) fn { … }` parses into `Expr::Spawn` (33d97ed5); every other form is refused in `crates/nikaia/src/parser/mod.rs` by a `fail` naming it (`async_expr`, `async_stmt`, the `*_is_not_built_yet` tests); NK2115 and NK2120 live in `check_places.nika` (836268c1). The runtime the lowering will call is `crates/nikaia-std/src/task/group.rs` (6331c7ab). This brief stops at the checker: a program using the new forms gets past `nikaia check` but `nikaia run` refuses it at lowering with a message naming the form as not built yet (a5 builds that).

**Owns:**
* `crates/nikaia/src/parser/mod.rs` (the `async_*` rules), `crates/nikaia/src/ast/mod.rs` and the `.nika` mirror of the AST if the checker reads one, the checker files the steps need, the lowering's one refusal for the new node, and new tests under `tests/compiler/tests/async_*`.

**Not yours:**
* The runtime (`task.rs`, `task/group.rs`), handles and options (`handle`, `deferred`, `limit:`, `within:`, `admission:` stay refused in the parser), touch sets / NK2104 between tasks (a later brief), the lowering onto the runtime (a5).
* `contracts/sync.rs`, `keeps.rs` (another lane).

## To build
1. **The node and the wait points** (#986 step 1, D6, D8 first bullet only, D9, D10): one AST node for `async e` (value, in a binding, an argument, an operand, a tuple part), `async e` as a statement and `async fn { … }` (the lambda is run, D10), and one for `async { … }`; the parser stops refusing those forms (their `*_is_not_built_yet` tests become checker or run tests, or are removed where the form is now accepted). The checker computes each task's wait point as D6 says (a binding: before the first statement of its block that names any of its names anywhere nested; an argument: before its call; an operand: before its operation; a statement or a binding nothing names: its group's end) and records it on the node for a5. Types (#986 step 9): `async e` has `e`'s type, `async fn { … }` its lambda's result type. Jumps (#986 step 10): a `break`/`continue`/`return` leaving the form is `NK1132`. Tests: a binding waited before the first statement naming it inside a nested `if`; `f(async g(), h())` type-checks with `f` taking a plain `T`; `return` out of `async { … }`'s task refused. Commit `Part of #986`.
2. **Groups and the empty window** (#986 step 2, D7 first paragraph, D9): a task's group is the innermost `async { }` in the same function, else the innermost block holding the form. `NK2110` for a task whose window is empty (`f(async g())` with nothing computed beside it, `let x = async g()` named in the next statement, `async g()` as the last statement of its group), help removes the word; `NK2111` for an `async { }` that starts no task. Both codes are in Part III already (check; if a code lacks its Part III entry, stop and report). Tests: each refusal with its `.stderr`, and an accepted program for each form. Commit `Part of #986`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A form Nikaia, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*. A further defect: no second fix in this brief, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both steps hold with tests. `nikaia test tests/compiler --both-settings` gives every count as before plus the new tests (the changed `*_is_not_built_yet` ones named in the report); the `it` tests filtered on `parser`, `async`, `spawn`; the guard set of `.claude/agents/builder.md`; `nikaia fmt` still formats every form (the a2 test passes). Clippy clean. No changelog line.

## Stop and report if
an existing `spawn`, `task::scope` or `overlap` program changes output or diagnostics; the wait point needs a fact the checker does not have and getting it would mean rewriting a walk (say which, `Result: partial`).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a4-async-value-check.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
