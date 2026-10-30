---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
# Brief: #1008 defect 1, the impl's method of a trait takes the trait method's `keeps`

Score: bug lane, a false acceptance (checker passes, rustc refuses; S=2) that blocks K3a part 2 (`comptime_computed` hands its callbacks to `ComptimeComputeHost`). Follow-up of `b1008-trait-fn-param.md`: its defect 2 (a kept fn value handed on is cloned) is built in d8a045fc; its report found that defect 1 is decided in the contract pass, outside that brief's files. This brief allows that file. A defect (kind 3: ADR-277 D11 says a body-less trait method keeps its `fn` parameters, and an impl must match its trait), no owner decision needed.

## Problem

A body-less trait method keeps its `fn` parameters (ADR-277 D11), so the parameter lowers as `nikaia_std::func::Kept<dyn Fn..>`. The impl's method gets `keeps` from its own body only (`uses_of` in `crates/nikaia/src/contracts/keeps.rs` `infer`; the `Item::Trait` arm gives the trait's entry its keeps), so `Plain::ask` keeps nothing, lowers `impl Fn` and does not match the trait: rustc E0049. The same column drives the checker's call-site facts (`check_core_args.nika` `check_core_args_runs` -> `kept_args`: `p.ask(known)` lowers `&&*known`, rustc "Mismatched types").

## Probe (`nikaia run`, a0f4021c)


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

Today (after d8a045fc): E0049 at the impl's `fn ask` and, with `let known: fn(ref String) -> i64? sync = fn(name) {..}` in `main` and `p.ask(known)` called directly, "Mismatched types" at the call. Expected: `6` and `3`.

## Locations
* `crates/nikaia/src/contracts/keeps.rs` `infer`: for a method of an `impl Trait for T`, union the trait method's keeps by parameter position (the trait's entry is computed in the `Item::Trait` arm); then the emitter (`emit_function.nika` ~565) and the checker (`kept_args`) read the right column with no change of their own.
* Test beside: `tests/compiler/tests/` (new file below).

## To build
1. In `keeps.rs` `infer`, the impl method of a trait keeps (at least) what the trait method keeps, by parameter position. Test `tests/compiler/tests/a_trait_method_callback_is_handed_on.nika` + `.stdout`: the probe (`6`), the direct `p.ask(known)` row with a typed let-bound lambda (`3`), and a recursive row `fn down[T: H](host: ref T, n: i64, known: ..) -> i64` handing `known` to `host.ask` and to itself (`down(p, 3, known)` prints `9`). `lower-std` fixpoint clean (only files whose lowering changes; a tools file whose impl of a trait now keeps changes). `Fixes #1008`.

## Done when
The test passes through `nikaia test` (copy it to the scratchpad to `nikaia run` it alone: a file inside `tests/compiler` is not the project's entry), the free-function control rows still print `6` and `9`, the `lower-std` fixpoint is clean, the gate of `.claude/agents/builder.md` is green; pushed. Commit subject: `contracts: an impl's method keeps what its trait method keeps (#1008)`, `Fixes #1008`. Changelog: one line (programs handing a callback through a trait method now run).

## Stop and report if
An edit outside `crates/nikaia/src/contracts/keeps.rs`, `tools/emit_*.nika` / `tools/check_*.nika` and their lowering, `lowered.rs`'s index lines and the test; an error in another agent's file. The fix in `keeps.rs` needs the trait's declaration where it is not reachable from `infer` (then `Found:` with where).

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
