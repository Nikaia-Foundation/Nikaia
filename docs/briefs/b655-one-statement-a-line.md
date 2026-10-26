---
agent: builder
model: sonnet
max_turns: 90
budget_usd: 15
---
# Brief: #655 step 4a, two statements on one line are a parse error

Score: bugs lane (`docs/priorities.md`): S=3 (the parser accepts `println!("hi")` as two statements; a reader and a model are both misled), O=small, C~3 M (b655 step 3 run), area parser.

## Issue text (#655, item 3 first half, step 4)
`println!("hi")` parses as two statements, `println` and `!"hi"`, and `1 2` on one line parses too. To build 4: two expressions on one line are a parse error. Tests.

## Probe (architect, 252bcf2, `target/fast/nikaia check t.nika`)
`fn main() {\n    println!("hi")\n}` and `fn main() {\n    let a = 1 2\n}` both print `Checked t.nika`. Wanted: a parse error at the second statement, saying a statement begins a line.

## Decision (architect; specification text to add with the build)
Add to Part I 3.1 (`docs/specification/10-nikaia-light.md`, after the first code block, ~line 919), verbatim:
> **A statement begins a line.** Two statements on one line are a parse error: `println!("hi")` is not `println` and then `!"hi"`, and `let a = 1 2` is not `let a = 1` and then `2`. The first statement of a block may stand on the line of its `{` (`{ return 0 }`), and a jump's trailing condition (`return 0 if done`) is part of its statement.
No new NK code: it is a parse error like the others (no code).

## Locations
* `crates/nikaia/src/parser/mod.rs`: `rule stmt_list` (2245, `stmts:stmt*`), `rule stmt` (2248); the helper `same_line` (180) and `Trivia::began_a_line` (153) already answer "is this token at the start of a line" (ADR-135 D3). Make every statement after the first in `stmt_list` require that it begins a line (a negated `same_line`, or a new `new_line` helper beside it, consuming nothing).
* Check `tools/*.nika`, `examples/`, `tests/`, `packages/` still parse: `target/fast/nikaia lower-std` (see builder-facts) and the test runs below. Code of ours that writes two statements on one line is rewritten onto two lines in the same commit (that is the fix, not a workaround).

## To build
1. The parser change and the spec sentence above.
2. Tests in `tests/compiler/tests/`: `two_statements_on_one_line_are_refused.nika` + `.stderr` (the `1 2` probe), `a_bang_after_a_name_is_not_a_second_statement.nika` + `.stderr` (the `println!` probe), and `a_statement_may_follow_its_brace.nika` + `.stdout` (`if true { println("a") }` and `return 0 if x` on one line run).
Run: `cargo test --profile fast -p nikaia --test it -- parse`, `target/fast/nikaia test --both-settings --project tests/compiler`. Commit `Part of #655` (steps 1, 2, 4b, 5 stay open).
Changelog: a statement begins a line; two on one line are a parse error (Part I 3.1).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b655-one-statement-a-line.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built; how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
