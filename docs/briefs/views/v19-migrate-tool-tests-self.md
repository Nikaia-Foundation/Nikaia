---
agent: builder
model: opus
max_turns: 80
budget_usd: 12
effort: medium
---

# v19: `nikaia migrate views-3` reads package tests with their package, and writes `keep self` (#1213, found by v17)

Score: ADR-views-3 (D1, D4). Read `v17-migrate-docs-packages.md.report.md` beside this brief, `Found:` (2) and (3), and `v9b-commit-migration-tool.md.report.md`. The tool: `crates/nikaia-std/src/tools/migrate_views3.nika`, door `crates/nikaia/src/migrate.rs` (Rust is allowed in the door for the views-3 lane, owner 2026-11-03, #1214; list it in `Body:`).

Facts: Checker and views, Packages, Tests and the gate

**(2)**: the tool reads a package's `tests/*.nika` alone (`Package::None`), so `json::parse(text)` has no ledger and every caller's parameter gets a fail-closed `keep` (`line(keep text)`). A test file of a package is read with that package's ledger, as `nikaia test` reads it.

**(3)**: an old by-value `self` that the body consumes gets no `keep self`:
```nika
struct Cart {
    items: Vec[String],
}

impl Cart {
    fn into_items(self) -> Vec[String] {
        return self.items
    }
}
```
After the tool, `into_items` is refused (NK1131). The tool writes `keep self` where the old `self` was by value and the body moves out of it, as it writes `keep` for a moved plain parameter.

**Owns:** `migrate_views3.nika`, `migrate.rs`, their lowered files, the tool's tests.

**Not yours:** the checker's keep rule (v18), the files the tool rewrites (a later brief reruns v17).

## To build
1. **(2)**: package tests read with the package's ledger; a test: the tool run over `packages/json` writes no `keep` into `packages/json/tests/`. Commit `Part of #1213`.
2. **(3)**: `keep self` for a consumed by-value `self`; the probe above as the tool's test (before/after). Commit `Part of #1213`.

`Rs2nika: none`.

## Done when
Both cases come out right in the tool's tests; the tool's existing tests pass.

## Stop and report if
reading a package test with its ledger needs a change in how `nikaia test` builds packages.
## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v19-migrate-tool-tests-self.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
