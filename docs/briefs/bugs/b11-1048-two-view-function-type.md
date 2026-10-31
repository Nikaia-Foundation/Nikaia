---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief: #1048, a function type that may pause, with two view parameters and a view result, has no lifetime

Score: compiler bug, false acceptance (`nikaia check` passes, rustc *Missing lifetime specifier*), text and lists alike; one cause traced, one branch to widen.

Facts: Where things live, Traps already met, Commands

## Issue #1048 (copied; rows a-c probed by the architect on the fast binary at 1e5b0a7c)
```nika
fn longer(a: ref String, b: ref String) -> ref String {
    if a.len() >= b.len() {
        return a
    }
    return b
}
fn apply(g: fn(ref String, ref String) -> ref String, a: ref String) -> i64 {
    return g(a, a).len()
}
fn main() {
    println(f"{apply(longer, "abc")}")
}
```
Rows:
* a: the probe - rustc *Missing lifetime specifier* at line 7 (the NK1261 warnings about `ref String` are expected, leave them; in the test write `String` without `ref` if the probe still fails that way, and say so in `Body:`). Must print `3`.
* b: the same over `ref Vec[i64]` (`longer(a: ref Vec[i64], b: ref Vec[i64]) -> ref Vec[i64]`, `apply(longer, xs)` with `xs = [1, 2, 3]`) - the same rustc error. Must print `3`.
* c (runs today, prints `3`, must stay): row a with the parameter type written `fn(ref String, ref String) -> ref String sync`.
* d (not probed): `g` kept, not run: a struct field `f: fn(ref String, ref String) -> ref String` set to `longer` and called through the struct. Probe it first; if it fails with the same rustc error it is part of this fix, if it runs it is a guard row; say which under `Tests:`.

## Suspected location (traced, sure for a-c)
* `crates/nikaia-std/src/tools/emit_core_types.nika`, the code-type branch (from :131). The #703 branch (:152-189) writes `for<'call> Fn(&'call str, &'call str) -> &'call str` when the result is a view and more than one parameter is lent, but only under `if code.is_sync && code.result != null`. A function type without `sync` may pause, so it falls through to `impl AsyncFn({params}) -> {outcome}` (:230-232, `walk.code_parameter_runs`) or `impl Fn(..) -> Pin<Box<dyn Future<Output = ..>>>` (:233) with elided lifetimes, which rustc refuses for two lent inputs. That is why row c (`sync`) runs.
* The fix: the same rule for code that may pause: count the lent parameters and the view result as :162-170 do, and when `lent > 1` and the result is a view, write the parameters and result with `EmitLifetimes::Call` under one `for<'call>`: `impl for<'call> AsyncFn(&'call str, &'call str) -> &'call str` for a run parameter; for the boxed-future form and the kept form (:206-216, `emit_boxed_future_in`, `BoxedIn<'call, ..>`) the outcome also takes `'call`. Any lowering rustc accepts that keeps one lifetime over all lent parameters and the result (the caller's result lives as long as the shorter one) is right; say in `Body:` which forms you wrote.
* No checker change expected. Lists go through `a_list_read_as_a_slice` as in the sync branch (#1016).

## To build
1. Before any edit, run rows a-d on the current binary; keep the results under `Tests:`.
2. The fix above. Test `tests/compiler/tests/a_function_type_with_two_views.nika` + `.stdout`: rows a, b, c (and d), one `apply` each, printing their answers. Commit `emit: a function type that may pause, with two views in and a view out, names one lifetime (Fixes #1048)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings` (by name); the `tests/compiler` tests whose names contain `function_type`, `code_param` or `lambda` pass (`ls tests/compiler/tests | grep -E ...`, run by name); `lower-std` fixpoint clean (no `tools/*.nika` change expected: `lowered/emit_core_types.rs` only); build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `emit_core_types.nika`, its lowered twin and the test; rustc refuses every `for<'call>` form over `AsyncFn` (then report `blocked` with the lowered line and rustc's message); an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b11-1048-two-view-function-type.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
