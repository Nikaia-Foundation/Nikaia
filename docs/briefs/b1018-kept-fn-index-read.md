---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
---
# Brief: #1018, `let f = fs[0]` of a kept fn value moves out of the Vec (rustc)

Score: compiler bug (false acceptance: the checker passes, rustc refuses), bugs-first default of `docs/priorities.md`.

## Problem (issue #1018, probed by the architect at ed30efd5)

```nika
fn main() {
    let mut fs: Vec[fn(i64) -> i64] = []
    fs.push(fn(x) { x + 1 })
    let f = fs[0]
    println(f(2))
    let g = fs[0]
    println(g(2))
    println(g(2))
}
```
Today: rustc `Cannot move out of a shared reference` at `let f = fs[0]`. Expected `3`, `3`, `3`. Part I (10-nikaia-light.md about line 2234): *a function value in any of them is kept: ... a copy of the value shares the one closure*. So an index read of a `fn`-typed element is a copy: lowered as a clone of the `nikaia_std::func::Kept` handle (an `Arc`, ADR-277 D11), cheap.

Not in this brief: the issue's row `fs[0](2)` does not parse (`Expected ')'`; calling an index result is no form of Part I), and `fs[0].clone()` is NK1245; leave both.

## Locations
* Index read emission: `emit_core_index_expr` in `crates/nikaia-std/src/tools/emit_host.nika:158`, and `emit_core_reads.nika` about line 466-510.
* What counts as copied by value: `a_copy_by_value` in `crates/nikaia-std/src/tools/check_types.nika:89` (and `value_of_a_copy`, line 73). Find how a `String` element read by index is cloned or refused today and follow the path that decides it (checker fact or emitter test); a `fn` type joins the cloned case. If the decision is a checker fact, name it in the commit (`check_facts.nika`, keyed by `node_key`); the checker files are allowed.
* Kept fn types are written in `emit_core_types.nika` about line 185-219 (`nikaia_std::func::Kept<dyn Fn..>`).

## To build
1. An index read of a `fn`-typed element bound by `let` (or handed to a kept position) lowers to a clone of the handle. Test `tests/compiler/tests/a_kept_fn_read_by_index.nika` + `.stdout`: the probe above (`3`, `3`, `3`). Commit `emit: a kept fn read by index shares the closure (#1018)`, `Fixes #1018`.

## Done when
The test passes through `nikaia test`; `lower-std` fixpoint clean; `cargo build --profile fast -p nikaia`; the guard set of `.claude/agents/builder.md` green; pushed. Changelog: one line (a function value read from a list by index shares the closure, Part I).

## Stop and report if
An edit is needed outside `crates/nikaia-std/src/tools/emit_*.nika`, `check_*.nika`, `check_types.nika`, their lowered files and the new test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1018-kept-fn-index-read.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
