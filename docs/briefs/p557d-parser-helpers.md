---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
# Brief: #557 step 4b, the parser's helpers into Nikaia, first slice: the error conversion and the findings (parser lane, Part #557)

Score: the parser port (owner, 2026-11-02, #1188: port to Nikaia, helpers first). Step 4a answered no (`docs/briefs/p557c-parser-interop.md.report.md`): the Rust `grammar!` block cannot call a rule of a lowered Nikaia grammar without a change to `winnow-grammar`, so per #1188 the rules are ported side by side and switched in one commit at the end. The helpers move first, so the Nikaia grammar of step 4c finds them in Nikaia. S=1, C~3-5 M.

**Owns:** in `crates/nikaia/src/parser/mod.rs` the functions `fits` (:677), `byte_of`, `char_of`, `failure_of`, `finding_of`, `unclosed_block_comment`, `a_parse_error` (to :772), `a_types_refusal` (:963), `a_parse_finding` (:1024), `a_mut_name_in_a_tuple` (:1050), `refuse_parsing` (:1074); the Nikaia file they move into (`tools/parse_errors.nika` where they fit its subject, otherwise a new `tools/parse_findings.nika`) and its generated `tools/lowered/*`; new tests under `tests/compiler/` if the move needs any. **Not yours:** `stops_at`/`stops_at_char`, `parse_unrefined`, `parse_expression_from` (they run the Rust grammar: they stay until step 5), `Parsed`, `Trivia`, `expand_grammar_types`/`put_rule_types` (later slices), every rule of the `grammar!` block (:1092 on), `winnow-grammar` and `winnow` (the owner's crates: never patch them).

## Issue text (#557 step 4 and the #1188 decision, copied)
* Port in chunks into a `.nika` grammar, running `tests/` and `tests/errors/` after each; `///` as a rule; the number rules become lexical rules calling `parse_numbers`.
* Decided in #1188: first check whether the Rust `grammar!` block can call rules of the lowered Nikaia grammar. If it can, the switch goes chunk by chunk; if not, the chunks are compared side by side and switched in one commit at the end. (Answered: not.)

## To build
1. **Move** the owned functions into Nikaia; what stays in `parser/mod.rs` is the call into the lowered function, or nothing where the caller can call Nikaia directly. A function whose body needs a form Nikaia, `std` or the toolchain lacks (e.g. building a `crate::check::Finding` from Nikaia, or `anyhow::Error`) is sorted per CLAUDE.md *A missing form stops the work*: absent on purpose (ADR named), build it that way; not decided or not thought of, stop and report `blocked` with a probe of a few lines; a defect, fix it in the compiler as its own commit (at most 2 defects, then stop). Commit `parser: error conversion and findings from tools/<file>.nika (#557 step 4b)`, `Part: #557`.
2. Run `rs2nika --score` on what moved and give the result under `Rs2nika:`.

## Done when
The owned functions are Nikaia (or a blocker is reported with its probe); `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before (no `.stderr` changes); the `it` tests filtered on `parse` plus the guard set of `.claude/agents/builder.md`; clippy clean; lower-std fixpoint. No changelog line (no language change).

## Stop and report if
a `.stderr` of an existing test changes; the move needs a change in `winnow-grammar`; a missing form of kind 2 (above).

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p557d-parser-helpers.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
