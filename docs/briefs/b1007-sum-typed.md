---
agent: builder
model: sonnet
max_turns: 80
budget_usd: 15
---
# Brief: #1007, `sum()` of a typed number sequence lowers without a type

Score: bug lane, a false acceptance (the checker passes, rustc refuses; S=2) in a one-line form any move that adds numbers writes; touches the emitter's method-call path only. Runs after k3a2 (bootstrap week: moves first; this blocks no move). Builders have no GitHub: the issue text is copied below.

## Problem

`sum()` over a sequence of a typed number list lowers to a bare `.sum()`; rustc cannot infer the result type when nothing annotates it (found by b1006, 2026-10-30). #931 (5162370e) wrote the type on the `let` only for an *open* number literal list.

## Probe (run on d2986cd9)

```nika
fn main() {
    let v: Vec[i64] = [3, 4]
    println(v.iter().sum())          // rustc: type annotations needed, Sum<i64>
}
```
* `let s = v.iter().sum(); println(s)`: same error (lowered `let s = v.iter().copied().sum();`).
* `println(total(v) + v.iter().sum())`: rustc `Cannot satisfy <i64 as Add<_>>::Output == _`.
* `let t: i64 = v.iter().sum()` works; `count()`, `max() ?? 0` work.

## Locations
* `crates/nikaia-std/src/tools/emit_core_calls.nika` ~1183: `collect`'s turbofish (`::<Vec<_>>`) in `emit_core_method_call_body`; `sum` gets none.
* #931's fix: `check_core_method.nika`, `check_flow.nika`, `check_numbers4.nika` (the checker knows `sum`'s result type: `Seq::sum` is `[T](self: Seq[T]) -> T` in `std.contracts:3146`).
* Test beside: `tests/compiler/tests/a_number_list_literal_summed_is_typed.nika`.

## To build
1. Lower `sum()` with a turbofish of its checked result type (`.sum::<i64>()`) whenever the call has no declared target, in programs and in `tools/*.nika` (`lower-std` fixpoint: only files that call `sum` change, or none). Test `tests/compiler/tests/a_typed_list_summed_needs_no_annotation.nika` + `.stdout` with the three probe rows and an `f64` row. `Fixes #1007`.

Expected `.stdout` of the new test (rows run by the architect on d2986cd9; all but the annotated ones fail today): `v.iter().sum()` printed `7`; `let s = v.iter().sum()` printed `7`; `total(v) + v.iter().sum()` `14`; `let t: f64 = w.iter().sum()` with `w = [1.5, 2.0]` and the bare `println(w.iter().sum())` print what `println` prints for `3.5` (take it from the run, say so in `Body:`). Any turbofish rustc accepts is fine (`.sum::<i64>()`); a `ref` element type stays `Sum<&i64>` on the Rust side, the turbofish names the result.

## Done when
The test passes through `nikaia test` (copy it to the scratchpad to `nikaia run` it alone: a file inside `tests/compiler` is not the project's entry), `tests/compiler/tests/a_number_list_literal_summed_is_typed` still passes, the `lower-std` fixpoint is clean, the gate of `.claude/agents/builder.md` is green; pushed. Commit subject: `emit: sum() names its result type (#1007)`, `Fixes #1007`. Changelog: one line (a program that was refused by rustc now runs).

## Stop and report if
The checker has no result type at the call (then the fix is in the checker: `Found:` with the probe); an edit outside `emit_core_calls.nika`, its lowering, `lowered.rs`'s index line and the test; an error in another agent's file.

**Not yours, never stage**: `crates/rs2nika/convert/nikaia.{contracts,derived,proofs}` and every `docs/briefs/*` file that `git status` shows modified or untracked except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1007-sum-typed.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
