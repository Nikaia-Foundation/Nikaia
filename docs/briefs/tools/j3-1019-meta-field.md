---
agent: builder
model: opus
max_turns: 110
budget_usd: 30
---
# Brief: `meta::Field`, a function over a reflected field (Part II 10.3, ADR-grammar-2 D4, #1019 step 3 prerequisite)

Facts: Traps already met

Score: #1019 step 3 (D4's `name:` function) cannot be written without it; S=A, O=~1.5M tokens, C=~1M tokens, checker only. The decision is the architect's (2026-10-31, *Language decisions*: obvious), written into Part II 10.3 and ADR-grammar-2 D4.

## Problem (probe it with `target/fast/nikaia run` at your start, in your scratchpad)
```nika
@meta::Attribute(field)
pub struct Name { value: String }

struct Contact {
	@Name("full_name")
	name: String,
	age: i64 = 7,
	email: String?,
}

fn wire_name(f: meta::Field) -> String {
	return f.attribute(Name)?.value ?? f.name
}

fn show[T: Struct]() {
	for f in T::fields {
		let w = wire_name(f)
		println(f"{f.name} -> {w}")
	}
}

fn main() {
	show[Contact]()
}
```
Today (architect's probe, 2026-10-31): `NK1102` "`wire_name` expects `f` to be `meta::Field`, but you're passing `$Field`", then `NK2105` "`f` was passed to `wire_name`, which keeps it". Wanted: `name -> full_name`, `age -> age`, `email -> email`. The same body written inline in the loop (`let w = f.attribute(Name)?.value ?? f.name`) already prints that.

## Decision (Part II 10.3, *A reflected field is a `meta::Field`*; ADR-grammar-2 D4 amendment)
* `meta::Field` is the name of the reflected field's type (today's internal `$Field`).
* A parameter of type `meta::Field` is `comptime` by its type: each call is made with the field of an unrolled turn and calls the copy made for that field (ADR-specialization-1 D1), in which `f.name`, `f.ty`, `f.attribute(X)`, `f.default`, ... are what they are in the loop's body. Writing `comptime f: meta::Field` means the same.
* A field is handed over, never moved: the turn's `f` stays readable after the call (no `NK2105`).
* `meta::Field` anywhere but as a parameter's type (a `let`'s type, a struct field, a result, an element) is `NK1320`: "a reflected field exists only while the program is built". `NK1320` is this lane's block and already in Part III; codes are written where they are refused (no registry).

## Locations
* `$Field` today: `crates/nikaia-std/src/tools/check_core_pilot.nika` `check_core_type_argument` (233-273), `check_reflect_field_type` (500-525), `check_reflect_field_kind` (528-547); `check_core_expr.nika` `check_core_expr_shape_element` (449-460); `check_core_method.nika` `check_core_method_walked` (825-1043, the member reads of a reflected field).
* The copy per type of a `[T: Struct]` function and the unrolling of `T::fields`: `check_core_pilot.nika` `check_specialised` (line 16, 5 callers), `check_program.nika` `check_program_unroll` (217), `check_walked.nika` `check_walked_unrolling` (156), the emitter's `emit_misc.nika` `unrolls_here` (346). A call to a `meta::Field` function inside an unrolled turn takes the same path a call to a generic function with the turn's type does: one copy per (function, struct, field).
* Moves: the `NK2105` comes from the move analysis; a `meta::Field` value is copied, never moved (find its type test with `scripts/where.nika`).
* Lowered files and the `lower-std` fixpoint for every `tools/*.nika` you change.

## To build
1. `meta::Field` as a parameter's type, one copy per field, no move; `NK1320` elsewhere. Tests in `tests/compiler/tests/`: `a_function_takes_a_reflected_field.nika` + `.stdout` (the probe above, plus a second function that reads `f.ty::kind` and is called from the same loop, so two copies differ by type), `a_reflected_field_kept_past_the_build_is_refused.nika` + `.stderr` (`struct Holder { f: meta::Field }` gives `NK1320`; one program, one finding). Commit `check: a function takes a reflected field, meta::Field (Part II 10.3)`, `Part: #1019`.

## Done when
Both tests pass through `nikaia test` (one program alone: `docs/builder-facts.md`, *Traps already met*), each failed once against the compiler without your change (say so under `Tests:`); `reflected_field_ty`, `match_on_a_kind`, `type_as_argument` and the `grammar_*` tests still pass; `lower-std` fixpoint clean; the cheap checks of Rules. Changelog: one line.

## Stop and report if
the copy per field needs the build-time runner (`comptime_run.rs`) rather than the checker's copies; an edit outside `check_*.nika`, `emit_*.nika`, `ty.nika`, their lowered files, the move analysis and the new tests; a form beyond Part II 10.3's paragraph; an error in another agent's file. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/j3-1019-meta-field.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
