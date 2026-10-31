---
agent: builder
model: opus
max_turns: 80
budget_usd: 20
---
# Brief: #1024, a fn parameter lent to a kept trait-method position is refused with `NK2101`, not by rustc

Score: compiler-bug lane; blocks #1012 step 2 (`b1012b`, test (h)). No new NK code.

## Problem

```nika
trait H {
    fn take(mut self, f: fn(i64) -> i64)
}
struct Keep { held: Vec[fn(i64) -> i64] }
impl H for Keep {
    fn take(mut self, f: fn(i64) -> i64) { self.held.push(f) }
}
fn hand[T: H](mut h: T, s: String) {
    h.take(fn(x) { x + s.len() })
    println(s)
}
```

gives a raw `error: Lifetime may not live long enough.` (rustc leaks through). With `let s = f"a{t}"` local in place of the parameter the checker refuses it with `NK2101`. The refusal of a lambda that borrows at a kept position counts locals, not fn parameters.

## Locations
* The `NK2101` path in the checker (grep `NK2101` under `crates/nikaia/src/check/` and `crates/nikaia-std/src/tools/check_*.nika`; the kept-arguments path, `kept_args`). Find why a captured fn parameter is not seen as borrowed there, and count it like a local. A parameter the lambda only copies (a `Copy` type) is not refused.

## To build
1. The fix, and a test in `tests/compiler/tests/`: the probe above with its `.stderr` naming `NK2101`, and a second function where the lambda captures an `i64` parameter (accepted, prints). `lower-std` fixpoint clean if a tools file moved. Commit `check: a fn parameter lent to a kept position is refused with NK2101 (#1024)`, `Fixes #1024`.

## Done when
The test passes through `nikaia test`, `a_trait_method_callback_is_handed_on.nika` still passes, the guard set of `.claude/agents/builder.md` is green; pushed. Changelog: one line.

## Stop and report if
An edit outside the checker (`crates/nikaia/src/check/`, `crates/nikaia-std/src/tools/check_*.nika` and their lowering), `crates/nikaia/src/contracts/`, and the new test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1024-kept-param-lent.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
