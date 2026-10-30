---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: #1008, a trait method's `fn` parameter (impl type and the kept callback's clone)

Score: bug lane, a false acceptance (checker passes, rustc refuses; S=2) that blocks K3a part 2 (`comptime_computed` hands its callbacks to `ComptimeComputeHost` at every step); bootstrap week: a blocker of a move goes first. Builders have no GitHub: the issue text is copied below; every probe row was run by the architect on 3b845e0b, except the recursive trait row of step 1, which cannot run before the fix (its free-function twin prints `9`): if it fails for a reason other than defects 1-2, that is a `Found:`.

## Problem

A body-less trait method keeps its `fn` parameters (ADR-277 D11: run is the absence of `keeps`), so the parameter lowers as `nikaia_std::func::Kept<dyn Fn..>`. Two defects follow, both accepted by the checker and refused by rustc:

1. **The impl's method lowers its `fn` parameter as `impl Fn`** (its body only runs it), so it does not match the trait: rustc E0049 "Method `ask` has 1 type parameter but its trait declaration has 0".
2. **A `Kept` parameter handed on is moved, never cloned**: handed twice (or into a loop, or into a recursive call), rustc "use of moved value". `Kept` is `Clone` (an `Arc`).

## Probe (`nikaia run`, 3b845e0b)

```nika
trait H {
    fn ask(ref self, known: fn(ref String) -> i64? sync) -> i64? sync
}
struct Plain { base: i64 }
impl H for Plain {
    fn ask(ref self, known: fn(ref String) -> i64? sync) -> i64? sync {
        return known("a")
    }
}
fn twice[T: H](host: ref T, known: fn(ref String) -> i64? sync) -> i64 {
    let a = host.ask(known) ?? 0
    let b = host.ask(known) ?? 0
    return a + b
}
fn main() {
    let p = Plain { base: 1 }
    println(twice(p, fn(name) { if name == "a" { 3 } else { null } }))
}
```
Today: E0049 at the impl's `fn ask` (defect 1) and "Use of moved value: `known`" at line 12 (defect 2). Expected: `6`.

* Same program with `let known: fn(ref String) -> i64? sync = fn(name) {..}` in `main` and `p.ask(known)` called directly: also "Mismatched types" at the call (`Kept<dyn ..>` expected, `&&dyn Fn` found); expected `3`.
* Free functions are fine (control row): `fn ask_once(known: fn(ref String) -> i64? sync) -> i64? sync { return known("a") }` called twice from `twice`, and `down(n, known)` recursing with `known`, print `6` and `9` today.

## Locations
* `crates/nikaia-std/src/tools/emit_function.nika` ~564: `let runs = !self.emit_fn_keeps(key, pname)` sets `walk.code_parameter_runs`; for a method of an `impl Trait for T` it must take the trait method's answer (body-less: keeps).
* `crates/nikaia-std/src/tools/emit_core_types.nika` 150-219: `runs` chooses `impl Fn` vs `nikaia_std::func::Kept<..>`.
* Arguments: where a call hands a local/parameter to a parameter lowered as `Kept` (the `kept_calls` paths: `emit_core_calls.nika` ~573, `emit_misc.nika` ~103, `emit_expr.nika` ~496); a `Kept` value read again later (or in a loop, or always: an `Arc` clone is cheap) is handed `.clone()`.
* Test beside: `tests/compiler/tests/` (new file below).

## To build
1. The impl's method of a trait takes the trait method's `keeps` for its `fn` parameters; a `Kept` handed to a `Kept` parameter is cloned where it is read again (any lowering rustc accepts). Test `tests/compiler/tests/a_trait_method_callback_is_handed_on.nika` + `.stdout`: the probe (`6`), the direct `p.ask(known)` row with a typed let-bound lambda (`3`), and a recursive row `fn down[T: H](host: ref T, n: i64, known: ..) -> i64` handing `known` to `host.ask` and to itself (`down(p, 3, known)` prints `9`). `lower-std` fixpoint clean (only files whose lowering changes). `Fixes #1008`.

## Done when
The test passes through `nikaia test` (copy it to the scratchpad to `nikaia run` it alone: a file inside `tests/compiler` is not the project's entry), the control rows (free functions) still print `6` and `9`, the `lower-std` fixpoint is clean, the gate of `.claude/agents/builder.md` is green; pushed. Commit subject: `emit: a trait method's fn parameter is kept in the impl too, and handed on by clone (#1008)`, `Fixes #1008`. Changelog: one line (programs handing a callback through a trait method now run).

## Stop and report if
The checker, not the emitter, decides the impl's type (then `Found:` with where); an edit outside `tools/emit_*.nika`, their lowering, `lowered.rs`'s index lines and the test; an error in another agent's file.

**Not yours, never stage**: `crates/rs2nika/convert/nikaia.{contracts,derived,proofs}`, `docs/selfhosting-plan.md`, and every `docs/briefs/*` file that `git status` shows modified or untracked except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1008-trait-fn-param.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
