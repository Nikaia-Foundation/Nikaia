---
agent: builder
model: opus
max_turns: 60
budget_usd: 10
---
# Brief: #1035, `return h.f` of a fn field out of a `ref` parameter is refused NK1131

Score: compiler bug (false refusal of a form Part I allows; `let g = h.f` in the same place runs), bugs-first default of `docs/priorities.md`. Found by b1031, probed by the architect at 39f8dbf6.

## Issue #1035 (copied)
```nika
struct H {
    f: fn(i64) -> i64
}

fn inc(x: i64) -> i64 {
    return x + 1
}

fn pick(h: ref H) -> fn(i64) -> i64 {
    return h.f
}

fn main() {
    let h = H { f: inc }
    let p = pick(h)
    println(f"{p(1)}")
}
```
Refused `NK1131` ("You can't give away `h.f` here: `h` is only borrowed"). Part I 5.4 (*A function type stands wherever a type does*): a copy of a function value shares the one closure, so reading it out of a loan takes nothing away. `let g = h.f` over `h: ref H` already runs, and `return h.n` (i64) runs. Since #1031 (637bec45) the emitter clones a field read of `fn` type (`facts.kept_fn_copies`, `check_core_expr.nika:316`), so `h.f.clone()` should come out.

Cause traced: `check_core_a_lent_place_given_away` (`check_core_leaves.nika:74`) returns early for `copies_plainly(ty)` (`check_types.nika:119`: numbers, bool, char, scalar, views). The `!handed_back` path (line ~163) also returns for `state.copied(world, held)`; the handed-back path (`return`, entered from `check_core_a_field_of_a_lent_place_handed_back`, line 56) asks nothing more, so a `Ty::Fn` is refused there.

## To build
1. A place of `fn` type out of a lent root is no give-away in `check_core_a_lent_place_given_away` (Part I 5.4): return early for a `Ty::Fn` beside `copies_plainly` (do not widen `copies_plainly` itself, it has 17 other callers). Test `tests/compiler/tests/a_fn_field_handed_back_from_a_loan.nika` + `.stdout` (`4`, `2`, `3`, one a line), the program below; run by the architect at 39f8dbf6: NK1131 at line 15. Check the lowered `pick` (`nikaia lower`) clones the field; if it does not, that is rustc's message under `Found:`. Commit `check: a fn field handed back out of a loan is a copy (#1035)`, `Fixes #1035`.

```nika
struct H {
    n: i64,
    f: fn(i64) -> i64
}

fn inc(x: i64) -> i64 {
    return x + 1
}

fn num(h: ref H) -> i64 {
    return h.n
}

fn pick(h: ref H) -> fn(i64) -> i64 {
    return h.f
}

fn main() {
    let h = H { n: 4, f: inc }
    println(f"{num(h)}")
    let p = pick(h)
    println(f"{p(1)}")
    println(f"{h.f(2)}")
}
```
Traced: the `return h.f` row. Untraced, not this brief's: a `fn` element handed back out of a lent list (`return hs[0]`), which takes the `element` path.

## Done when
The test passes through `nikaia test` and fails on the unfixed compiler; the NK1131 tests (`nikaia run scripts/where.nika -- NK1131` lists them; the `.nika` ones under `tests/compiler`) still pass; `lower-std` fixpoint clean; `cargo build --profile fast -p nikaia`; the guard set of `.claude/agents/builder.md` green; pushed. Changelog: one line (a function value handed back out of a borrowed struct is a copy that shares its closure, no longer refused NK1131). Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `crates/nikaia-std/src/tools/check_core_leaves.nika`, `check_types.nika`, their lowered files (and `lowered.rs`) and the new test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b1035-fn-field-from-loan.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
