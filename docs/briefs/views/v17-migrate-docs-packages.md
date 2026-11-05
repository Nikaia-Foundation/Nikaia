---
agent: builder
model: opus
max_turns: 80
budget_usd: 12
effort: medium
---

# v17: run the `ref` migration over the guide, the examples, the packages and the small test folders (#1213 step 6, part 2)

Score: the language topics (owner, 2026-11-03), ADR-views-3 is the record, D4 above all (`ref` leaves the language outside `extern`). Read issue #1213 whole (step 6 is yours in part), `v9-ref-migration.md` and the report `v9b-commit-migration-tool.md.report.md` beside this brief. The tool exists and is committed: `nikaia migrate views-3 <path>...` (`crates/nikaia-std/src/tools/migrate_views3.nika`, door `crates/nikaia/src/migrate.rs`). This brief only runs it; v10-v16 built what v9 said running it waited for (a bare `self` is lent, `Clone` is a bound).

Surface today (`grep -rcw ref`, `.nika` and `.md`): `guide/` 31 lines, `examples/` 57, `packages/` 58 (yaml 43, json 15), `tests/language` 280, `tests/accepted` 88, `tests/build-time` 42, `tests/guide` 10, `benches/` 267. Some of these are prose about `ref`, `extern` code, or tests that are about `ref` itself.

Facts:
* The tool refuses a package with dependencies (their ledgers would be needed). Run it per package, dependencies first (`json` before `yaml` if yaml uses json), or say under `Found:` which package it refused.
* Markdown blocks are ` ```nika `; prose is not rewritten. Prose in `guide/` that teaches `ref` as a parameter form is rewritten by hand to the views-3 wording of Part I ch. 6 (`docs/writing-guide.md`); prose about `extern` stays.
* A test whose point is `ref` itself (NK1261 "`ref` is redundant", a `ref` refused at a call, `extern`) keeps its `ref`: leave it and list it under `Body:`. NK1261 retires in a later brief, once std is migrated, not here.
* A test file inside the project runs with `nikaia test <dir> --filter <name>`. Look at every file `--bless` writes before committing; an empty `.stdout` for a program that does not check is a defect, not a blessing.

**Owns:** the rewritten files under `guide/`, `examples/`, `README.md`, `packages/`, `tests/language`, `tests/accepted`, `tests/build-time`, `tests/guide`, `benches/` and their expected outputs.

**Not yours:** `crates/nikaia-std` (std and the compiler's `tools/*.nika`, a later brief), `tests/compiler` (the next brief), the tool itself, the checker, the ledger.

## To build
1. **Guide, examples, README and the packages**: run the tool, hand-fix the prose, and every guide, example and package test that passed before passes. Commit `Part of #1213`.
2. **The small test folders and benches** (`tests/language`, `tests/accepted`, `tests/build-time`, `tests/guide`, `benches/`): run the tool, re-bless only outputs that changed because a `ref` is gone (an NK1261 warning that disappears), and the tests of these folders pass (`docs/builder-facts.md` names how each is run). Commit `Part of #1213`.

A defect of the tool or the checker that a rewritten program shows (it checked with `ref` and does not without): fix it in the tool or the checker if it is small, with a test, as its own commit; otherwise stop, keep that file unmigrated, and report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
`grep -rnw ref guide examples README.md packages tests/language tests/accepted tests/build-time tests/guide benches` finds only `extern` code, prose about it, and the tests about `ref` itself listed under `Body:`; the tests of these folders pass.

## Stop and report if
a rewritten program no longer checks because the checker still needs the `ref` and the fix is not small, or the tool writes a `keep` the checker then refuses.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v17-migrate-docs-packages.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
