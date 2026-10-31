---
agent: builder
model: opus
max_turns: 80
budget_usd: 20
---
# Brief: #1025, a parameter a kept lambda captures is kept (owned), not left lent for rustc to refuse

Score: compiler-bug lane (rustc leaks through a checked program); found by #1024's build (ac8a38a6). No new NK code.

## Problem

A lambda handed to a position the callee keeps, capturing a lent `String` parameter, with no use of the parameter afterwards, passes the checker and reaches rustc:

```nika
fn keep(f: fn(i64) -> i64) -> Vec[fn(i64) -> i64] {
    let mut v: Vec[fn(i64) -> i64] = []
    v.push(f)
    v
}
fn hand(s: String) -> i64 {
    let v = keep(fn(x) { x + s.len() })
    v.len()
}
fn main() {
    println(hand("abc"))
}
```

gives `error: Lifetime may not live long enough.` at line 7 (probed at a3ec9600). The same holds for a trait method that keeps (`h.take(fn(x) { x + s.len() })`, #1024's probe without the `println(s)`).

Controls (probed): with `let s = f"a{t}"` a local in place of the parameter it runs and prints `1` (the lambda takes the local); with `println(s)` after the call it is refused with `NK2101` (#1024).

## Cause

The keeps walk (`crates/nikaia-std/src/tools/keeps.nika`) reads a lambda argument's body as running during the call (`keeps_blocks_in`, line 467: `Expr::Closure { body, .. } => keeps_block(..)`), so a parameter the lambda names is not recorded as kept even when the callee keeps that position. `spawn` is the model: `keep_what_a_task_names` (line 890) records everything a task names as kept (ADR-312 D1).

## Locations
* `keeps_blocks_in` keeps.nika:467; `keep_argument` keeps.nika:~625 (records `Passed { parameter, callee, at }` for the fixpoint); `keep_method_arguments`; `keep_what_a_task_names` keeps.nika:890.
* Checker side already done by #1024: `takes_of_a_lambda`, `check_lambdas.nika:103`.

## To build
1. A lambda argument at position `at` of a call: the parameters its body names are recorded as handed to `(callee, at)` (`Passed`, so the fixpoint asks the callee's column; an unknown callee keeps, D2), for free calls and method calls alike; a lambda at a position the callee does not keep stays as it is (its body runs during the call). Then `hand` keeps `s`, `s` lowers owned and the probe prints `1`. Tests in `tests/compiler/tests/`: the probe (`.stdout` `1`), the trait-method form (`tests/compiler/tests/a_lent_parameter_kept_by_a_trait_method_is_taken.nika` without its `println(s)`, plus `println("ok")` in `main`; `.stdout` `ok`), and a lambda handed to a non-keeping callee that stays lent and is used afterwards (`fn twice(f: fn(i64) -> i64) -> i64 { f(1) + f(2) }`, `let n = twice(fn(x) { x + s.len() })`, `println(s)`, `n`; prints `abc` then `9`; all three rows probed at a3ec9600: the first two give the rustc error, the third runs). `lower-std` fixpoint clean. Commit `keeps: a parameter a kept lambda names is kept (#1025)`, `Fixes #1025`.

## Done when
Tests pass through `nikaia test`; `a_lent_parameter_kept_by_a_trait_method_is_taken.nika`, `a_trait_method_callback_is_handed_on.nika` and the guard set of `.claude/agents/builder.md` are green; pushed. Changelog: one line.

## Stop and report if
An edit outside `crates/nikaia-std/src/tools/keeps.nika` and its lowering (`lowered/keeps.rs`), the Rust caller of the keeps walk (`crates/nikaia/src/contracts/keeps.rs`) if a new field must reach it, `crates/nikaia/tests/it/keeps.rs` only to keep it passing, and the new tests; an error in another agent's file. Read `docs/builder-facts.md` first.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1025-kept-lambda-names.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
