---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# v5: a field fed from a view, for non-text types; a struct holding a view of a local, anchored (#1218)

Score: the language topics (owner, 2026-11-03), #1218, part of #1213 (ADR-views-3, option D). It follows v2 step 2, which stopped on a wrong order of the earlier brief: such a struct is **anchored, not refused** (#1218 *Decision*). Read #1218 whole, `docs/specification/adr/adr-views-3.md` D5, D9 and D10, Part I 6.6, ADR-283 D23, and `docs/briefs/views/v2-inferred-views.md.report.md` first. S=1, C~3-4 M.

**Owns:**
* The emitter parts that write a tethered field (`crates/nikaia-std/src/tools/emit_*.nika` as the change needs, beginning where a text field's tether is emitted), the checker and ledger parts that decide a field holds a view (`tether.nika`, `tiers.nika`, `views.nika`, `ledger*.nika`) and the place that raises NK2105 for a struct literal, their lowered files, and new tests under `tests/compiler/`.

**Not yours:**
* The parser; `keeps.nika`, `lowered/keeps.rs`, `contracts/keeps.rs`, `contracts/sync.rs` (another lane): a change needed there stops the step and is reported under `Needs:`.
* The specification text.

## To build
1. **A non-text field fed from a view is tethered** (#1218 step 1): a field that receives a view (`xs[0]`, `config.part`, a lent parameter) is emitted as tethered, as a text field is; one that receives an owned value (a call result, a literal) owns it; the ledger lists the field under `tethered`. A struct literal lends a parameter: `go`'s NK2105 goes away. Tests: #1218's probes (b) and the views-3 spelling of (a) inside one function (no return), each running with its `.stdout`. Commit `Part of #1213`.
2. **A struct holding a view of a local is returned, anchored** (#1218 step 2): the local's buffer moves into the result's keep (D9, D10, ADR-283 D23); no copy, no error. Tests: #1218's `make` probes (the `ref` one now only after `ref` is refused elsewhere: use the views-3 spelling), plus a large `xs` with no `.clone()` in `--emit` output. Commit `Fixes #1218`.

A form Nikaia, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*. A further defect: no second fix in this brief, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both rules hold with tests (or a blocker is reported with its probe). `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before plus the new tests; the `it` tests filtered on `view`, `tether`, `ledger`; the guard set of `.claude/agents/builder.md`. Clippy clean; the lower-std fixpoint holds; the committed ledgers of `crates/nikaia-std` and the packages unchanged or their change explained in `Body:`. No changelog line.

## Stop and report if
a `.stderr` of an existing test changes in a way the rule does not explain; the work needs `keep` or `from` parsed (that is v3 and v4: stop, report under `Needs:`); the work needs a change in `keeps.nika`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v5-view-fields-anchored.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
