---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: #557 steps 1-2, the grammar language gets `span(p)` and `here` (compiler lane, Part #557)

Score: the bootstrap week's **critical path** (`docs/bootstrap-week.md`, section 2): `parser/mod.rs` (2,280 Rust lines) moves as a Nikaia grammar only after #557 steps 1-3. S=3, C~8-15 M. Runs first in the fix checkout C3.

**Owns:** `crates/nikaia/src/parser/mod.rs` (only the rules that parse a grammar's patterns, `grammar_item`/`grammar_rule` at :1896/:1906 and the pattern rules they call), `crates/nikaia-std/src/tools/ast.nika` (only `Pattern`, :1343), `check_grammar.nika`, `emit_grammar.nika`, `grammar_decode.nika`, `crates/nikaia/src/grammar_run.rs` and `build_time.rs` only if the build-time run of a grammar needs the new forms, new tests under `tests/compiler/`, the generated `tools/lowered/*`. **Not yours:** everything a move lane owns (`docs/bootstrap-week.md`, *File ownership*); `Pattern` gains variants, so every `match` over it in another lane's file that stops compiling is a `Needs:` (look first: `nikaia run scripts/where.nika -- Pattern`).

## Issue text (#557, copied; decided: ADR-339, Part II 10.8)
* `span(p)` yields the byte range `p` matched; `here` yields a position and consumes nothing. Part II 10.8's table (`docs/specification/20-nikaia-advance.md:648-650`): over `Bytes`, `span(p)` gives a `Span` (two `u32`) and `here` a `u32`; over text a `text::Match` / `text::Pos`.
* ADR-383's comment on #557: `span(p)` must end a span at its last token (the trailing-trivia cut `PrTrivia::cut` that `at` applies), so the span table the numbering pass writes stays byte-identical when the compiler's grammar moves.
* **Decided for this week (architect, `docs/bootstrap-week.md` section 2):** the compiler's grammar will run over the source as `Bytes`, so build the `Bytes` forms. The text forms wait for #743 (`text::Pos`, `text::Match` are not in `std`): over text, `span(p)`/`here` are refused with the existing "not built yet" path or an existing NK code (no new code; if none fits, stop and report under `Needs:`).

## To build
1. **Check first** (#557 step 1): an action that fails inside an alternative behaves as 10.8's cut says, and its message survives in `ParseError`. Test `tests/compiler/tests/grammar_failing_action_in_alternative.nika` (`rule n -> i64 = t:text(digit+) { parse(t) }` where `parse` fails, inside `a | b`) with its `.stdout`/`.stderr`. If it already holds, the test alone is the step; if not, fix it. Commit, `Part: #557`.
2. **`span(p)` and `here` over `Bytes`** (#557 step 2): the grammar front end (the pattern rules in `parser/mod.rs`, `Pattern` in `ast.nika`), the checker (`check_grammar.nika`: the type of the binding), the lowering (`emit_grammar.nika`: what the generated `winnow-grammar` rule writes; the Rust grammar's own `@=` and `.at()` show how the crate hands back a range) and `grammar_decode.nika` if it decodes patterns. Tests: a `Bytes` grammar whose rule binds `s:span(p)` gives the same `start`/`end` as the bytes `p` covered (also after skipped whitespace: the span ends at the last token); `here` consumes nothing (`a:here b:here` around nothing are equal); both in a build-time run of the grammar if `build_time.rs` runs grammars (one test). Commit `grammar: span(p) and here over Bytes (#557 step 2)`, `Part: #557`.

Step 3 (`same_line`) is the next brief, not this one.

## Done when
`nikaia test tests/compiler --both-settings` (every count as before plus the new tests); `it` tests filtered on `grammar` plus the guard set of `.claude/agents/builder.md`; clippy clean; lower-std fixpoint after the rebase. Changelog: one line for the supervisor under `Changelog:` (a language change raises the version; you do not).

## Stop and report if
`winnow-grammar` cannot hand back the range of a sub-pattern (then say what it offers: the decision is the architect's); a `.stderr` of an existing test changes; a `Pattern` match in another lane's file needs an arm.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; worktree lane, `docs/bootstrap-week.md` section 3)
* Read `docs/builder-facts.md` first. **You work in the worktree `/home/user/nikaia-fix`, not in `/home/user/Nikaia`** (the driver's builder works there; never edit, build or stage anything in it except your report file). Set up once: `cd /home/user/Nikaia && git fetch origin develop && (test -d /home/user/nikaia-fix || git worktree add --detach /home/user/nikaia-fix origin/develop)`; then `cd /home/user/nikaia-fix && git status` (another agent of your checkout may have left files: not yours, report them). Build with `CARGO_INCREMENTAL=0 cargo build --profile fast -p nikaia`; use only `/home/user/nikaia-fix/target/fast/nikaia`, never another checkout's binary.
* `git add <files>` only; commit after each step. Push: `git fetch origin develop && git rebase origin/develop`, rebuild, `lower-std` once more, commit any lowered file that changed (`lowered: regenerate after rebase`), then `git push origin HEAD:develop`; rejected: again, at most three times, then `Pushed: rejected`. A conflict in `tools/lowered/*.rs` or `tools/lowered.rs` is never merged by hand: `git checkout --theirs -- <file>`, `git rebase --continue`, rebuild, `lower-std`, commit.
* Commit every lowered file `lower-std` changed, yours and others' (a compiler change re-lowers other files; that is correct in your tree). Do not edit `docs/selfhosting-plan.md`, `docs/architect.md`, `docs/priorities.md`, `docs/bootstrap-week.md` or `scripts/self_hosting.py`: rows for them go into `Body:` of your report.
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most three checkouts with `target/` this week (C1, C2, C3); if `df -h /home/user` shows under 4 GB free, delete `/home/user/nikaia-fix/target` and report `blocked`. At the end delete `/home/user/nikaia-fix/target/debug` and `/home/user/nikaia-fix/target/release/nikaia-scratch` (keep `/home/user/nikaia-fix/target/fast` for the lane's next brief) and report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/lane-bugs-557a-span-here.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
