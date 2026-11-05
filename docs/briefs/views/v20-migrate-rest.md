---
agent: builder
model: opus
max_turns: 80
budget_usd: 12
effort: medium
---

# v20: finish the `ref` migration of the examples and packages, then the small test folders (#1213 step 6, part 3)

Score: the language topics (owner, 2026-11-03), ADR-views-3 is the record, D4 above all (`ref` leaves the language outside `extern`). Read issue #1213 (step 6), and the report `v17-migrate-docs-packages.md.report.md` beside this brief whole: v17 migrated what came out right and listed what it left and why. v18 (a `match` on a lent parameter is no keep; a lent `String` passed on to a lent `str`) and v19 (the tool reads a package's tests with the package's ledger; `keep self` where a by-value `self` is consumed) fixed its four findings. The docs come first (owner, 2026-11-05: the examples and documentation on develop as soon as possible).

Facts: Where things live, Tests and the gate, Packages.
* The tool: `nikaia migrate views-3 <path>...` (`crates/nikaia-std/src/tools/migrate_views3.nika`, door `crates/nikaia/src/migrate.rs`). It refuses a package with dependencies; run it per package, dependencies first.
* Check every file the tool changed before and after (copy the binary first, as v17 did): same errors and exit codes, and no `keep` the old signature did not mean. A `keep` the tool writes on a parameter the function only reads is a defect of the tool or the checker, not a file to hand-fix.
* A test whose point is `ref` itself (NK1261, a `ref` refused at a call, `extern`) keeps its `ref`; list it under `Body:`.

**Owns:** the rewritten files under `examples/`, `packages/`, `guide/`, `tests/language`, `tests/accepted`, `tests/build-time`, `tests/guide`, `benches/` and their expected outputs and `nikaia.derived` / contracts.

**Not yours:** `crates/nikaia-std` (std is migrated in a later brief, so a call into std that still takes `ref` stays as it is: `examples/inventory` waits for it, say so), `tests/compiler`, the tool, the checker, the ledger (a small defect in them: see below).

## To build
1. **What v17 left in examples and packages**: `examples/json`, `examples/http`, `examples/escaping` (if v18 has not already dropped its `ref`), `examples/fortunes`, `packages/json`, `packages/yaml/src` and the package tests; and the "today" cart of guide/ownership.md *What changes next* only if Part I now calls the old spelling an error (then the section is rewritten to the views-3 wording, `docs/writing-guide.md`). Every example and package test that passed before passes. Commit `Part of #1213`.
2. **The small test folders and benches** (`tests/language`, `tests/accepted`, `tests/build-time`, `tests/guide`, `benches/`): run the tool, re-bless only outputs that changed because a `ref` is gone, and the tests of these folders pass. Commit `Part of #1213`.

A defect of the tool or the checker that a rewritten program shows (it checked with `ref` and does not without): fix it in the tool or the checker if it is small, with a test, as its own commit; otherwise keep that file unmigrated and report it under `Found:` with its probe. Step 1 is committed before step 2 starts, whatever step 2 meets.

`Rs2nika: none`.

## Done when
`grep -rnw ref examples packages guide tests/language tests/accepted tests/build-time tests/guide benches` finds only `extern` code, prose about it, the tests about `ref` itself and what waits for std, each listed under `Body:`; the tests of these folders pass.

## Stop and report if
the tool writes a `keep` the checker then refuses, or a rewritten program no longer checks and the fix is not small.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v20-migrate-rest.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
