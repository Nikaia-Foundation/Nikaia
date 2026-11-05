---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# v4: a result's source, `from s` / `from a | b`, parsed and checked (#1213 step 4)

Score: the language topics (owner, 2026-11-03), #1213 (ADR-views-3, option D). Needs v3 on develop (the parameter marks): if `keep` does not parse, stop with `blocked`. Rust is allowed where the code is Rust (#1214); the check is new Nikaia. Read `docs/specification/adr/adr-views-3.md` D7-D10 and Part I 6.6 first. S=1, C~3-4 M.

**Codes reserved for this brief (supervisor):** `NK1335` a way out hands back a view of a parameter the written `from` does not name; `NK1336` a result that is a view with no `from` where there is no body to infer it from (a trait's method, a function type, a foreign or Rust function). Add their rows to the `NK1xxx` row of Part III and to the compiler's codes table. The caller rule (D9) uses the existing `NK2305`.

**Owns:**
* `crates/nikaia/src/parser/mod.rs` (result types), `crates/nikaia/src/ast/mod.rs`, a new checker file `crates/nikaia-std/src/tools/check_result_from.nika` and its lowered file, the ledger writer's `views` entry for a written `from` (D10: recorded as written), the two code rows, new tests under `tests/compiler/`.

**Not yours:**
* `keeps.nika`, `lowered/keeps.rs`, `contracts/keeps.rs`, `contracts/sync.rs`: read, never edit; a needed change goes under `Needs:`.
* The migration tool, the specification beyond the two rows.

## To build
1. **Parse and check `from`** (D8): `-> T from s` and `-> T from a | b` after a result type, on functions, methods, trait methods and function types. Where written, every way out is checked against it (`NK1335`); where it must be written and is missing, `NK1336`. The ledger records a written `from` as written. Tests: `first_word` with `from s` (accepted), `longest` with `from a | b` (accepted), a body that returns a view of a parameter not named (refused), a trait method with a view result and no `from` (refused). Commit `Part of #1213`.
2. **The caller rule** (D9): a caller that changes a source in place, or hands it to a `keep` parameter, while a view the result holds is still used, is refused with `NK2305` naming the view, the call and the parameter, for a written and an inferred source alike. Check first whether `NK2305` already covers the inferred case; build only what is missing. Tests: change of the source after the last use (accepted), before it (refused), a reassignment of the source while the view lives (accepted: D9 gives the buffer to the view). Commit `Part of #1213`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A form Nikaia, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*. A further defect: no second fix in this brief, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both steps hold with tests. `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before plus the new tests; the `it` tests filtered on `view`, `ledger`, `from`; the guard set of `.claude/agents/builder.md`. Clippy clean; the lower-std fixpoint holds; the committed ledgers unchanged or their change explained in `Body:`. No changelog line.

## Stop and report if
`from` is ambiguous against an existing expression or type form (report both readings under `Needs:`); the work needs `keeps.nika` changed.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v4-result-from.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
