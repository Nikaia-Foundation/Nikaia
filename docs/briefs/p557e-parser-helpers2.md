---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
# Brief: #557 step 4b, the parser's helpers into Nikaia, second slice: the tree helpers, `Parsed`'s logic, the grammar-type copy (parser lane, Part #557)

Score: the parser port (owner, 2026-11-02, #1188: port to Nikaia, helpers first; step 4a answered no, so the rules are ported side by side and switched in one commit). The first slice (52ea0a2c, `tools/parse_findings.nika`) moved the error conversion and the findings. This slice moves the rest of the helpers that do not run the Rust grammar. S=1, C~3-5 M.

**Owns:** in `crates/nikaia/src/parser/mod.rs`:
* `flat_shape` (:21) and `lambda` (:37), the action helpers. They go beside the other action helpers in `tools/tree_build.nika`; the `grammar!` actions call the lowered functions.
* The logic of `Parsed`'s methods `span_of`, `kept_function_call`, `kept_function_text`, `keeping`, `growing`, `text` and `unaliased` (:402-620). `Parsed` itself stays a Rust struct until step 5; what stays is a thin forward with an unchanged signature, so no caller changes.
* `expand_grammar_types` and `put_rule_types` (:856-973). Their logic goes into `tools/grammar_types.nika`, beside what they already call there.
* The new or changed `.nika`, their generated `tools/lowered/*`, and new tests under `tests/compiler/` if needed.

**Not yours:**
* What runs the Rust grammar, which stays until step 5: `hole`, `read_expression`, `parse_expression*`, `stops_at*`, `parse_unrefined`.
* `Trivia` and `doc_here`/`same_line`/`new_line` (they become rules in step 4c).
* The number functions (:194-317, lexical rules calling `parse_numbers` in step 4c).
* Every rule of the `grammar!` block.
* `crate::text_tiers` (it is another lane's, M15b). If `expand_grammar_types` needs its visitors in Nikaia, use what Nikaia already has, or stop at that piece and report.
* `winnow-grammar` and `winnow` (the owner's crates).
* The files of the bug builder running at the same time: `tools/emit_*.nika`, `tools/check_*.nika`.

## To build
1. `flat_shape` and `lambda` into `tools/tree_build.nika`. Commit `parser: flat_shape and lambda from tools/tree_build.nika (#557 step 4b)`, `Part: #557`.
2. `Parsed`'s logic, each method as its own piece. Commit after each finished piece or group, with `Part: #557`.
3. `expand_grammar_types`/`put_rule_types` into `tools/grammar_types.nika`. Commit with `Part: #557`.

A piece whose body needs a form Nikaia, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*:
* absent on purpose (name the ADR): build it the way that ADR says;
* not decided, or not thought of: stop that piece, keep what is committed, and report it with a probe of a few lines;
* a defect: fix it in the compiler as its own commit, at most 2 defects, then stop.

`rs2nika` is deleted (docs/builder-facts.md): translate by hand; `Rs2nika: none`.

## Done when
The owned pieces are Nikaia, or a blocker is reported with its probe. `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before; a test that fails only because the binary vanished mid-run is re-run singly. The `it` tests filtered on `parse`, `grammar` and `span`, plus the guard set of `.claude/agents/builder.md`. Clippy clean; the lower-std fixpoint holds. No changelog line. The `scripts/self_hosting.py` row for a new file goes into `Body:`.

## Stop and report if
a `.stderr` of an existing test changes; the move needs a change in `winnow-grammar`; you meet a missing form of kind 2 (above).

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p557e-parser-helpers2.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
