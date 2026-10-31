---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief: #947, a `get ... ?? return` over a list of literals, compared, reaches rustc (`&&str` vs `&str`, `&{integer}` vs `{integer}`)

Score: compiler bug, false acceptance (`nikaia check` passes, rustc refuses), on a common form; the copy fact exists and is too narrow. Held back for #938, which is closed.

Facts: Where things live, Traps already met, Commands

## Issue #947 (copied; rows probed by the architect on the suite binary at 2831276b)
```nika
fn has_a(k: i64) -> bool {
    let xs = ["a", "b"]
    return (xs.get(k) ?? return false) == "a"
}
fn main() { println(f"{has_a(0)} {has_a(5)}") }
```
Rows:
* a: the probe - rustc *Can't compare `&str` with `str`*. Must print `true false`.
* b: `fn first_is_a(xs: Vec[String], k: i64) -> bool { return (xs.get(k) ?? return false) == "a" }` called with `["a", "b"]` - the same rustc error. Must print `true`.
* c: `let xs = [1, 2]` / `let x = xs.get(k) ?? return false` / `return x == 1` - rustc *Can't compare `&{integer}` with `{integer}`*. Must print `true false`.
* d: row c with the text: `let xs = ["a", "b"]; let x = xs.get(k) ?? return false; return x == "a"` (the architect saw it fail at c30b7bc0; re-probe). Must print `true false`.
* e (prints `true false`, must stay): row c with `let xs: Vec[i64] = [1, 2]`.

An unfixed binary for before/after probes: `/home/user/nikaia-suite/target/fast/nikaia`.

## Suspected location (traced to the fact, not to the exact test that misses)
The copy is the `copied_jump_reads` fact, read by the emitter at `crates/nikaia-std/src/tools/emit_core_flow.nika:774` (writes `nikaia_std::num::value(__nikaia_value)` in the `Some` arm). It is set in two places:
* `check_core_ops.nika:791` (a `??` anywhere): `check_core_ops_a_word(inner)`;
* `check_core_stmt.nika:1061-1064` (under a `let`): `a_copy_by_value(bound)`, then `check_core_stmt_copied_jump_reads` (:128).
Both cover a declared number element (row e), not an untyped number literal list (row c; its element type is probably still the literal's open type when the test runs) nor a list of text views (rows a, b, d; a `&str` element copies, so the `Some` arm can deref it, but `num::value` is for numbers: check what it accepts in `crates/nikaia-std/src/num*` and use a deref rustc takes for `&&str`, e.g. `*__nikaia_value` or `.copied()`, any lowering rustc accepts). Widen the two tests, and the emitter if `num::value` does not fit text, without making a non-copy element (a `Vec[String]` of owned text, a struct) copy: Nikaia never copies behind your back.

## To build
1. Before any edit, run rows a-e on the current binary; keep the results under `Tests:`.
2. The fix. Test `tests/compiler/tests/get_or_return_compared_over_literals.nika` + `.stdout` (rows a-e, one function each, printing their answers). Commit `emit: a get past a jump over a list of literals is the element, compared by value (Fixes #947)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings` (by name), with the tests whose names contain `coalesce`, `jump`, `get_or` or `copied` (run by name); `lower-std` fixpoint clean (a change in a lowered `tools/*.nika` other than the files you edited is a finding: name it under `Tests:`); build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `check_core_ops.nika`, `check_core_stmt.nika`, `emit_core_flow.nika`, `crates/nikaia-std/src/num*` (std's `value`), their lowered twins and the test; an error in another agent's file.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b14-947-get-or-return-compared.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
