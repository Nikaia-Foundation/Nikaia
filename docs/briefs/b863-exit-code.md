---
agent: builder
model: sonnet
max_turns: 70
budget_usd: 10
---
# Brief: `process::exit(2)` in a `throws` helper ends with code 1 (#863)

Score: S=3 (a program asking for exit code 2 gets 1 and a stray error line; our own scripts' escalation codes depend on it), O=unknown until reduced, bug lane 1.

## Issue #863 (copied)
Found by the #858 builder: in a version of `scripts/agent.nika`, a helper marked `throws`, called from a `throws` `main`, calling `process::exit(2)` (`agent_escalate`, line 96 today), ended the program with exit code 1 and `Error: process_Exit` on stderr instead of code 2. Part III, `process::exit`: the requested code reaches the OS.

## Architect's probes (develop 8335430, `target/fast/nikaia run p.nika; echo $?`)
* `fn quit() throws { process::exit(2) }` / `fn main() throws { quit() }`: status 2 (correct).
* `agent_escalate`'s body (`fs::write`, `println`, `process::exit(2)`) called from `main`: status 2 (correct).
So the failing form is not reduced. How exit works: `process::exit` throws `process::Exit` (ADR-345, `crates/nikaia-std/src/process.rs:350-380`); `emit_entry.nika` `main_exits` (about line 302) wraps `main` in `process::ended(outcome)` only when `main`'s contract `fails_with` names `process::Exit`. The text `process_Exit` looks like a variant of a combined error enum: likely failing forms are a `main` (or helper) that also throws a second error type (e.g. `fs::read` with `?`, `text::parse_i64` errors, a `throw` of its own), so `Exit` travels wrapped, and either `main_exits` misses it or `ended` does not unwrap it. Try those, and a helper that the checker finds cannot throw anything but `Exit`.

## To build
1. Reduce: find the smallest failing program (vary as above; `git log -p ba1f1cc -- scripts/agent.nika` shows the version of that time). If nothing fails after a fair matrix (about 8 forms), report `Result: noop` with the matrix under Deviation; the issue is then closed by hand.
2. Test `tests/compiler/tests/process_exit_in_a_throws_helper.nika` (+ `.stdout`/`.stderr`; if the runner cannot check a status, a parent program running the child with `process::run` and printing its code).
3. Fix at the root (`main_exits` in `crates/nikaia-std/src/tools/emit_entry.nika`, or `process::ended` in `crates/nikaia-std/src/process.rs`, or the combined error's lowering), so the code reaches the OS whatever the `throws` chain; `lower-std` and rebuild after a `tools/*.nika` change. Commit `Fixes #863`.

## Files you may change
`crates/nikaia-std/src/tools/emit_entry.nika` (+ its `lowered/` file), `crates/nikaia-std/src/process.rs`, the new test. Anything else: stop and report under Needs.
Known facts: read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b863-exit-code.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
