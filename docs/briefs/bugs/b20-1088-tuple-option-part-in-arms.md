---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief: #1088, a tuple with a `T?` part at a `match`/`if` end is not wrapped in `Some`

Blocks: p5c-project-drive.md

Score: compiler bug, a lowering rustc refuses for a form the checker accepts. It blocks the P5c move (`project_drive.nika`: `return match built.binary { null => (101, null), binary => (host.run_directly(binary), binary.clone()) }`; the draft is on `wip/p5c-project-drive` cc40334f). Location traced by reading, fairly sure.

Facts: Where things live, Traps already met, Commands

## Issue #1088 (summary)
A tuple with a `T?` part at an end of a `match` or `if` does not get `Some` on that part. This holds for a tail, a `return` and a `let` with a declared type. rustc refuses it: E0308, "Try wrapping the expression in `Some`". The same tuple written directly, `return (2, t.clone())`, works.

Probe, row a (checked by the architect on 2026-10-31 19:45's `target/fast/nikaia`, `nikaia run`):
```
fn pick(x: String?) -> (i32, String?) {
    return match x {
        null => (1, null),
        s => (2, s.clone()),
    }
}
fn main() {
    let (a, b) = pick("x")
    print(a)
    print(b ?? "none")
}
```
Expected `2x`. Each of these fails the same way (all checked): row b is `return if x { (2, t.clone()) } else { (1, null) }` with `x: bool, t: String`. Row c is `match x { 0 => (1, null), else => (2, t.clone()) }` with `x: i32`. Row d is `let r: (i32, String?) = match x { .. }` followed by `return r`. Row e, the same match as a tail without `return`, takes the same path (`check_function_wrap_tail`). These already work and must keep working: `if x { return (2, t.clone()) }` then `return (1, null)`, and a bare `String?` arm (`s => s.clone()` under `-> String?`).

## Suspected location (traced by reading, fairly sure)
`crates/nikaia-std/src/tools/check_overflow.nika`, `check_overflow_list_into_nullables` (called from `check_widened_slot`, the same file, which the tail, `return`, `let` and field paths all reach). It hands only a bare `Expr::Tuple` to `check_overflow_tuple_into_nullables`. An `if`/`match`/block falls to `else => return false`, so no part gets `FactWrap::Constructor` in `state.facts.wrapped`. Another catch: the `found` type for the whole `match` is the join of its ends, `(i32, String?)`. Checked against that, the part `s.clone()` already looks nullable (`!check_overflow_a_nullable(had)` fails). Each end has to be compared with its own type. `check_texts.nika` `tuple_literal` already walks the ends (`ends_in_tuple`, `tuple_ends`, `check_texts.nika` :513, :531) and is the model. How each end's own type is reached (a recorded type by `node_key`, or the end re-typed) is your choice: name it in `Body:`.

## To build
1. Reproduce rows a-e on the unfixed binary; keep the result under `Tests:`.
2. A value whose `tuple_ends` are all tuple literals or `null` wraps each tuple end part by part, against that end's own type. Any lowering rustc accepts. Name in `Body:` every function that repeats the rule (at least `check_overflow_tuple_into_nullables`, `tuple_literal`).
3. `lower-std` and its fixpoint.
4. Test `tests/compiler/tests/tuple_option_part_in_arms.nika` + `.stdout` with rows a-e and the two working forms, failing on the unfixed binary.
Commit `check: a tuple at a match or if end wraps its optional parts (Fixes #1088)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings`, together with the tests whose names contain `tuple`, `nullable`, `option`, `optional`, `tail`, `widen` (by name). Also: `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`: commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `check_overflow.nika`, `check_texts.nika`, `check_function.nika`, their lowered twins and the new test, or you hit an error in another agent's file.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b20-1088-tuple-option-part-in-arms.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
