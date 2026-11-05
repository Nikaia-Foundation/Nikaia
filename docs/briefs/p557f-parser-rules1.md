---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
# Brief: #557 step 4c, first chunk of rules: the lexical layer of the compiler's grammar into `tools/parser_grammar.nika` (parser lane, Part #557)

Score: the parser port (owner, 2026-11-02, #1188: port to Nikaia, helpers first; step 4a answered no, so the rules are ported side by side and switched in one commit at step 5). Step 4b is built (52ea0a2c, 80bf6cda). This is the first chunk of rules: the lexical layer, which every later chunk stands on. S=1, C~4-6 M.

**Owns:**
* A new `crates/nikaia-std/src/tools/parser_grammar.nika` (registered like the other tools), its generated `tools/lowered/parser_grammar.rs`, and new tests under `tests/compiler/`.
* In it, the Nikaia form of the 79 UPPERCASE (lexical) rules of the `grammar!` block in `crates/nikaia/src/parser/mod.rs`: `WSE WS COMMENT BLOCK_COMMENT BLOCK_INNER STRING STR_CHAR SEGMENT G_BUILTIN G_BUILTIN_WORD FLOAT EXPONENT`, every `KW_*`, `RESERVED*`, `NAME UNDERSCORE`, `F_STRING F_PART F_TEXT F_TEXT_CHAR F_HOLE F_SPEC CHAR CHAR_BODY DIGITS DIGIT_GROUP`.
* What the Rust block takes from outside as `extern rule` and `state Trivia`, written the ADR-339 way: `same_line` and `here` from the grammar language; `///` docs as an ordinary rule (ADR-339, ADR-135 D3's cases; a stray `///` is a parse error); `number_lit`/`negative_number_lit`/`float_end` as lexical rules whose actions call `tools/parse_numbers.nika`. The comment list (ADR-337 D2) and the trailing-trivia cut of a span (ADR-383, #557 comment of 2026-10-28) are what the Nikaia form has to give the later chunks; say in the commit message how it gives them.

**Not yours:**
* The Rust `grammar!` block itself: it is not changed and not called from the new grammar (4a: it cannot be). It stays the parser in use until step 5.
* Every lowercase (syntactic) rule; they are the next chunks.
* `winnow-grammar` and `winnow` (the owner's crates). A rule the grammar language cannot say is a finding, not a patch there.
* The files of other builders running at the same time: `tools/emit_*.nika`, `tools/check_*.nika` (bug lane), `keeps.nika`, `describe*.nika`.

## To build
1. `tools/parser_grammar.nika` with the trivia (`WS`, both comments, `///`) and the keyword, name and reserved-word rules. Tests: a `.nika` test per group that runs the rule on inputs and prints what it gave, the expected output taken from what the Rust rule gives for the same input (write the inputs from `tests/compiler` and `tests/errors` programs, including ADR-307 D1/D2 nesting, an unclosed `/*`, and each ADR-135 D3 case). Commit `parser: the trivia and word rules in tools/parser_grammar.nika (#557 step 4c)`, `Part: #557`.
2. The literal rules: strings, chars, f-strings, numbers (calling `parse_numbers`), `SEGMENT`, `G_BUILTIN*`. Tests as in 1, plus the number literals of ADR-285 D16 and a refused one whose message must equal the Rust grammar's. Commit with `Part: #557`.
3. In the commit message of the last commit, propose how later chunks compare the two grammars on every `tests/compiler` and `tests/errors` program (#1188 asks for it), in Nikaia only. If no comparison is possible without new Rust, say so under `Needs:` with the smallest gap.

A rule whose body needs a form Nikaia, the grammar language, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*:
* absent on purpose (name the ADR): build it the way that ADR says;
* not decided, or not thought of: stop that piece, keep what is committed, and report it with a probe of a few lines;
* a defect (the grammar front end, checker or lowering gets a specified form wrong): fix it in the compiler as its own commit with a test, at most 2 defects, then stop.

`rs2nika` is deleted (docs/builder-facts.md): translate by hand; `Rs2nika: none`.

## Done when
The 79 lexical rules and the ADR-339 forms of the externs are Nikaia with tests, or a blocker is reported with its probe. `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before plus the new tests; a test that fails only because the binary vanished mid-run is re-run singly. The `it` tests filtered on `parse`, `grammar` and `span`, plus the guard set of `.claude/agents/builder.md`. Clippy clean; the lower-std fixpoint holds. No changelog line. The `scripts/self_hosting.py` row for the new file goes into `Body:`.

## Stop and report if
a `.stderr` of an existing test changes; the work needs a change in `winnow-grammar`; you meet a missing form of kind 2 (above).

Read `docs/builder-facts.md` and ADR-339.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `cargo build --profile fast -p nikaia` (incremental on, `bootstrap.md` section 3; `CARGO_INCREMENTAL=0` only under 6 GB free); use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p557f-parser-rules1.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
