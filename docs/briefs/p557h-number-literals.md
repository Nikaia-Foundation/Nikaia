---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# p557h: the number literal rules of the parser grammar (#1201, part of #557)

Score: the parser port (owner, 2026-11-02, #1188: port to Nikaia, helpers first; rules side by side, switched in one commit later). p557g built the other literal rules (9d1461d1) and stopped at the number rules: they need a bound `peek`, which winnow-grammar refused. Owner's decision on #1201 (option A): winnow-grammar allows a bound peek; it is on develop since e265d24f (winnow-grammar 7235ce46). Read `docs/briefs/p557g-parser-rules2.md.report.md` and its debrief first. S=1, C~3-4 M.

**Owns:**
* `crates/nikaia-std/src/tools/parser_grammar.nika`, its `tools/lowered/parser_grammar.rs`, new tests under `tests/compiler/`, and the grammar front end, checker and lowering parts a defect needs (its own commit, with a test).

**Not yours:**
* The Rust `grammar!` block in `crates/nikaia/src/parser/mod.rs`: not changed.
* Every lowercase (syntactic) rule other than `number_lit`/`negative_number_lit`/`float_end`; #1198 is not built here.
* `winnow-grammar` and `winnow` (the owner's crates): not patched; a rule they cannot say is a finding.
* The comment list (step 4 of p557g): it waits on an owner question (an entry without trivia skipping); not built here.
* The files of other builders: `tools/emit_*.nika`, `tools/check_*.nika`, `keeps.nika`, `describe*.nika`.

## To build
1. Probe first: `rule NUM -> String = p:peek(text(PRE)) t:text(any{p.len() as i64}) -> { t.clone() }` (the probe of #1201) checks and runs with the new winnow-grammar. If it does not run, that is a defect of the lowering of a bound peek: fix it in the compiler with a test `grammar_bound_peek.nika`, its own commit `Fixes #1201`; otherwise add that test with the first rule commit and write `Fixes #1201` there.
2. `FLOAT`, `float_end`, `number_lit` and `negative_number_lit`, translated by hand from the Rust rules, whose actions call `tools/parse_numbers.nika`. Tests as in p557g step 3 (expected output from what the Rust rule gives for the same inputs), plus the number literals of ADR-285 D16 and a refused one whose message must equal the Rust grammar's. Commit `Part: #557`.

A rule whose body needs a form Nikaia, the grammar language, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*: absent on purpose (name the ADR): build it that way; not decided or not thought of: stop that piece, keep what is committed, report it with a probe of a few lines; a further defect: no second fix in this brief, file nothing, report it under `Found:` with its probe.

`rs2nika` is deleted (docs/builder-facts.md): translate by hand; `Rs2nika: none`.

## Done when
The four rules are Nikaia with tests (or a blocker is reported with its probe); #1201's probe passes as a test. `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before plus the new tests; a test that fails only because the binary vanished mid-run is re-run singly. The `it` tests filtered on `parse`, `grammar` and `span`, plus the guard set of `.claude/agents/builder.md`. Clippy clean; the lower-std fixpoint holds. No changelog line.

## Stop and report if
a `.stderr` of an existing test changes; the work needs a change in `winnow-grammar`; you meet a missing form of kind 2 (above).

Read `docs/builder-facts.md`, ADR-339, ADR-285 D16 and the p557g report.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p557h-number-literals.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
