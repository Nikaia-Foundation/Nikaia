---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: a `T?`-typed `let` whose arm is a proved name (Fixes #893)

Score: bugs lane (`docs/priorities.md`, bugs-first default): `nikaia check` passes, rustc refuses; found by the #888 builder.

## Issue text (#893)
## Probe
```nika
fn f(b: String?, c: String) -> String? {
    let s: String? = if b != null { b } else { c }
    return s
}
fn main() { println(f("x", "y") ?? "n") }
```
`nikaia check` passes; rustc: *Mismatched types* at the `b` arm (`b` is written as the proved `String` while the `let` wants `Option<String>`; the `c` arm also needs `Some(..)`). Fails before and after #888 (3b77d7c), which handled only a `let` whose type is owned whole and not optional.

## To build
1. A proved name at an arm tail of a value bound at a `T?` position (a `let s: T? = ...`, an assignment to a `T?` place, a `return` from a `-> T?` function) is written as the option itself (`b`, not `b.unwrap()` re-wrapped), and a non-optional arm is wrapped in `Some(..)`. Find where `return` of the same form is lowered and use the same path (`nikaia run scripts/where.nika -- narrowed_reads_given_back`). Test: `tests/compiler/tests/a_proved_arm_bound_at_an_optional_let.nika` + `.stdout`, forms `let`, assignment, `return`, with `String` and a struct.

Rules: ADR-382 (a proved name handed back whole), ADR-352.

## Done when
The new test; `nikaia test tests/compiler --both-settings`; `it` filtered on `narrow nullable handed_over`; the guard set of `.claude/agents/builder.md`; clippy clean. Commit with `Fixes: #893`; `Changelog:` one sentence.

## Stop and report if
The fix needs a rule ADR-382 / ADR-352 do not give (an open design choice); an error in another agent's file.

Read `docs/builder-facts.md`.

* Not yours (another lane's uncommitted files in the checkout, 2026-10-27): `crates/nikaia-std/src/tools/emit_core_calls.nika`, its `lowered/emit_core_calls.rs`, `crates/rs2nika/convert/**`. Never stage them; if `lower-std` rewrites `lowered/emit_core_calls.rs`, leave it out of your commit and say so in `Body:`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b893-optional-let-proved-arm.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
