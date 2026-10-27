---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: a proved name in the arm of a `let`-bound `if` is not given up whole (Fixes #888)

Score: bugs lane (`docs/priorities.md`, bugs-first default; #888 is a checker-passes/rustc-fails bug, found by the #879 builder; reproduced by the architect on develop 993664a).

## Issue text (#888, narrowed by the architect's probe)
d89aba7 (ADR-382, #879) makes a name proved not `null` the value itself when the function hands it back whole, by `return` or as its tail (`check_arm_leaves.nika`: `a_narrowed_read_given_back`, `narrowed_reads_given_back`). A value bound by `let` is not covered. Probe on 993664a:
```nika
fn t2(n: i64) -> String {
    let b: String? = if n > 0 { f"i{n}" } else { null }
    let s: String = if b != null { b } else { "none" }
    s
}
fn main() { println(t2(3)); println(t2(0)) }
```
`nikaia check` passes; `nikaia run` fails in rustc, "Mismatched types" at the `b` in the `then` arm (lowered `b.as_ref().unwrap()`). The `match` form `let s: String = match b { null => "none", x => x }` already runs (control). A lent *parameter* `a: String?` in the same forms is refused with NK1103/NK1104: that is #882's territory (main-session lane), do not touch it.

## Locations
`crates/nikaia-std/src/tools/check_arm_leaves.nika:342` (`a_narrowed_read_given_back`, `:356`, `narrowed_reads_given_back`, `narrowed_reads_given_back_in`, added by d89aba7); callers `check_core_stmt.nika` (the `return` case) and `check_function.nika` (the tail, `if !tail_view`). The `let` statement's check is in `check_core_stmt.nika` (find the `Stmt::Let` arm with `nikaia run scripts/where.nika -- NAME`). Test of d89aba7: `tests/compiler/tests/a_proved_value_handed_back_whole_is_given_up.nika`.

## To build
1. Probes that fail on today's develop: the program above; the same with `let s = if ...` (type inferred) and with an assignment `s = if b != null { b } else { "x" }` to a `let mut s: String`. Controls: the `match` form above; `let s: String = if b != null { b.clone() } else { "none" }`.
2. Checker: the value of a `let` (and an assignment) whose type is owned text or another owned non-copied type is given up as a `return` is: call `narrowed_reads_given_back` on it. The read is a move of `b`: the same rule as `let s: String = t` for a plain `String` local `t` applies (probe what the checker does when `b` is read after the `let`, and make the narrowed form do the same; if that form is refused, the narrowed one is refused with the same code). A view-typed `let` (`ref String`, or a local the checker makes a view) stays as today.
3. Test `tests/compiler/tests/a_proved_value_bound_by_let_is_given_up.nika` with `.stdout`: every form of step 1 and the controls. Last commit `Fixes #888`.

## Done when
The new test; `nikaia test tests/compiler --both-settings`; the guard set of `.claude/agents/builder.md`; `lower-std` unchanged or changed only where a tools file used the form (say which under `Tests:`). Changelog: one line.

## Stop and report if
An open design choice (for example: the move makes a later read of `b` refused where today it compiles); an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b888-let-bound-proved-name.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
