---
agent: builder
model: opus
max_turns: 80
budget_usd: 16
---
# Brief: #1080, a bare `String` handed to a lent `String?` parameter is emitted without `Some`

Blocks: c1a-ledger-items.md

Score: compiler bug, a lowering rustc refuses for a form `docs/builder-facts.md` lists as working (a `String?` parameter takes a bare `String`); it blocks the C1a move (`declared.nika` `declared_function(.., &target, &outer)`). Location traced by reading, not sure.

Facts: Where things live, Traps already met, Commands

## Issue #1080 (summary)
A `String?` parameter the callee only reads is lowered as `&Option<String>`. A bare `String` handed to it is emitted as `&name`, with no `Some(..)`. With the body `(target ?? "").len()` the parameter lowers as an owned `Option<String>` and the call works, so the trigger is the lent-option lowering.

Probe, row a (checked on 18:40's `target/fast/nikaia`, `nikaia run`):
```
fn show(target: String?, n: i64) -> i64 {
    let k = target?.len() ?? 0
    return k + n
}

fn main() {
    let name = ["ab"].join("")
    println(f"a{show(name, 1)}")
}
```
Lowered `fn show(target: &Option<String>, n: i64)` and `show(&name, 1)`; rustc *Mismatched types*, expected `&Option<String>`, found `&String`. Expected output `a3`.
Row b: in the same file, `for name in ["ab", "c"] { let target = name.clone(); println(f"a{show(target, 1)}") }`: same error; expected `a3`, `a2`.

## Suspected location (traced by reading, not sure)
`crates/nikaia-std/src/tools/check_arguments.nika`, the argument loop: `state.the_compiler_writes_the_reference(..)` (`check_references.nika` :124, its one caller) writes the `&` and `continue`s before any branch records `FactWrap::Constructor` in `state.facts.nullable_args` for a non-nullable value into a nullable slot; the branches at :355 (views, #598) and :405 (element reads, #655) do it for their cases only. The emitter's wrap is `emit_exprs.nika` :107 (`Some(`..`)`), read where arguments are written (`emit_core_args.nika`). A missing or extra `&` at a call is the checker's (`check_references.nika`); the fact carrying the wrap is `nullable_args`, by `id_of(value)`.

## To build
1. Reproduce rows a and b on the unfixed binary; keep the result under `Tests:`.
2. A non-nullable value handed to a lent `T?` parameter goes inside its option; any lowering rustc accepts (`&Some(name)` at its last use, `&Some(name.clone())` otherwise). Name in `Body:` every function that repeats the rule (the :355 and :405 branches at least).
3. `lower-std` and its fixpoint.
4. Test `tests/compiler/tests/lent_option_param_bare_arg.nika` + `.stdout` with rows a and b, failing on the unfixed binary.
Commit `check: a bare value handed to a lent option parameter goes inside its option (Fixes #1080)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings`, with the tests whose names contain `nullable`, `option`, `optional`, `argument`, `lent` (by name); `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`: commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `check_arguments.nika`, `check_references.nika`, `emit_core_args.nika`, `emit_exprs.nika`, their lowered twins and the new test; an error in another agent's file.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b19-1080-lent-option-bare-arg.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
