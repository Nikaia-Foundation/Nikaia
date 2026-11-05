---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# v3: `keep` on parameters and `self`, parsed and checked (#1213 step 2)

Score: the language topics (owner, 2026-11-03), #1213 (ADR-views-3, option D). Rust is allowed where the code is Rust (#1214, CLAUDE.md *Exception: the specification first*): the parser's grammar block is Rust; the check is new Nikaia. Read `docs/specification/adr/adr-views-3.md` D1-D4, D6 and D10, and #1213's *To build* first. S=1, C~3-4 M.

**Codes reserved for this brief (supervisor):** `NK2302` a body keeps a parameter not marked `keep` (error; help adds `keep`; Part III already lists it); `NK1334` (warning) a `keep` on a parameter the body never keeps. Add their rows to the `NK1xxx` row of Part III (`docs/specification/30-nikaia-tooling.md`) and to the compiler's codes table; no other new code.

**Owns:**
* `crates/nikaia/src/parser/mod.rs` (parameters and receivers), `crates/nikaia/src/ast/mod.rs`, a new checker file `crates/nikaia-std/src/tools/check_keep_marks.nika` and its lowered file, the place that calls it, the two code rows, new tests under `tests/compiler/`.

**Not yours:**
* `keeps.nika`, `lowered/keeps.rs`, `contracts/keeps.rs`, `contracts/sync.rs` (another lane): read their result (the ledger's inferred keeps), never edit them; a change needed there stops the step and goes under `Needs:`.
* `from` on results (v4), the migration tool (later brief), the specification beyond the two rows (v1).

## To build
1. **Parse `keep`**: `keep` before a parameter, on functions, methods and trait methods; receivers `self`, `mut self`, `keep self`. `ref self` / `ref mut self` / `ref T` keep parsing until the migration tool removes them (#1213 step 6). The mark is carried on the AST parameter. Tests: each form parses and runs as today. Commit `Part of #1213`.
2. **Check the mark against the inferred keeps** (D2): compare the written `keep` with what the keep inference already decides for each parameter. An unmarked parameter that is kept is `NK2302` at the keeping use, with a help that writes `keep` in the signature; a `keep` never kept is `NK1334`. A trait's signature fixes the mark for every implementation. Until the migration has written `keep` everywhere, the refusal would break every existing program, so `NK2302` is raised only in a file that uses `keep` at least once (the file has opted into views-3); `NK1334` always. Say this in the code row as a transition rule that the migration brief removes. Tests: a kept unmarked parameter in a file that uses `keep` (refused), the same marked (accepted), an unkept `keep` (warning), `keep self` that consumes (accepted), a trait method whose implementation differs from the trait's mark (refused). Commit `Part of #1213`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A form Nikaia, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*. A further defect: no second fix in this brief, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both steps hold with tests. `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before plus the new tests; the `it` tests filtered on `keep`, `ref`, `self`, `ledger`; the guard set of `.claude/agents/builder.md`. Clippy clean; the lower-std fixpoint holds; the committed ledgers unchanged. No changelog line.

## Stop and report if
the inferred keeps are not reachable from a new checker file without editing `keeps.nika`; an existing program changes output.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/views/v3-keep-marks.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
