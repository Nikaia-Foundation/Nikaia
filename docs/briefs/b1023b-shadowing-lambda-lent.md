---
agent: builder
model: opus
max_turns: 70
budget_usd: 15
---
# Brief: #1023 step 2, a let lambda that shadows a `fn` parameter is lent with `&` (blocks K3a part 2)

Score: bug before move; the last of #1023's three defects, it blocks the self-hosting row K3a (k3a2's precondition). Steps 1 and 3 of #1023 are on develop (3b6bf8c7, 90d7e2ad); b1023 stopped on this one because its cause is in the checker, outside its file list. This brief allows the checker files.

## Problem and probe (run by the architect at b4938048: still rustc E0308 "expected `&_`, found closure"; must print `2`)
```
fn alt(name: String) -> i64? sync { return name.len() }
fn use_it(known: fn(ref String) -> i64? sync) -> i64 sync { return known("ab") ?? 0 }
fn outer(known: fn(ref String) -> i64? sync) -> i64 sync {
    let known = fn(name) { known(name) ?? alt(name) }
    return use_it(known)
}
fn main() { println(f"{outer(fn(name) { null })}") }
```
Without the shadowing (`let more = ...; use_it(more)`) it writes `&more`.

## Cause (b1023's finding, checked by the architect)
`CheckState::a_lent_parameter` (`crates/nikaia-std/src/tools/check_fields.nika:213`) matches a `Variable` to the walked function's signature parameters **by name only**. The local `known` that shadows the parameter `known` reads as the lent parameter, so `the_compiler_writes_the_reference` (`check_references.nika:295`, *Nor does a function this function was lent*) writes no `&`. `state.run_code` already follows shadowing (`check_core_stmt.nika:500-502`); `a_lent_parameter` does not.

## Decision (architect)
A name is the parameter only while its innermost binding is the parameter's own: `a_lent_parameter` answers `false` when the innermost binding of the name lies in a frame deeper than the function's parameter frame (the frame `check_function.nika:733` pushes; a lambda's own frames lie deeper, but a capture of the parameter inside a lambda still finds the binding in the parameter frame, so it stays the parameter). Fix it in `a_lent_parameter` itself, so its other callers (`check_arm_leaves.nika:622`, `:629`) get the same answer. If the frame index is not known at that point, add one field to `CheckState` (`check_state.nika`) set where the frame is pushed and restored after, like `run_code`.

## To build
1. The fix above, `lower-std` (fixpoint clean), `cargo check --profile fast -p nikaia-std --features compiler` clean. Test `tests/compiler/tests/a_shadowing_lambda_is_lent.nika` (the probe, `.stdout` `2`). Still pass: `a_lambda_handed_round_a_cycle_runs`, `a_recursive_function_hands_a_new_lambda_to_itself`, `a_trait_method_*`, `a_method_hands_its_callback_to_a_function`, and the guard set of `.claude/agents/builder.md`. Commit `checker: a let that shadows a fn parameter is not the lent parameter (Fixes #1023)`.

## Done when
The test passes through `nikaia test`, the tests named above pass, gate green, pushed. Changelog: one line (a let lambda that shadows a `fn` parameter is lent with `&`).

## Stop and report if
An edit outside `crates/nikaia-std/src/tools/check_fields.nika`, `check_state.nika`, `check_function.nika`, `check_references.nika` and their lowering (`lowered/*.rs`, `lowered.rs`), and the new test; the probe still fails after the fix with a cause in the emitter (report it under `Found:` with the lowered line); an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (among them `crates/rs2nika/convert/nikaia.*`, `docs/selfhosting-plan.md`), and every `docs/briefs/*` file except your report. If `lowered.rs` shows another hunk, stage only yours.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1023b-shadowing-lambda-lent.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
