---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief: #997, a let-bound lambda with a `String` parameter reaches rustc

Score: bug lane; blocks K3a part 2 (`k3a2-comptime-compute.md`, last in this batch: `comptime_run.rs:393` binds `known` by `let`). S=1, a lowering defect on the self-hosting path. Builders have no GitHub: the issue text is copied below.

## Problem

A lambda bound by `let` with an untyped `String` parameter, then handed on as `fn(String) -> T? sync`, passes the checker and reaches rustc: it lowers as `|n| known(&n)`, rustc infers `n: str` (E0277, `Sized`). The same lambda written straight into the call runs. With a map read as the tail, `fn(n) { more.get(n) }` also misses the `.copied()` (`Option<&i64>` for `i64?`). Found by b995 (0cf0b173); blocks K3a part 2 (#125): `comptime_run.rs:393` is `let known = |name| known(name).or_else(|| ahead.get(name).cloned())`.

## Probe (target/fast at 0cf0b173)

```nika
fn known(n: String) -> i64? {
    if n == "a" { return 1 }
    return null
}
fn ask(f: fn(String) -> i64? sync, k: String) -> i64? {
    return f(k)
}
fn main() {
    let w = fn(n) { known(n) }
    print(ask(w, "a") ?? 0)
}
```

rustc: `The size for values of type str cannot be known` at :9. Expected `1`. Second row: a local `BTreeMap[String, i64]` `more`, `let w = fn(n) { known(n) ?? more.get(n) }` and `let v = fn(n) { more.get(n) }`, each through `ask`. Third row: the element a `String` / an enum with a `String` part (`?.clone()` written, per ADR-293 D15).

## Locations

* Lowering of a lambda: `emit_core_closure`, `crates/nikaia-std/src/tools/emit_core_flow.nika:229` (writes `|{names}|` without types); kept lambdas `emit_flow_kept`.
* The type the checker gives a let-bound lambda's parameter: follow `Expr::Lambda` in `check_core_leaves.nika` / `check_core_args.nika` (`nikaia run scripts/where.nika -- emit_core_closure`).

## To build

1. Probe on a fresh build; rows 1-3.
2. Lower a let-bound lambda so rustc accepts it (any lowering rustc accepts: e.g. the parameter's type written, `|n: &str|` or `|n: String|`, matched to how the call site passes it), and its map-read tail copied as a `return` of it is.
3. Test `tests/compiler/tests/let_lambda_string_param.nika` + `.stdout` (rows 1-3), through `nikaia test`; `lower-std` fixpoint; the `it` guard set. Commit `Fixes #997`.

## Notes

Read `docs/builder-facts.md`; `CLAUDE.md` rules for builders; report form of `docs/architect.md`, *Briefs and reports*.
* Do not touch: the uncommitted files `git status` shows at start (`crates/rs2nika/convert/nikaia.*`, `docs/selfhosting-plan.md`, the k1/k3a1 drafts, other reports): another session's. Changelog: one sentence.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b997-let-lambda-string-param.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
