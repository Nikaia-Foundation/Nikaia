---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: #1016 step 3, a `ref Vec[T]` with a named lifetime, and a typed `let` of one, lowers as a slice (rustc E0308)

Score: S=3 (false acceptance: the checker passes, rustc refuses; found by b4-1016 under `Found:`), O=0.3 M, C=2 M (emit only, the rule b4 built for the plain case), area bugs. Third in this batch.

## What b4-1016 built (6832794b) and left

A `ref Vec[T]` parameter and result lower as `&[T]`. Two places still write `&Vec<T>`, so a slice reaches them and rustc refuses. Probed by the architect on 3adb2b60:

```nika
fn longer(a: ref Vec[i64], b: ref Vec[i64]) -> ref Vec[i64] {
    if a.len() >= b.len() {
        return a
    }
    return b
}
fn twice(a: ref Vec[i64]) -> i64 {
    return longer(a, a).len()
}
fn typed(a: ref Vec[i64]) -> i64 {
    let w: ref Vec[i64] = a
    return w.len()
}
fn main() {
    let v = [1, 2]
    println(f"{twice(v)}")
    println(f"{typed([4, 5, 6])}")
    println(f"{longer(v, [7]).len()}")
}
```

Lowered today: `fn longer<'r>(a: &'r Vec<i64>, b: &'r Vec<i64>) -> &'r Vec<i64>` (a function with two view parameters and a view result takes named lifetimes, and then keeps `&'r Vec<T>`), so `longer(a, a)` in `twice` (`a: &[i64]`) is *Arguments to this function are incorrect* at 8:5; and `let w: &Vec<i64> = a` is *Mismatched types* at 11:5. Expected output `2`, `3`, `2`. Both causes traced in the lowered `.rs`; the hand-edited Rust (`&'r [i64]`, `let w: &[i64]`) is plain coercion.

Not in this brief (filed separately by the architect): a function *type* with two view parameters and a view result (`fn(ref Vec[i64], ref Vec[i64]) -> ref Vec[i64]`, also with `String`) lowers to `Fn(&[i64], &[i64]) -> &[i64]` with no lifetime (rustc *Missing lifetime specifier*); and the nullable `ref Vec[T]?` (b4's step 2, `Option<&Vec<T>>` today, consistent on both sides except a plain view returned as a nullable result), which is #1016 step 4.

## Locations

* Parameters with a named lifetime: `crates/nikaia-std/src/tools/emit_function.nika:539-549` (`reference = "&'r "`), `:597-612` (`plain_list`, `by_three` already lists `&'r `: find why the named form escapes it, e.g. the lifetime added later at `:707` / `:824-846` `emit_renamed_lifetime`).
* Result: `emit_function.nika:765-775` (`plain_list` -> `a_list_read_as_a_slice`, `emit_ops.nika:213`): make it see `&'r Vec<..>` too.
* Typed `let`: `emit_stmt.nika:260` (`Stmt::Let`), the declared type's spelling; the rule is the parameter's (`a_list_read_as_a_slice` on a `ref Vec[T]`, not `mut`, not nullable).
* Read `docs/builder-facts.md`.

## To build

1. A `ref Vec[T]` parameter and result with a named lifetime (`'r`, `'r2`, ...) lowers as `&'r [T]`; one commit.
2. A typed `let x: ref Vec[T] = ..` lowers its type as `&[T]`; one commit (or with 1).
3. Test `tests/compiler/tests/a_list_view_named_lifetime_is_a_slice.nika` with `.stdout` (`2`, `3`, `2`): the probe above. Run it against the unfixed compiler first; then `a_list_view_result_is_a_slice` and the tests whose names hold `slice`, `ref_vec`, `lifetime`.

Yours: `emit_function.nika`, `emit_stmt.nika`, `emit_ops.nika`, their lowered files and `lowered.rs`, the new test. Another file needed: stop, report `partial` with what and why. The commit says `Part of #1016` (step 4 is left), and removes b4's attachments: `git rm docs/briefs/bugs/b4-1016.patch docs/briefs/bugs/b4-1016.test.nika docs/briefs/bugs/b4-1016.test.stdout`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b7-1016-named-lifetime-and-typed-let-slice.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
