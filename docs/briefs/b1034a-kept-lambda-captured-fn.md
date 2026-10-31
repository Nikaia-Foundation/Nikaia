---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: #1034 step 1, a kept lambda that calls a captured kept fn (rustc)

Score: compiler bug (false acceptance: the checker passes, rustc refuses; every lambda that composes function values), bugs-first default of `docs/priorities.md`. Found by b1031, narrowed by the architect at 39f8dbf6.

## Issue #1034 (copied, step 1)
Part I 5.4, *A function type stands wherever a type does*: "the lambda moves what it captures, and a copy of the value shares the one closure".
```nika
fn main() {
    let f: fn(i64) -> i64 = fn(x) { x + 1 }
    let w: fn(i64) -> i64 = fn(x) { f(x) * 2 }
    println(f"{w(3)}")
}
```
`nikaia check` passes; rustc: *Cannot move out of `f`, a captured variable in an `Fn` closure*. Lowered (`nikaia lower`): `Kept(Arc::new(move |x| { let __nikaia_future: BoxedIn<'_, i64> = Box::pin(async move { f(x).await * 2 }); __nikaia_future }))`: the per-call `async move` moves the closure's own `f`. The outer `move` also takes `f`, so `f(1)` after the lambda gives *Borrow of moved value*. A struct field lambda (`Box2 { run: fn(x) { f(x) * 10 } }`) is the same.

Model that works (verified, a captured list `v`): `{ let v = Arc::new(v); Kept(Arc::new(move |x| { let v = v.clone(); { ...async move { x + v.len() } } })) }`. A captured hull (`Shared`, ADR-230 D4) is cloned in a block around the closure and once per call. A `Kept` is already an `Arc`, so a captured local of `fn` type wants exactly the hull treatment: `let f = f.clone();` around the closure and per call.

Step 2 of #1034 (a `fn` parameter captured by a returned lambda, `run_code` in `check_function.nika`) is **not** this brief's.

## Locations
* `check_lambdas.nika:94` `hulls_of_a_lambda` (records `facts.kept_hulls` per lambda), `:89` `a_name_of_a_hull`, `:40` `a_hull_type`. Asked for a kept lambda in a `let`, a field, and a parameter the callee keeps.
* `emit_core_flow.nika:217` `emit_flow_hulls`, `:328` `emit_flow_kept` (clones each hull before the closure, and per call when `pauses`).
* Which names hold a `Kept` (#1031's rule, reuse it): `check_core_expr.nika:316` `check_core_expr_kept_copy`: a local found by `lookup` and not in `state.run_code`, or a field; a named function is no place.
* Model test: `tests/compiler/tests/a_kept_fn_copied_from_a_name_or_field.nika`.
* Suggested shape (any that passes the test is fine): in `hulls_of_a_lambda`, also push a name whose binding is `Ty::Fn` and not in `run_code`; rename nothing, add a doc line that a `Kept` is a handle like a hull.

## To build
1. Test `tests/compiler/tests/a_kept_lambda_calls_a_captured_fn.nika` + `.stdout` (`10`, `13`, `20`, `2`, `2`, one a line), the program below; run by the architect at 39f8dbf6: three rustc errors (*Cannot move out of `f`/`g`*, lines 16 and 19). Commit `emit: a kept lambda that calls a captured fn clones its handle (#1034)`, `Part: #1034` (not `Fixes`: step 2 stays open).

```nika
struct Box2 {
    run: fn(i64) -> i64
}

fn inc(x: i64) -> i64 {
    return x + 1
}

fn dbl(x: i64) -> i64 {
    return x * 2
}

fn main() {
    let f: fn(i64) -> i64 = inc
    let g: fn(i64) -> i64 = dbl
    let w: fn(i64) -> i64 = fn(x) { f(x) + g(x) }
    println(f"{w(3)}")
    println(f"{w(4)}")
    let b = Box2 { run: fn(x) { f(x) * 10 } }
    println(f"{b.run(1)}")
    println(f"{f(1)}")
    println(f"{g(1)}")
}
```
Traced to the named cause: the `let w` rows (line 16). The field row (line 19) gives the same rustc message, unverified that it goes through the same `kept_hulls` path. If one row has a second cause, name it and its rustc message under `Found:`, commit the test without that row, and say so in `Body:`.

## Done when
The test passes through `nikaia test` and fails on the unfixed compiler; `a_kept_fn_copied_from_a_name_or_field` and the hull tests (`nikaia run scripts/where.nika -- kept_hulls` lists them) still pass; `lower-std` fixpoint clean; `cargo build --profile fast -p nikaia`; the guard set of `.claude/agents/builder.md` green; pushed. Changelog: one line (a lambda kept in a `let` or a field that calls a function value it captures shares that value's closure, instead of a lowering rustc refused). Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `crates/nikaia-std/src/tools/check_lambdas.nika`, `check_facts.nika`, `emit_core_flow.nika`, their lowered files (and `lowered.rs`) and the new test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1034a-kept-lambda-captured-fn.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
