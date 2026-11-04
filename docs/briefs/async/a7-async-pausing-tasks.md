---
agent: builder
model: opus
max_turns: 90
budget_usd: 16
effort: medium
---

# a7: a task whose call pauses builds and runs (#1226)

Score: the language topics (owner, 2026-11-03), ADR-async-5 is the record (D20: a pausing task is polled by its group). Rust is allowed (#1214) where the code is Rust; `.nika` stays Nikaia. Read issue #1226 whole: its probe and *To build* are yours.

Surface today: a5b (`a5b-async-fixes-and-lowering.md` and its report) lowered `async e`, its binding and `async { }` onto the group runtime (`crates/nikaia-std/src/task.rs`, `task/group.rs`, `emit_core_args`, `emit_expr`, `emit_stmt`). A task whose call can pause (`fs::read_to_string`) is not covered: the binding form panics in `render_refusal` (a refusal at position -1, #1222), the argument form is refused as a lambda passed to an unknown callee.

Facts:
* #1222 (a refusal at -1 panics in Span::new) is a separate issue; you only give the refusal you meet a real span, you do not fix the renderer.
* The pool at `user_parallelism: yes` is not built; both settings run in sequence.
* A test file inside the project cannot be run with `nikaia run`. Run it with `nikaia test tests/compiler --filter <name>`.

**Owns:** the lowering of `async` (the emit files a5b changed), the group runtime in `task.rs` / `task/group.rs`, and the checker's pause analysis where it judges an `async` task.

**Not yours:** the parser, keeps, views, `render_refusal`.

## To build
1. **Pausing tasks**: `let a = async size_of(p)` and `add(async size_of(a), async size_of(b))` where `size_of` pauses build and print the right sum; tests `tests/compiler/tests/async_pausing_*` (binding, argument, a task block that pauses), each shown failing on the binary before your change. Commit `Part of #1226`.
2. **The span**: the refusal that today lands at -1 for an `async` expression carries that expression's span; a test whose `.stderr` shows it (any refusal that still applies after step 1, or name in `Body:` that none is left). Commit `Fixes #1226`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: no third fix, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both probes of #1226 build and print the expected values; all `async_*` and `task_*` tests in `tests/compiler` pass.

## Stop and report if
an existing `task::scope`, `spawn` or `overlap` program changes output; the build needs the pool at `user_parallelism: yes`.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a7-async-pausing-tasks.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
