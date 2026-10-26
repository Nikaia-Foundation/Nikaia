---
agent: builder
model: sonnet
max_turns: 110
budget_usd: 18
---
# Brief: #655 step 4a again, a statement begins a line, with the tests that hold one-line programs

Score: bugs lane (`docs/priorities.md`): S=3 (`println!("hi")` parses as two statements), O=medium (many test strings), C~4 M (first run 1.5 M tokens), area parser + Rust `it` tests.

## Issue text (#655, item 3 first half, step 4)
`println!("hi")` parses as two statements, `println` and `!"hi"`, and `1 2` on one line parses too. To build 4: two expressions on one line are a parse error. Tests.

## History
9936c70 built this (parser `rule stmt_list` in `crates/nikaia/src/parser/mod.rs`, Part I 3.1 sentence, three `tests/compiler/tests/` programs) and was reverted by `b-revert-9936c70.md`: it ran only `--test it -- parse`, and CI then failed in the `it` binary, whose tests hold one-line programs (`fn main() { let n = 10 / 2 println(f"{n}") }`) and in `accepted_compiles` (the corpus). Start from that commit: `git show 9936c70` and re-apply it (`git revert --no-edit <the revert's sha>`).

## Decision (unchanged; Part I 3.1, as in 9936c70)
> **A statement begins a line.** Two statements on one line are a parse error: `println!("hi")` is not `println` and then `!"hi"`, and `let a = 1 2` is not `let a = 1` and then `2`. The first statement of a block may stand on the line of its `{` (`{ return 0 }`), and a jump's trailing condition (`return 0 if done`) is part of its statement.

## To build
1. Re-apply 9936c70 (parser, spec sentence, its three tests).
2. Run the whole `it` binary once: `cargo test --profile fast -p nikaia --test it 2>&1 | grep -E 'FAILED|panicked' > target/it-fail.txt` (no filter). Every failing test whose program puts two statements on one line is rewritten so each statement begins a line (`\n` in the Rust string, or a raw string over lines), its assertion unchanged; this is changing a Rust test only to keep it passing (CLAUDE.md). The same for `.nika` programs of ours (`accepted_compiles` corpus, `tests/`, `examples/`, `packages/`, `tools/*.nika` via `target/fast/nikaia lower-std`). A test that fails for another reason: `Found:`.
   If more than 80 tests fail, commit nothing, report `blocked` with the count and the files, so the architect splits the work.
3. Run again: the whole `it` binary (no filter) passes, and `target/fast/nikaia test --both-settings --project tests/compiler` passes. Then one commit, `A statement begins a line; two on one line are a parse error (Part of #655)`.

Changelog: a statement begins a line; two on one line are a parse error (Part I 3.1).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b655-one-statement-a-line-redo.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built; how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
