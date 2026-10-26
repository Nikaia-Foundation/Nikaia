---
agent: builder
model: sonnet
max_turns: 70
budget_usd: 10
---
# Brief: a view of a `.to_string()` local of a number expression is returned as `&` of a local (#865)

Score: S=3 (a valid program fails in rustc, after the checker accepted it), O=1 file, bug lane; sibling of #864 (6ba5c74, same function).

## Issue #865 (copied)
Found by the #864 builder (6ba5c74), reproduced by the architect on develop d33d355 (`target/fast/nikaia run p.nika`):

```nika
fn f(t: String) -> String {
    let s = t.len().to_string()
    return s.trim()
}
fn main() { println(f("abc")) }
```

rustc: `Cannot return value referencing local variable 's'`. The same shape with `let s = f"{t}!"` (#864's neighbour) and `let s = t.trim().to_string()` runs.

## Cause
`made_here` in `crates/nikaia-std/src/tools/buffers.nika` (about line 435) makes a `let` a buffer of the frame's own (`Buffer::Named("String")`, so a view of it leaving the function lives in the caller's keep) for `x.to_string()` only when `formats(receiver)` holds, and `formats` (about line 535) accepts only a bare name whose written type is not text. `t.len().to_string()`, `5.to_string()`, `(n + 1).to_string()` fall through to `makes_a_buffer`, which answers `Buffer::Unknown`.

## To build
1. Matrix first: `let s = R.to_string(); return s.trim()` for R = `t.len()`, `5`, `(n + 1)` with `n: i64`, `x.0` of a tuple of numbers, `true`, a call `g()` returning `i64`; note which fail.
2. Test `tests/compiler/tests/a_view_of_a_formatted_local_is_returned_owned.nika` + `.stdout` with the failing forms (it must fail before the fix).
3. Fix in `formats`: the text form of a receiver that is not text (ADR-282 D3) makes a buffer. A receiver that is or may be text (a `String`/`ref String` local, a `.trim()`) is #851's open question (`decision`): leave it as today. `lower-std`, rebuild, commit `Fixes #865`.

Files: `crates/nikaia-std/src/tools/buffers.nika` (+ `lowered/buffers.rs`), the new test.
Anything else: stop and report under Needs. Known facts: read `docs/builder-facts.md`. Pattern: `joins_text` (#864) right below `made_here`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b865-to-string-local-view.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
