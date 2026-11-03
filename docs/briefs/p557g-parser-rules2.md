---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
# Brief: #557 step 4c, chunk 1 continued: two grammar defects (#1196, #1197), then the literal rules of `tools/parser_grammar.nika` (parser lane, Part #557)

Score: the parser port (owner, 2026-11-02, #1188: port to Nikaia, helpers first; the rules are ported side by side and switched in one commit at step 5). Step 1 of chunk 1 is built (0479949f: trivia, `///`, keywords, names, reserved words; report `p557f-parser-rules1.md.report.md`, read it first). This brief fixes the two defects that report found and builds step 2. S=1, C~4-6 M.

**Owns:**
* The grammar front end, checker and lowering parts the two defects need (as their own commits, each with a test), `crates/nikaia-std/src/tools/parser_grammar.nika`, its `tools/lowered/parser_grammar.rs`, and new tests under `tests/compiler/`.

**Not yours:**
* The Rust `grammar!` block in `crates/nikaia/src/parser/mod.rs`: not changed, stays the parser in use until step 5.
* Every lowercase (syntactic) rule; they wait on #1198 (ADR-388, the caller's interner table) and are later chunks. Do not build #1198.
* `winnow-grammar` and `winnow` (the owner's crates). A rule the grammar language cannot say is a finding, not a patch there.
* The files of other builders: `tools/emit_*.nika`, `tools/check_*.nika`, `keeps.nika`, `describe*.nika`.

## To build
1. **#1196** (defect): a rule `-> String` whose alternatives are a bound name and a text literal (`n:raw_ident -> { n.clone() } | "_" -> { "_" }`) has no form that passes (NK1104 without `.clone()`, NK1301 with it). Build the issue's *To build* list, test `grammar_text_literal_alternative.nika`. Commit `Fixes #1196`.
2. **#1197** (defect): a labelled `span(...)` over two or more elements loses the names bound inside it in the action (check passes, run says "Cannot find value `n`"; one element works). Build the issue's *To build* list, test `grammar_span_binds_inner_names.nika`. Commit `Fixes #1197`.
3. The literal rules (step 2 of p557f): `STRING STR_CHAR CHAR CHAR_BODY DIGITS DIGIT_GROUP FLOAT EXPONENT`, `number_lit`/`negative_number_lit`/`float_end` whose actions call `tools/parse_numbers.nika`, `SEGMENT`, `G_BUILTIN G_BUILTIN_WORD`, and of the f-string family what does not need `expr` (`F_TEXT F_TEXT_CHAR F_SPEC`; `F_HOLE`/`F_PART`/`F_STRING` wait for the expression chunk; say so in the commit message). Tests as in p557f step 1 (expected output from what the Rust rule gives for the same inputs), plus the number literals of ADR-285 D16 and a refused one whose message must equal the Rust grammar's. Commit `Part: #557`.
4. The comment list (ADR-337 D2) as the p557f report proposed: a lexical scan of its own over `COMMENT`, `BLOCK_COMMENT` and the literals, each comment a `span`. Test with ADR-307 D1/D2 nesting and a `//` inside a string. Commit `Part: #557`.
5. In the last commit message, the comparison proposal of p557f (pg_render over every `tests/compiler` and `tests/errors` program against `nikaia check`) made concrete for chunk 2: which tool, which output, in Nikaia only.

A rule whose body needs a form Nikaia, the grammar language, `std` or the toolchain lacks is sorted per CLAUDE.md *A missing form stops the work*: absent on purpose (name the ADR): build it that way; not decided or not thought of: stop that piece, keep what is committed, report it with a probe of a few lines; a further defect: no third fix in this brief, file nothing, report it under `Found:` with its probe.

`rs2nika` is deleted (docs/builder-facts.md): translate by hand; `Rs2nika: none`.

## Done when
#1196 and #1197 fixed with tests; the literal rules and the comment scan are Nikaia with tests, or a blocker is reported with its probe. `nikaia test tests/compiler --both-settings` and `tests/errors` give every count as before plus the new tests; a test that fails only because the binary vanished mid-run is re-run singly. The `it` tests filtered on `parse`, `grammar` and `span`, plus the guard set of `.claude/agents/builder.md`. Clippy clean; the lower-std fixpoint holds. No changelog line.

## Stop and report if
a `.stderr` of an existing test changes (other than one the two fixes make right, named in the commit); the work needs a change in `winnow-grammar`; you meet a missing form of kind 2 (above).

Read `docs/builder-facts.md`, ADR-339 and the p557f report.

## Rules (CLAUDE.md; shared checkout)
* Read `docs/builder-facts.md` first. You work in `/home/user/Nikaia` on `develop` (the shared checkout). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/Nikaia/target/fast/nikaia`.
* `git add <files>` only, never `-a`/`-A`; commit after each step. Never `git reset`, `stash`, `checkout .`, `restore .`; restore only your own files by name. An error in another agent's file is escalated, not touched.
* Commit every lowered file `lower-std` changed, yours and others'. Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before committing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `nikaia run scripts/pre-push.nika`, and the gate of `.claude/agents/builder.md`. You commit; the supervisor pushes.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk is small: `NIKAIA_CACHE_LIMIT=2G`; if `df -h /home/user` shows under 2 GB free, stop and report `blocked`. Never delete another checkout's `target/`.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/p557g-parser-rules2.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
