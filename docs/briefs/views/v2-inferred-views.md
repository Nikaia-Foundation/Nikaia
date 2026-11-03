---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# v2: inferred views in fields, elements and struct literals (#1213 step 3)

Score: the language topics (owner, 2026-11-03), #1213 (ADR-views-3, option D). This step needs no new syntax: it is the checker half of views-3 D5. Read `docs/specification/adr/adr-views-3.md` D5 and D9, Part I 6.6, and #1152 (its probe) first. S=1, C~3-4 M.

**Owns:**
* The checker and ledger parts in `crates/nikaia-std/src/tools/` that decide whether a field or element holds a view (`tether.nika`, `tiers.nika`, `text_tiers.nika`, `views.nika`, `view_changes.nika`, `ledger*.nika` as the change needs), their lowered files, and new tests under `tests/compiler/`.

**Not yours:**
* The parser (`crates/nikaia/src/parser/mod.rs`): `keep` and `from` wait on the decision #1214; nothing here parses a new word.
* `keeps.nika`, `lowered/keeps.rs`, `contracts/keeps.rs`, `contracts/sync.rs` (another lane): a change needed there stops the step and is reported under `Needs:`.
* The specification text (v1).
* `tools/emit_*.nika`, `describe*.nika`.

## To build
1. **A changed field or element holds an own value** (D5): where a field or an element of a collection is changed in place, or handed to a parameter that keeps it, and a view (a piece of a buffer) would flow into it, the program is refused at the place the view flows in, with a help naming where to write `.clone()`. A field that only holds views stays a tethered view and is listed in the ledger's `tethered`. Use the existing error code that refuses a view flowing where an own value is needed if one fits; if none fits, stop this piece and report it under `Needs:` (a new code is the supervisor's). Tests: a struct whose field is built from a piece of a buffer and then changed (refused, help names `.clone()`); the same with `.clone()` (accepted); a map whose keys are pieces of a buffer and never changed (accepted, ledger lists the view). Commit `Part of #1213`.
2. **A struct literal lends a local or a parameter as a call argument does** (D5 last paragraph, #1152): building a struct, variant, tuple or list from a local or a parameter is no keep while the value stays in the function; the struct may not outlive what it lends. Read `docs/briefs/bugs/b37-1152-ref-field-from-local.md.report.md` first: it built part of this and left a question, which views-3 D5 answers: a struct holding a view of a local, handed back past that local, is refused (its probe under `Found: (1)` reaches rustc today; it must stop in the checker with the existing escape error). Tests: #1152's probe (in #1213's history, folded there) accepted; b37's probe refused. Commit `Part of #1213` (#1152 is closed into #1213).

A form Nikaia, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*. A further defect: no second fix in this brief, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both rules hold with tests (or a blocker is reported with its probe). `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before plus the new tests; the `it` tests filtered on `view`, `tether`, `ledger`; the guard set of `.claude/agents/builder.md`. Clippy clean; the lower-std fixpoint holds; the committed ledgers of `crates/nikaia-std` and the packages unchanged or their change explained in `Body:`. No changelog line.

## Stop and report if
a `.stderr` of an existing test changes in a way the rule does not explain; the work needs `keep` or `from` parsed; the work needs a change in `keeps.nika`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v2-inferred-views.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
