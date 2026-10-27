---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: a `catch` on a call used as a statement drops the call's value (#885)

Score: bugs lane first (`docs/priorities.md`; CLAUDE.md *Bugs before moving*): `nikaia check` passes a program rustc refuses, found by rs2nika lane H (#877), on the code #884 (0.0.1239) just touched.

## Issue #885 (copied)
**Probe** (`half` needs its statements on separate lines):

```nika
enum Refused { Negative }
impl Error for Refused { fn message(ref self) -> String { return "negative" } }
fn half(n: i64) -> i64 throws {
    if n < 0 { throw Refused::Negative }
    return n / 2
}
fn main() {
    let mut failed = false
    half(-4) catch { failed = true }
    half(4) catch {}
    print(failed)
}
```

**Seen:** `nikaia check` passes, and rustc refuses the lowered code: "match arms have incompatible types". The `Ok` arm is the `i64`, the `Err` arm is `()`.
**Expected (decided, architect 2026-10-27):** a call used as a statement drops its value, so its `catch` block need not hand one back. Prints `true`. Not NK1103: a statement has no value to match.

## Locations
* Emitter: `crates/nikaia-std/src/tools/emit_core_flow.nika:829` `emit_core_try_catch`; the `Ok` arm is written at :944-951 (`Ok(value) => value`, and the #884 forms `Some(value)` / `value.to_owned()` chosen by `self.facts.nullable_catches` / `owned_catches`). Called from `emit_expr.nika:381`.
* Checker: `crates/nikaia-std/src/tools/check_core_flow.nika:1004` `check_core_flow_try_catch`; the #884 pattern records a fact at :1061 (`state.facts.nullable_catches.insert(node_key(guarded_expr))`); the fact sets live in `check_facts.nika:341` / :538.
* Statement position: `Stmt::Expr` arms in `emit_below.nika`, `emit_core_below.nika`, `emit_core_reads.nika`; `nikaia run scripts/where.nika -- NAME...` for the rest.
* Tests beside: `tests/compiler/tests/a_catch_that_hands_back_null_widens_to_nullable.nika` (+`.stdout`), `a_catch_that_hands_back_another_type_is_refused.nika` (+`.stderr`).

## To build
1. A `catch` whose guarded expression is a whole `Stmt::Expr` lowers with its success value dropped (`Ok(_) => {}`) and its handler's value dropped too, whatever the block hands back. How the emitter learns the position (a fact the checker records like `nullable_catches`, or the statement emitter telling `emit_core_try_catch` through its flow) is your choice: say which in the commit message. A `catch` in a `let`, an argument, a `return` or a block's last expression that gives the block's value is unchanged (the existing catch tests show it).
2. Test `tests/compiler/tests/a_catch_on_a_call_statement_drops_its_value.nika` + `.stdout`: the probe, plus a statement `catch` whose block hands back a value (`half(1) catch { 0 }`), and one inside a loop body. Run it against the unfixed compiler first (say so under `Tests:`).

## Done when
`lower-std` rerun (the lowered files of the touched `.nika` change, nothing else); `nikaia test --both-settings --project tests/compiler -- catch` and the guard set of `.claude/agents/builder.md`; `it` filter `catch`. Commit: `a `catch` on a call used as a statement drops the call's value (Fixes #885)`. Changelog: one line (*a `catch` on a call used as a statement no longer needs its block to hand back the call's type*).

## Stop and report if
A statement `catch` turns out to be refused or typed elsewhere by the specification text (Part I 7.1) in a way that contradicts the decision above: report `blocked` with the quote.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b885-catch-statement.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
