---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief: #1065, a call through a struct field holding code passes its arguments unlent

Score: compiler bug, a wrong lowering that reaches rustc (a list unborrowed, an owned text moved) where Part I and ADR-282 D2 say the compiler lends; found by b12, small and traced.

Facts: Where things live, Traps already met, Commands

## Issue #1065 (summary)
A call through a field holding code (`h.f(x)`) does not lend its arguments; the same call through a `let` or a parameter (`g(x)`, `apply(count, x)`) does. Not about pausing: `sync` or not, one view or two.

Probe (checked on 46e6179f's binary, each fails in rustc):
```
struct H { f: fn(ref Vec[i64]) -> i64 }
fn count(a: ref Vec[i64]) -> i64 { a.len() }
fn main() {
    let h = H { f: count }
    let x = [1, 2, 3]
    println(h.f(x))          // rustc: Mismatched types, expected `&[i64]`, found `Vec<{integer}>`
}
```
* Row b: `struct H { f: fn(String) -> i64 }`, `fn count(a: String) -> i64 { a.len() }`, `let n = 3`, `let s = f"ab{n}"`, `println(h.f(s))`, then `println(s)`: rustc *Borrow of moved value: `s`*.
* Row c (b12's): `struct H { f: fn(ref Vec[i64], ref Vec[i64]) -> ref Vec[i64] }`, `fn first(a: ref Vec[i64], b: ref Vec[i64]) -> ref Vec[i64] { a }`, `let y = [4]`, `println(h.f(x, y).len())`: *Arguments to this function are incorrect*.
Expected output a 3, b 3 then `ab3`, c 3.

## Suspected location (traced by reading, fairly sure)
* Checker: `check_core_a_called_field` (`crates/nikaia-std/src/tools/check_core_args.nika` :791) calls `check_core_arguments_given` (:797) and drops its result. The kept-call path `check_core_call_kept` (`check_core_call.nika` :174) passes it to `check_core_call_lent` (:159), which records `facts.lent_args`. Make it reachable (it is a private `fn` in `check_core_call.nika`: `pub`, or a twin; say which) and call it on the field path.
* Emitter: `emit_core_a_field_call` (`emit_core_reads.nika` :339, the loop at :362) writes each argument with bare `emit_expr`; it reads no `lent_args`. Lend as `emit_core_args` does (`emit_core_args.nika` :386: `lent_args` and the text-literal exception), ideally by reusing that writer rather than copying its rule; if you copy, name the rule's other homes in `Body:`.
If the trace is wrong, find where the kept call gets its `&` and give the field call the same.

## To build
1. Reproduce rows a-c on the unfixed binary; keep the result under `Tests:`.
2. The checker and emitter fix above, with their lowered twins (`lower-std`, fixpoint).
3. Test `tests/compiler/tests/a_field_call_lends.nika` + `.stdout` holding rows a-c, failing on the unfixed binary.
Commit `check, emit: a call through a field holding code lends its arguments (Fixes #1065)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings`, with the tests whose names contain `field`, `kept`, `code_param`, `function_type`, `lambda` (by name); `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line.

## Stop and report if
An edit is needed outside `check_core_args.nika`, `check_core_call.nika`, `emit_core_reads.nika`, `emit_core_args.nika`, their lowered twins and the new test; an error in another agent's file.

**Not yours, never stage**: every file `git status` shows modified or untracked at your start (`lowered/project_members.rs` is uncommitted residue), and every `docs/briefs/*` file except your report.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b16-1065-field-call-lends.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
