---
agent: builder
model: opus
max_turns: 150
budget_usd: 40
---
# Brief: a `comptime` binding reads a reflected field and calls a function over it; `T?` crosses (#1087, #1019 step 3c's base)

Facts: Traps already met

Score: #1019 step 3c (`name:` per field while building, `NK1311`, `NK1312`) rests on this: a `name:` function's wire names must be known while building. S=A, O=~2.5M tokens, C=~2M tokens; checker and build-time runner (`check_core_comptime.nika`, `comptime_tree.nika`, `dump.nika`, `build_values.nika`). Runs after j4 (`members[S]`), on files j4 does not touch.

## Problem (probe both with `target/fast/nikaia run` at your start, in your scratchpad)
```nika
struct Contact {
	name: String,
	age: i64 = 7,
}

fn shout(s: String) -> String {
	return f"{s}!"
}

fn wire(f: meta::Field) -> String {
	return f"{f.name}"
}

fn show[T: Struct]() {
	for f in T::fields {
		comptime a = f.name
		comptime b = shout(f.name)
		comptime c = wire(f)
		println(f"{a} {b} {c}")
	}
}

fn main() {
	show[Contact]()
}
```
Today (architect's probe, 2026-10-31): `NK1127` on all three: `a` "A `comptime` has to be computed while the program is built" (`check_core_comptime.nika:363`), `b` and `c` "This compiler couldn't build the program that computes it, which is this compiler's fault" (`COMPTIME_NOT_LOWERED`, `comptime_tree.nika` `comptime_computed`, `ComptimeRan::DidNotBuild`). Wanted: `name name! name` then `age age! age`.

```nika
fn pick(b: bool) -> String? {
	if b { return "x" }
	return null
}

comptime P: String? = pick(true)

fn main() {
	println(P ?? "-")
}
```
Today: `NK1127` "Its type `String?` can't be handed back from the program that computes it" (`dump.nika` `NO_FORM`, `comptime_tree.nika:769`). Wanted: `x`.

## Decision (no owner decision needed)
* Part II 10.3: a reflected field is a build-time value; a `meta::Field` parameter is `comptime` by its type, copied per field (ADR-grammar-2 D4 amended; j3 built the copies, 7fe9f88: `check_specialised(fn, "Struct::field")`, `CheckFacts.field_copies`). Part II 10.2: a `comptime` calls the program's own functions. So `comptime a = f.name`, `shout(f.name)` and `wire(f)` in an unrolled turn are meant to work; today they do not (kind 3).
* **`T?` crosses** where `T` does (architect, 2026-10-31, *Language decisions*: obvious; Part II 10.2 *What crosses from build time to run time*, sentence added with #1087): `null` is a `const`, any other value crosses as its `T`; a `String?` arrives as a `ref String?`. How `BuildValue` holds `null` (a new variant, or `Variant` of the option) is yours.
* A call of `wire(f)` while building runs j3's copy for the turn's field (`wire__Contact__name`), or evaluates `f`'s members in the sub-program: any way that agrees with the copy the run time calls.

## Locations
* `crates/nikaia-std/src/tools/check_core_comptime.nika` `check_core_comptime_binding` (203; the `NK1127` "has to be computed" branch at 363); how a turn's reflected field is known there (j3's `check_program_unroll` (217) and `check_program_turns` (169) in `check_program.nika`, and `EmitWalk.field_owners`, show how a turn's field is bound).
* `crates/nikaia-std/src/tools/comptime_tree.nika`: `comptime_sub_program` (38), `comptime_constants_as_lets` (57), `comptime_literal_of` (335), `comptime_held_as` (379), `comptime_computed` (695, the `NotLowered` / `DidNotBuild` arms at 767-785). The sub-program must carry the turn's field (its name, attributes, type) or the field copy.
* `crates/nikaia-std/src/tools/build_values.nika` `enum BuildValue` (17); `crates/nikaia-std/src/tools/dump.nika` `dump` (7, `NO_FORM` 110): the crossing of `T?`.
* `crates/nikaia-std/src/tools/check_core_pilot.nika` `check_specialised` (16), `check_core_field_copies` (540).
* Lowered files and the `lower-std` fixpoint. The generated sub-program's Rust is under `~/.cache/nikaia/` (`ls -dt`, newest first): read rustc's error there first, it names what is missing.

## To build
1. In an unrolled turn over `T::fields`, a `comptime` binding reads the turn's field: `f.name`, `f.attribute(X)?.value` (an attribute with a field; a fieldless attribute is #1086, not yours), and `shout(f.name)`. Test `tests/compiler/tests/a_comptime_reads_the_reflected_field.nika` + `.stdout`.
2. `comptime c = wire(f)`, a function over `meta::Field`, is computed while building. Test `tests/compiler/tests/a_comptime_calls_a_function_over_a_field.nika` + `.stdout` (the first probe above, plus a field with `@Name("full_name")` as in `a_function_takes_a_reflected_field.nika`).
3. `T?` crosses: `tests/compiler/tests/a_comptime_option_crosses.nika` + `.stdout` (the second probe with `pick(true)` and `pick(false)`, and in a turn `comptime w: String? = wire_name(f)` where `wire_name` returns `null` for one field by its name and the attribute's value or `f.name` otherwise, printed with `w ?? "-"`). An existing test whose `.stderr` pinned `String?` as uncrossable is updated (name it in `Body:`).
One commit per step or one for all; the last says `Fixes #1087`, `Part: #1019`.

## Done when
The new tests pass through `nikaia test` (one program alone: `docs/builder-facts.md`, *Traps already met*), each failed once on the compiler without your change (say so under `Tests:`); `a_function_takes_a_reflected_field`, `reflected_field_ty`, every `comptime_*` test and the `grammar_*` tests still pass; `lower-std` fixpoint clean; the cheap checks of Rules. Changelog: one line.

## Stop and report if
the turn's field cannot reach the build-time run without a change to the run's host interface in Rust (`crates/nikaia/src/`; say which function and what it must carry, `Needs: architect decision`); an edit outside `check_*.nika`, `comptime_*.nika`, `build_values.nika`, `dump.nika`, `emit_*.nika`, `crates/nikaia/src/comptime_run.rs`, their lowered files and the new tests; a form beyond Part II 10.2/10.3; an error in another agent's file. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/j5-1019-comptime-field.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (`green` or `red <check>`: your cheap checks, never `none`, which cuts the batch, j4 2026-10-31), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
