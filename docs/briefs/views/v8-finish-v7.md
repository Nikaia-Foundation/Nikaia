---
agent: builder
model: opus
max_turns: 60
budget_usd: 10
effort: medium
---

# v8: finish v7 from its uncommitted edits (#1230, #1231)

Score: the language topics (owner, 2026-11-03), ADR-views-3 is the record, D2 and D14 above all. Rust is allowed (#1214) where the code is Rust; `.nika` stays Nikaia. Read issues #1230 and #1231, `v7-lowering-lifetimes.md` and its debrief `v7-lowering-lifetimes.debrief.md` first: v7's builder ended on an API error with its step 1 built but not committed.

Surface today, uncommitted in the shared checkout (they are yours now):
* `crates/nikaia-std/src/tools/emit_item.nika`, `emit_function.nika`, `keeps.nika`, `lends.nika` and their files in `lowered/`;
* the tests `tests/compiler/tests/views_trait_from_lifetime_shared_by_its_sources.*` and `fn_type_string_param_takes_a_named_function.*`.

Facts (from v7's debrief):
* #1230 is not only the lowering: a trait method kept every by-value parameter (`keeps.nika`, the seed *by value means kept*, waiting on #1012 step 3) and a method never lent a plain parameter (`lends.nika`, `lends_with`). v7 changed both narrowly: a parameter the result's `from` names is lent (D2, D14). That scope is granted here.
* #1231's probe prints `abab` on the current binary and on the release binary of 2026-11-02: it does not reproduce. Its test is coverage; no fix.
* Trait signatures lower in `emit_item.nika` (`emit_trait_method`), lifetimes in `emit_function.nika` (`several_at`).
* A test file inside the project runs with `nikaia test tests/compiler --filter <name>`, not `nikaia run`.

**Owns:** the edits above, the lowering of trait method signatures, and the `from`-names-are-lent rule in keeps and lends for methods.

**Not yours:** the checker, the `ref` migration, the ledger, `a_match_on_a_field_of_a_mut_parameter_*` (another agent's untracked test).

## To build
1. **#1230**: build the binary from the edits, run the `views_trait_from_lifetime_*` test (shown failing before: cite v7's probe on the old binary, or run it with the release binary of 2026-11-02), then the `views_*`, `trait_*`, `lambda_*` and `keeps_*`/`lends_*` tests. Fix what they show. Commit `Fixes #1230`.
2. **#1231**: commit the `fn_type_string_param_*` test as coverage, with a probe line in the commit message saying it does not reproduce on the current or the 2026-11-02 binary. Commit `Fixes #1231`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A further defect: no third fix, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both tests pass; all `views_*`, `fn_type_*`, `trait_*` and `lambda_*` tests in `tests/compiler` pass.

## Stop and report if
the lent rule for `from` parameters breaks a test outside the lanes above that needs a keeps decision (#1012 step 3) to fix.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v8-finish-v7.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
