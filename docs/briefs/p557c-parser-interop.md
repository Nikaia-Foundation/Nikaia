---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: #557 step 4a, can the Rust `grammar!` block call a rule of a lowered Nikaia grammar? (parser lane, Part #557)

Score: the parser port (owner, 2026-11-02, #1188: port to Nikaia, helpers first). `parser/mod.rs` is about 2.2 points of the self-hosting metric. This probe decides how the port switches: chunk by chunk (the Rust block calls ported rules while the rest stays Rust) or in one commit at the end (both grammars side by side, compared, then switched). S=1, C~2-4 M.

**Owns:** a probe of your own in your scratch directory; if the answer is yes, a new `crates/nikaia-std/src/tools/parser_grammar.nika` holding **one** small leaf rule (e.g. an identifier or a punctuation rule the Rust block uses in several places), its generated `tools/lowered/*`, and the one call site in `crates/nikaia/src/parser/mod.rs` that uses it instead of the Rust rule; new tests under `tests/compiler/` if the step needs any. **Not yours:** every other rule of `parser/mod.rs`, `emit_grammar.nika` (if the lowering needs a change for this, stop and report it under `Found:`/`Needs:`), `winnow-grammar` and `winnow` (the owner's crates: never patch them).

## Issue text (#557 step 4 and the #1188 decision, copied)
* Port in chunks into a `.nika` grammar, running `tests/` and `tests/errors/` after each; `///` as a rule; the number rules become lexical rules calling `parse_numbers`.
* Decided in #1188: first check whether the Rust `grammar!` block can call rules of the lowered Nikaia grammar (both come from the same `winnow-grammar` macro). If it can, the switch goes chunk by chunk; if not, the chunks are compared side by side and switched in one commit at the end.

## To build
1. **Probe.** A lowered Nikaia grammar is a `grammar! { grammar Name { ... } }` block (see `crates/nikaia-std/src/tools/lowered/ledger.rs:33`, from `tools/ledger.nika:67`). Find out how one `grammar!` block can call a `pub rule` of another: a path, a `use`, a plain function with winnow's parser signature, or not at all. Check the three things that must agree: the input type (the compiler's grammar runs over `Bytes`/`&[u8]`, `docs/bootstrap.md` *The parser*), the state (`state Trivia` and the `WS` skip: does a rule called across the boundary skip whitespace the same way, and does the trailing-trivia cut of `at`/`span` still hold), and the error type (`ParseError` and its messages). Write the probe as a few lines outside the repository first.
2. **If yes:** move one leaf rule into `tools/parser_grammar.nika` and call it from the Rust block; `nikaia test tests/compiler` and `tests/errors` give the same results as before. Commit `parser: first rule from tools/parser_grammar.nika (#557 step 4a)`, `Part: #557`. **If no:** commit nothing; the report's `Body:` says what is missing (the exact error of the probe), and `Found:` names it only if it is a defect of the lowering in our own code.

## Done when
Step 1 answered with a probe; for yes, step 2 built with `nikaia test tests/compiler --both-settings` (every count as before) and the `it` tests filtered on `grammar` plus the guard set of `.claude/agents/builder.md`; clippy clean; lower-std fixpoint. No changelog line (no language change).

## Stop and report if
the call needs a change in `winnow-grammar` (say what, the decision is the supervisor's); the whitespace or span behaviour differs across the boundary; a `.stderr` of an existing test changes.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p557c-parser-interop.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
