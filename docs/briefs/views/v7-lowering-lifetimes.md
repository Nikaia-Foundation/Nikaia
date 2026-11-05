---
agent: builder
model: opus
max_turns: 80
budget_usd: 14
effort: medium
---

# v7: a trait's `from` result gets its lifetime; a function with a `String` parameter fits a function type (#1230, #1231)

Score: the language topics (owner, 2026-11-03), ADR-views-3 is the record, D2 and D14 above all. Rust is allowed (#1214) where the code is Rust (the lowering's Rust parts); `.nika` stays Nikaia. Read issues #1230 and #1231 whole: their probes and *To build* are yours.

Surface today: v4 and v6 (`v4-*.md`, `v6-*.md` and their reports) built `from` on results and on function types in the checker. The lowering was left as it was: a trait method `fn pick(a: String) -> ref String from a` lowers without a lifetime, and `let g: fn(String, i64) -> String = tag` fails rustc with "`String: Of` is not satisfied" (the latter also on the binary before v6).

Facts:
* A plain parameter is lent (D2), so `a: String` with `from a` must lower to one named lifetime shared by `a` and the result; `a: ref String` builds today.
* Names in a function type do not change the type (D14).
* A test file inside the project cannot be run with `nikaia run`. Run it with `nikaia test tests/compiler --filter <name>`.

**Owns:** the lowering of trait method signatures and of function types, and their tests.

**Not yours:** the checker, keeps, the `ref` migration, the ledger.

## To build
1. **#1230**: a trait method whose result says `from a` (or `from a | b`) lowers with one named lifetime shared by those parameters and the result, `ref` written or not. Test `tests/compiler/tests/views_trait_from_lifetime_*` (trait, implementation, call; runs), shown failing on the binary before your change. Commit `Fixes #1230`.
2. **#1231**: a named function whose parameters are a `String` and a number is accepted for a function type of that signature; test `tests/compiler/tests/fn_type_string_param_*` printing `abab`, shown failing before. Commit `Fixes #1231`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: no third fix, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both probes run; all `views_*`, `fn_type_*`, `trait_*` and `lambda_*` tests in `tests/compiler` pass.

## Stop and report if
the fix for #1231 needs a change in what the checker accepts as a function type (that is a language question, not a lowering defect).
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v7-lowering-lifetimes.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
