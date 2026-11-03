---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---

# a2: async-5 parse refusals, `fmt` for every form, parse tests (#984 steps 3, 5, 6)

Score: the language topics (owner, 2026-11-03), #984 against ADR-async-5. Needs a1b on develop. Forms a1b refuses at parse (not built yet, until #986) are not formatted here. Rust is allowed where the code is Rust (#1214); `fmt` is `tools/fmt.nika`, written in Nikaia. Read `docs/specification/adr/adr-async-5.md` D5, D30, D33, and the `NK21xx` row of Part III first. S=1, C~2-3 M.

**Owns:**
* `crates/nikaia/src/parser/mod.rs` and the check pass that raises NK2115 / NK2120 if it is not the parser, `crates/nikaia-std/src/tools/fmt.nika` and its lowered file, new tests `tests/compiler/tests/async_parse_*.nika` and `tests/errors` entries for the two codes.

**Not yours:**
* The other async codes (#986), lowering (#988), the runtime (a3), the specification text (the two codes are already specified in Part III).

## To build
1. **NK2115 and NK2120** (#984 step 3): `async` before a function's declaration, a `let` or a loop is NK2115 (D5); an `async` form inside a loop's body is NK2120 (D30), with the help Part III names (`xs.par_iter().for_each fn(x) { … }`). Both codes are specified; register them where the codes table of the compiler lives if they are not there. Tests: a refusal for each, with its `.stderr`. Commit `Part of #984`.
2. **`fmt` and the parse tests** (#984 steps 5, 6): `nikaia fmt` formats every form a1b parses, and formatting twice changes nothing. Tests: one formatter test over all forms (input, expected output, idempotent), plus parse tests for any form a1 left without one. Commit `Part of #984`; if #984 is then fully built (steps 1-6), the last commit says `Fixes #984`.

List under `Body:` every Rust function you added or changed, for #557 / #125.

A form Nikaia, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*. A further defect: no second fix in this brief, file nothing, report it under `Found:` with its probe.

`Rs2nika: none`.

## Done when
Both steps hold with tests. `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before plus the new tests; the `it` tests filtered on `fmt`, `async`, `spawn`; the guard set of `.claude/agents/builder.md`. Clippy clean; the lower-std fixpoint holds. No changelog line.

## Stop and report if
a1's nodes are missing; an existing program is refused by NK2115/NK2120 (name it under `Found:`).

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/async/a2-async-parse-refusals.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
