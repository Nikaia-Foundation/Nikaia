---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: #1016 continued, a `ref Vec[T]` result (plain and nullable) lowers as `&[T]` like its parameter (rustc E0308)

Score: S=3 (a builder was stopped on it: b3-1016 ended `partial` with a working patch, only for want of one file; a false acceptance: the checker passes, rustc refuses), O=0.5 M, C=2 M (the patch exists), area bugs. First in this batch.

## What the last run found (b3-1016 report)

The patch `docs/briefs/bugs/b4-1016.patch` (copied from the last run's scratchpad) and the test draft `docs/briefs/bugs/b4-1016.test.nika` / `b4-1016.test.stdout` (covering a function value too) make both probes print 2, `f(v).clone()` then push give 3, and trait method, method field result, typed `let` and `for` over the result run. It broke passing a function as a value: `fn(ref Vec[i64]) -> ref Vec[i64]` keeps its result `&Vec<i64>` in `emit_core_types.nika:139-145` (a code type's result, `code.result -> inner`), so it must get `a_list_read_as_a_slice` as its parameters do; also the Call-lifetime result near `emit_core_types.nika:172`. If the patch is gone, rebuild it from the brief's locations below.

**Architect's decision (2026-10-31):** `emit_core_types.nika` is yours in this brief. And a `ref Vec[T]?` result follows the same rule (`Option<&[T]>`): probed today, it fails the same way:

```nika
fn f(a: ref Vec[i64]) -> ref Vec[i64]? {
    if a.len() == 0 {
        return null
    }
    return a
}
fn main() {
    let v = [1, 2]
    let n = f(v)?.len() ?? 0
    println(f"{n}")
}
```

rustc: *expected `&Vec<i64>`, found `&[i64]`* at `return a`. Expected `2`. If a nullable `ref Vec[T]` *parameter* still lowers as `Option<&Vec<T>>`, make it `Option<&[T]>` too, so parameter and result agree; if that grows beyond this run, build the plain result (step 1) alone, and report `partial` naming what is left.

## The issue's probes (copied)

```nika
fn f(a: ref Vec[i64]) -> ref Vec[i64] {
    return a
}
fn main() {
    let v = [1, 2]
    println(f"{f(v).len()}")
}
```

Expected `2`; lowered today `fn f(a: &[i64]) -> &Vec<i64> { a }`. And the enum probe of #1016 (`inner(a: ref Vec[P]) -> ref Vec[P]` returning a field `parts` of a variant), expected `2`.

## Locations

* Parameters: `emit_function.nika` (`plain_list`, `by_three`, `&Vec<` -> `&[`), `emit_ops.nika` `a_list_read_as_a_slice` (#458).
* Result: `emit_function.nika` near `// Kap 7.1` (`let mut returned`, `match ret_type`); `emit_item.nika` (`written = f"&[{element}]"`) for a trait method.
* Code types: `emit_core_types.nika:139-145` and `:172`. Other spellers: `nikaia run scripts/where.nika -- a_list_read_as_a_slice`.

## To build

1. A `ref Vec[T]` result (not `mut`) lowers as `&[T]` with today's lifetime, in a function, a method, a trait method and a function type; one commit.
2. The nullable result `ref Vec[T]?` the same (`Option<&[T]>`), with the parameter if needed (above).
3. Test `tests/compiler/tests/a_list_view_result_is_a_slice.nika` with `.stdout`: both issue probes, the nullable probe, a caller that copies (`let mut c = f(v).clone()`, `c.push(3)`, prints `3`), a function passed as a value. Run against the unfixed compiler first; then the tests whose names hold `slice`, `ref_vec`, `view_result`, `function_value`.

Yours: `emit_function.nika`, `emit_item.nika`, `emit_ops.nika`, `emit_core_types.nika`, their lowered files and `lowered.rs`, the new test. Another file needed: stop, report `partial` with what and why.

The commit that finishes it says `Fixes #1016` and removes the three attachments (`git rm docs/briefs/bugs/b4-1016.patch docs/briefs/bugs/b4-1016.test.nika docs/briefs/bugs/b4-1016.test.stdout`).

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b4-1016-list-view-result-slice.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
