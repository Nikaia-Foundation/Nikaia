---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: #1016, a `ref Vec[T]` result lowers as `&Vec<T>` while its parameter lowers as `&[T]` (rustc E0308)

Score: S=3 (a false acceptance: the checker passes, rustc refuses, on the shortest view-returning function there is; found by the yaml lane, y6), O=0.5 M, C=3 M, area bugs.

## Issue #1016 (copied; reduced by the architect at c30b7bc0)

Reduced probe:

```nika
fn f(a: ref Vec[i64]) -> ref Vec[i64] {
    return a
}
fn main() {
    let v = [1, 2]
    println(f"{f(v).len()}")
}
```

Expected `2`. Today rustc: *Mismatched types, expected `&Vec<i64>`, found `&[i64]`*. Lowered: `fn f(a: &[i64]) -> &Vec<i64> { a }`.

The issue's probe (also fails, same cause):

```nika
enum P { Seq(Vec[P]), Leaf(i64) }
fn inner(a: ref Vec[P]) -> ref Vec[P] {
    if a.len() == 1 { match a[0] { P::Seq(parts) => return parts, else => {} } }
    return a
}
fn main() {
    let v = [P::Seq([P::Leaf(1), P::Leaf(2)])]
    println(f"{inner(v).len()}")
}
```

lowers to `fn inner(a: &[P]) -> &Vec<P>`. Expected `2`. Parameter and result of the same view type lower to the same Rust type, `&[T]`; a field's `&Vec<P>` (`parts`) coerces to `&[P]` at the `return` by itself in rustc. A `.clone()` of a slice view is already written `.to_owned()` (probed: `let mut b = a.clone()` over a `ref Vec[i64]` parameter runs), so callers that copy the result keep working; check that with a caller that does `f(v).clone()` and pushes.

## Locations

* Parameters: `crates/nikaia-std/src/tools/emit_function.nika:597-612` (`plain_list`, `by_three`, `&Vec<` -> `&[`); and `emit_ops.nika:213` `a_list_read_as_a_slice` (#458, the same rewrite for a function value).
* Result: `emit_function.nika:715` on (`// Kap 7.1`, `let mut returned`, `match ret_type`): the result's spelling gets no such rewrite. Check `emit_item.nika:588` (`written = f"&[{element}]"`) for a trait method's result, the same rule.
* Whoever spells a call's result type elsewhere (a `let` with a written type, a kept lambda's signature) must agree; find them with `nikaia run scripts/where.nika -- a_list_read_as_a_slice`.

## To build

1. A `ref Vec[T]` result (not `mut`, not nullable) lowers as `&[T]` with the lifetime it has today, as a `ref Vec[T]` parameter does; a `ref Vec[T]?` result stays as it is unless the parameter rule covers it too (follow what the parameter does, and say so in `Body:`).
2. Test `tests/compiler/tests/a_list_view_result_is_a_slice.nika` with `.stdout`: both probes above, and a caller that copies the result (`let mut c = f(v).clone()`, `c.push(3)`, prints `3`). Run against the unfixed compiler first; then the existing tests whose names hold `slice`, `ref_vec` or `view_result` (`nikaia test` filters).

Only `emit_function.nika`, `emit_item.nika`, `emit_ops.nika`, their lowered files (and `lowered.rs`) and the new test are yours. Another file needed: stop, report `partial` with what and why.

The commit that finishes it says `Fixes #1016`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b3-1016-ref-list-result-slice.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
