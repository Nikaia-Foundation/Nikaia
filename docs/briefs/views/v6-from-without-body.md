---
agent: builder
model: opus
max_turns: 80
budget_usd: 14
effort: medium
---

# v6: `ref` results in traits are not warned; function types name parameters for `from` (#1224 steps 1-2)

Score: the language topics (owner, 2026-11-03), ADR-views-3 is the record, D8 and D14 above all (D14 was added 2026-11-04, read it whole). Rust is allowed (#1214) where the code is Rust (the parser's grammar block); `.nika` stays Nikaia. Read issue #1224 whole: its steps 1 and 2 are yours, step 3 is not.

Surface today: v4 (`v4-result-from.md` and its report) built `from` on results (NK1335, NK1336, NK2305) in `check_result_from.nika`, the parser and the AST. It left out `from` on function types because their parameters have no names, and it met NK1261 warning a trait's `-> ref String` as redundant.

Facts:
* Until the `ref` migration (#1224 step 3, not yours), a written `ref` result is the only mark that a body-less result is a view; NK1336 keys on it.
* Names in a function type do not change the type: `fn(s: String) -> String` and `fn(String) -> String` are the same type (D14).
* A test file inside the project cannot be run with `nikaia run`. Run it with `nikaia test tests/compiler --filter <name>`.

**Owns:** the parser's function-type rule, the AST of function types, `check_result_from.nika`, the place NK1261 is decided (`check_core_places.nika` or where it lives) and their tests.

**Not yours:** keeps, the `ref` migration, the ledger.

## To build
1. **NK1261**: no redundancy warning for a `ref` result in a body-less signature (trait method, foreign function); a body's `-> ref` is still warned as today. Test `tests/compiler/tests/views_trait_ref_result_*`, shown warning on the binary before your change. Commit `Part of #1224`.
2. **Function types**: a function type may name its parameters (`fn(s: String, n: i64) -> String from s`); the names do not change the type (a value of `fn(String, i64) -> String` is accepted where the named type is asked, and the other way); a `from` on a function type must name its parameters, and naming one that is not there is refused with NK1335's wording for an unknown source. Tests `tests/compiler/tests/views_fn_type_from_*` (accepted, the same-type case, the refusal). Commit `Part of #1224`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: no third fix, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both steps' tests pass; all `views_*` tests and v4's tests in `tests/compiler` pass.

## Stop and report if
named parameters in function types collide with an existing syntax (a function type that is parsed otherwise today); step 2 needs a new NK code.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v6-from-without-body.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
