---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: #1029, a pausing kept lambda typed `-> i64` hands back i32 literals (rustc)

Score: compiler bug (false acceptance: the checker passes, rustc refuses; every typed `let` lambda with a literal result), bugs-first default of `docs/priorities.md`. Found by b1017.

## Issue #1029 (copied)
Found by b1017 (report of ee9aa6a6), widened by the architect's probes at 61df8cf5.

### Problem

A pausing lambda kept under a declared `fn` type whose result is an integer type loses that type: every bare integer literal it hands back (tail, arm, branch, `return`) is `i32` in rustc, and rustc refuses.

```nika
fn main() {
    let k: fn(i64) -> i64 = fn(x) { 5 }
    println(k(1))
}
```
Today: `Expected {async block ...} to be a future that resolves to i64, but it resolves to i32`. Expected `5`.

Lowered: `Kept(Arc::new(move |x| Box::pin(async move { 5 }) as nikaia_std::func::BoxedIn<'_, _>))`. The unsizing coercion to `dyn Fn` does not carry the expected type back into the closure, and `_` leaves the literal to rustc's default `i32`.

Probe matrix (all at 61df8cf5):
| Row | Today |
| :--- | :--- |
| `let k: fn(i64) -> i64 = fn(x) { 5 }` | i32 error |
| `... = fn(x) { match x { 1 => 5, else => 7 } }` | i32 error |
| `... = fn(x) { if x == 1 { 5 } else { 7 } }` | i32 error |
| `... = fn(x) { if x > 9 { return 3 } 8 }` (two lines) | i32 error |
| `let m: fn(i64) -> i64? = fn(x) { if x > 0 { 4 } else { null } }` | `Option<i32>` error |
| `let k: fn(i64) -> u8 = fn(x) { 5 }` | i32 error |
| `let k: fn(i64) -> i64 = fn(x) { x + 5 }` | 6 (fine: `x` fixes it) |
| `let k: fn(i64) -> f64 = fn(x) { 5.0 }` | 5 (fine: f64 is rustc's default) |
| `let k: fn(i64) -> i64 sync = fn(x) { match x {..} }` | 5 (fine: not boxed) |
| `apply(fn(x) { match x {..} })`, `apply(f: fn(i64) -> i64)` | 5 (fine: `AsyncFn` bound) |

### Locations
* The cast: `emit_flow_kept` in `crates/nikaia-std/src/tools/emit_core_flow.nika:395` (`as {emit_boxed_future_in(..)}<'_, _>`); the same cast for a named function kept: `emit_expr_kept_function`, `emit_expr.nika:521` (fine today: the call fixes the type; leave it).
* Where a kept lambda is recorded: `check_core_leaves.nika:229` (`Ty::Fn { params, is_sync, can_throw, .. }` under a declared place, `kept_lambdas.insert`) and `check_core_args.nika:117` (handed to a kept parameter, `result` already bound there). Fact table: `check_facts.nika:242` (`kept_lambdas: BTreeMap[i64, (bool, bool)]`), keyed by `id_of(value)`, which the emitter reads as `node_key(node)`.
* Rust spelling of a `Ty`, `_` where unsure: `plain_rust_spelling`, `emit_text.nika:194`.

### To build
1. The checker records a pausing kept lambda's declared result where it is an integer type or an optional of one (a new fact beside `kept_lambdas`, holding the `Ty`, or the spelling); the emitter writes that spelling in place of the cast's `_` (`BoxedIn<'_, i64>`, `BoxedIn<'_, Option<i64>>`), and keeps `_` everywhere else. A lambda that may fail (`throws`) keeps `_`: not probed, out of this step. Test `tests/compiler/tests/a_kept_lambda_keeps_its_integer_result.nika` + `.stdout`, rows 1-6 of the matrix in one program, printing `5`, `5`, `7`, `8`, `4`, `5` (calls `k1(1)`, `k2(1)`, `k3(2)`, `k4(1)`, `m(1) ?? 0`, `k6(1)`). Commit `emit: a kept lambda's integer result types its literals (#1029)`, `Fixes #1029`.

Rows 1-6 were each traced by the architect to the `_` of the cast (the generated `.rs` read for row 1; rows 2-6 give the same rustc message at the same column). The b1017 builder tried `state.number_asked` on the lambda tail; it did not help (and could not reach a `return`), so the fix is the cast.

## Done when
The test passes through `nikaia test`; `lower-std` fixpoint clean; `cargo build --profile fast -p nikaia`; the guard set of `.claude/agents/builder.md` green; pushed. Changelog: one line (a lambda kept under a declared `fn` type with an integer result hands back that type's literals).

## Stop and report if
An edit is needed outside `crates/nikaia-std/src/tools/emit_*.nika`, `check_*.nika`, `check_facts.nika`, their lowered files (and `lowered.rs`) and the new test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1029-kept-lambda-int-result.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
