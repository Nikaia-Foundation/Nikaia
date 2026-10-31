---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
---
# Brief: #1101, a tuple `match`/`if` with a `null` end and a text-literal end is refused NK1104 when the same parameter also takes an owned text

Blocks: none (found by b20, `docs/briefs/bugs/b20-1088-tuple-option-part-in-arms.md.report.md`)

Score: compiler bug, a wrong refusal of a program Part I allows (ADR-364: the wanted type reaches every end). Cause suspected, not traced to the line: medium size.

Facts: Where things live, Traps already met, Commands

## Issue #1101 (probe checked by the architect on `target/fast/nikaia` at aa1ca981)
```nika
fn other(t: String) -> (i32, String?) { return (2, t.clone()) }
fn literal(x: i32) -> (i32, String?) { return match x { 0 => (1, null), else => (2, "lit") } }
fn show(r: (i32, String?)) {
    let (a, b) = r
    println(f"{a} {b ?? "none"}")
}
fn main() {
    show(literal(1))
    show(other("t"))
}
```
Expected `2 lit` / `2 t`. Got NK1104 "You're returning `(?, ref String)`, but the function is declared to return `(i32, String?)`" at the `match`.

| `literal`'s body (with `other` and `show` as above) | result |
| :--- | :--- |
| `return match x { 0 => (1, null), else => (2, "lit") }` | NK1104 `(?, ref String)` |
| `return if x == 0 { (1, null) } else { (2, "lit") }` | NK1104 `(?, ref String)` |
| `let n: i32 = 1` then `return match x { 0 => (n, null), else => (n, "lit") }` | NK1104 `(i32, ref String)` |
| `return match x { 0 => (1, "a"), else => (2, "lit") }` | passes |
| two `return` statements, `(1, null)` and `(2, "lit")` | passes |
| row 1 without `show(other("t"))` | passes |

## Suspected location (medium confidence)
`crates/nikaia-std/src/tools/check_core_leaves.nika`, the `return` check (lines 685-729): `found` becomes a view at `if lending` (686) or through `lent_arms_handed_back` (693); then `check_core_literal_by_use` (413) reaches `check_texts.nika` `tuple_literal`, whose ends branch (404-425) gives every end the joined `found` (`plain`) rather than the end's own type. b20 (54503278) fixed the same shape in `check_overflow.nika` `check_overflow_ends_into_nullables` by reading each end's type from `state.arm_types`. What the owned caller switches on is a text-tier fact (`text_tiers.nika`): dump the facts for row 1 with and without `show(other("t"))` and compare, before editing.

## To build
1. Reproduce rows 1-3 on the unfixed binary (`Tests:`).
2. Trace to the fact that differs; fix the refusal at its cause. If the fix repeats a rule another function tests (b20 names three: `check_overflow_tuple_into_nullables`, `check_overflow_ends_into_nullables`, `tuple_literal`), keep them agreeing and name them in `Body:`.
3. `lower-std` and its fixpoint.
4. Test `tests/compiler/tests/tuple_null_end_owned_caller.nika` + `.stdout`: rows 1-3, each through one `show` parameter beside an owned result as in the probe; fails on the unfixed binary. Also run `tuple_option_part_in_arms` and the tests named `tuple|nullable|tier`.
Commit `check: a tuple with a null end and a text-literal end fits a parameter that also takes owned text (Fixes #1101)`.

## Done when
The new test fails on the unfixed compiler and passes through `nikaia test --project tests/compiler --both-settings`. Also: `lower-std` fixpoint clean; build, fmt, clippy, `pre-push-check.sh`; committed locally. Changelog: one line. Stop about 15 turns before `max_turns`: commit what is finished and report `partial`.

## Stop and report if
An edit is needed outside `crates/nikaia-std/src/tools/check_*.nika`, `text_tiers.nika`, `tiers.nika`, their lowered twins, `lowered.rs` and the new test. Or you hit an error in another agent's file.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/b24-1101-tuple-null-end-owned-caller.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (`green` or `red <check>`: your cheap checks, never `none`, which cuts the batch, j4 2026-10-31), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
