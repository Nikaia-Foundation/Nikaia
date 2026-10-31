---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief: #1057 step 2, a non-copy field of an element read through `??` or a `let`, handed back, reaches rustc

Score: compiler bug, false acceptance (`nikaia check` passes, rustc refuses); step 1 (b10, 44e16442) built the plain row, these two are its siblings. One checker function, two arms.

Facts: Where things live, Traps already met, Commands

## Issue #1057, step 2 (copied; rows probed by the architect on the suite binary at 2831276b)
```nika
struct Inner { name: String }
struct Item { inner: Inner }
fn last_inner(path: Vec[Item]) -> Inner? {
    return (path.last() ?? return null).inner
}
fn main() {
    let xs = [Item { inner: Inner { name: "a" } }, Item { inner: Inner { name: "b" } }]
    let r = last_inner(xs) ?? return
    println(r.name)
}
```
Rows (same `Inner`/`Item`/`main`, body of `last_inner`):
* b: the probe - rustc *Cannot move out of a shared reference*; want NK1131 naming `path`, help ``Write `(path.last() ?? return null).inner.clone()` for a copy.`` or a shorter help that names the field (say which in `Body:`).
* c: `let s = path.last() ?? return null` / `let x = s.inner` / `return x` - rustc *Mismatched types* (`&Inner`); want NK1131 at `return x` (or at the `let`, where the field is taken by value; say which and why in `Body:`).
* a (refused NK1131 since b10, must stay): `let s = path.last() ?? return null; return s.inner`.
* d (prints `b`, must stay): `return (path.last() ?? return null).inner.clone()`, and row c with `let x = s.inner.clone()`.
* e (refused NK1131, must stay): `let s = path.last() ?? return null; return s` (#1044).

An unfixed binary for before/after probes: `/home/user/nikaia-suite/target/fast/nikaia`.

## Suspected location
* Row b (fairly sure): `check_core_a_lent_place_given_away`, `crates/nikaia-std/src/tools/check_core_leaves.nika:119`, the `Expr::Field` arm (:161). `state.place_path(world, value)` gives no path when the base is an `Expr::Coalesce`, so the arm falls through. b9's fix for #1055 (0788a29c, *an optional field read off a ?? result is a view of the element*) already tells a field read off a `??` result: reuse how it finds the element read (`git show 0788a29c`), then refuse through `check_core_leaves_an_element_read_given_away` with `part = true` when the field's type `does_not_copy`.
* Row c (unsure, trace first): `let x = s.inner` with `s` a view of an element read probably binds `x` with `view_of == ""`, so the `Expr::Variable` arm (:128) returns at once. Find where a `let` sets `view_of` (`nikaia run scripts/where.nika -- view_of`, `check_core_stmt.nika`'s let) and whether a field of a view local, not copying, should make `x` a view of the same read (then `check_core_leaves_list_of_a_read(local.view_of)` refuses it as for row a), or should be refused at the `let` as a move out of a view. Pick the one Part III's NK1131 entry and ADR-282 D7 (an element past a jump is a view, never copied) support, and name it.

## To build
1. Before any edit, run rows a-e on the current binary; keep the results under `Tests:`.
2. Rows b and c. Tests: add both rows to `tests/compiler/tests/a_field_of_an_element_read_handed_back.nika` + `.stderr` (b10's test) as functions of their own, and the two copies of row d to `a_field_of_an_element_read_copied.nika` + `.stdout`. Commit `check: a field of an element read through ?? or a let, handed back by value, is NK1131 (Fixes #1057)`.

## Done when
Both tests fail on the unfixed compiler for the new rows and pass through `nikaia test --project tests/compiler --both-settings` (by name), with `an_element_read_handed_back_by_value`, `an_element_read_copied_with_clone` and the tests whose names contain `coalesce` or `element_read`; `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. If Part III's NK1131 entry does not cover the new rows' wording, extend it (`docs/specification/30-nikaia-tooling.md`, as b10 did). Changelog: one line. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`; row b alone done is `partial` with row c's trace.

## Stop and report if
A `tools/*.nika` of the toolchain is refused by the new rule (that is a true finding: report file and line under `Found:`, do not loosen the rule); an edit is needed outside `check_core_leaves.nika`, `check_core_stmt.nika`, their lowered twins, Part III and the tests; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start, and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b13-1057-field-of-a-coalesce-and-let.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
