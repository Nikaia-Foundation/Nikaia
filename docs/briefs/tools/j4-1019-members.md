---
agent: builder
model: opus
max_turns: 150
budget_usd: 40
---
# Brief: `members[S]` builds a struct by the mapping rule (ADR-grammar-2 D3, D5, #1019 step 3a)

Facts: Traps already met

Score: #1019 step 3, the element that makes `Json::read[T]` a typed read; S=A, O=~3M tokens, C=~2.5M tokens, parser, `grammar_types.nika`, grammar checker and emitter. Follows j2 (`match` on a kind, 1ee4cca) and j3 (`meta::Field`); this step does not need j3.

## Problem (probe it with `target/fast/nikaia run` at your start, in your scratchpad)
```nika
struct Point {
	x: i64,
	y: i64 = 0,
	label: String?,
}

struct Line {
	from: Point,
	to: Point,
}

grammar J {
	rule WS = "" -> { }
	entry rule read[T] -> T = v:Value[T] eof -> { v }
	rule Ch -> i64 = x:any if(x != '"') -> { 1 }
	rule Key -> String = "\"" s:text(Ch*) "\"" ":" -> { s }
	rule Null = "null" -> { }
	rule Value[T] -> T = match T::kind {
		I64 => d:dec[i64](digit+) -> { d }
		Text => "\"" s:text(Ch*) "\"" -> { s }
		Maybe(A) => v:Value[A] -> { v }
		Struct(_) => v:Object[T] -> { v }
	}
	rule Object[S: Struct] -> S =
		"{" => m:members[S](Key, Value[_], ","; null: Null) "}" -> { m }
}

fn main() {
	let l: Line = J::read[Line](Bytes("{\"to\":{\"x\":3,\"label\":\"b\"},\"from\":{\"y\":2,\"x\":1,\"label\":null}}".bytes().collect())) catch {
		println("error")
		return
	}
	let a = l.from.label ?? "-"
	let b = l.to.label ?? "-"
	println(f"{l.from.x} {l.from.y} {a} {l.to.x} {l.to.y} {b}")
}
```
Today (architect's probe, 2026-10-31): `Expected ')' here, but found ','` at `members[S](Key, …`. Wanted: `1 2 - 3 0 b` (members in any order; `to.y` missing takes its default `0`; `from.label` given `null` goes into the `String?`; `to.label` given).

## Decision (accepted records, no owner decision needed)
* [ADR-grammar-2](../../specification/adr/adr-grammar-2.md) D3, D5; Part II 10.8 (`docs/specification/20-nikaia-advance.md`, *`members[S]` builds a struct, and the compiler owns it*). `members[S](key, value, sep; name:, null:, unknown:)` reads `key value` members separated by `sep`, in any order, and yields an `S`. `value` is a rule with `_` for its type, applied with each field's type (one copy of `Value` per field type, through j1's copies). **The mapping rule**: a field the input does not give takes its default, else `null` in a `T?`, else the parse fails; a member matching `null:` goes into a `T?`, into another field it takes the default, else the parse fails; a member given twice fails the parse; a member no field takes fails the parse unless `unknown:` names a rule that skips it. Left out, `name:` is the field's own name, `null:` matches nothing, `unknown:` skips nothing (Part II 10.8, architect 2026-10-31).
* Key dispatch is the compiler's (D5): any form rustc accepts (a `match` on the key's bytes is enough here). The `unknown:` rule builds nothing.
* A parse failure is the entry's `ParseError` with a message naming the field (missing, twice) or the key (unknown); the exact text is yours, the tests pin it. The path on the stack (`$.to.label`) and strings as views are **not** this step (j-next).
* **Not this step**: `name:` (j5, needs j3), `NK1311`, `NK1312`.

## Locations
* Parser: `crates/nikaia/src/parser/mod.rs`, `rule grammar_rule` (~2247) and the pattern rules under it; j1's `expand_grammar_types` (894) and j2's `put_rule_types` (1053).
* AST: `crates/nikaia-std/src/tools/ast.nika` `enum Pattern` (1357): `Ref` holds generics and pattern args but no config, `Call` holds config but `Expr` args; a new variant (e.g. `Members { ty, key, value, sep, name, null, unknown }`) is yours to choose; `node_numbers.nika` carries every new field (j1 did the same).
* Copies: `crates/nikaia-std/src/tools/grammar_types.nika`: `grammar_types_rule` (304-328), `grammar_types_pattern` (501-560), `grammar_types_kind`/`grammar_types_parts` (269-301; where a struct's fields, types and defaults are read from the program's items). `members[S]` is expanded in the copy for a known `S`; `Value[_]` asks for one copy per field type.
* Checker: `check_grammar.nika`. Emitter: `emit_grammar.nika` `emit_grammar_body` (260-359; how `list`/`span` add template rules to the backend), `emit_grammar_rule` (361-463), `emit_pattern.nika`. Lowering: per field a slot, a fold over members (`fold(p, init, step)`, Part II 10.8, already lowered) or a loop in a template rule, then the struct built from the slots by the mapping rule. The generated Rust is under `~/.cache/nikaia/run/main-*/target/nikaia/gen/` (`ls -dt`, newest first).
* Lowered files and `lower-std` fixpoint.

## To build
1. `members[S](key, value, sep; null:, unknown:)` with the mapping rule. Tests in `tests/compiler/tests/`:
   - `grammar_members_builds_a_struct.nika` + `.stdout`: the probe above (nested struct, any order, default, `null` into `T?`), plus a missing `T?` member that becomes `null`;
   - `grammar_members_a_missing_field_fails.nika` + `.stdout` (a field without default and not `T?` absent: the `catch` prints the message), `grammar_members_a_member_twice_fails.nika` + `.stdout`, `grammar_members_null_into_a_field_that_takes_none_fails.nika` + `.stdout` (`null` into an `i64` without default; and into one with a default takes it);
   - `grammar_members_an_unknown_member.nika` + `.stdout`: fails without `unknown:`, skipped with `unknown: Skip` (a rule that reads a value and builds nothing).
   Commit `grammar: members[S] builds a struct by the mapping rule (ADR-grammar-2 D3, D5)`, `Part: #1019`.

## Done when
The new tests pass through `nikaia test` (one program alone: `docs/builder-facts.md`, *Traps already met*), each failed once against the compiler without your change (say so under `Tests:`); j1's, j2's and the other `grammar_*` tests still pass; `lower-std` fixpoint clean; the cheap checks of Rules. Changelog: one line.

## Stop and report if
j2 is not on develop; the backend cannot choose a value rule by a key read just before it (say what the generated Rust needs); building `S` needs a construction form the language does not have (D3: the compiler builds it in the lowering; no `field.set`); an edit outside the parser, the AST, `node_numbers.nika`, `check_*.nika`, `emit_*.nika`, `grammar_*.nika`, `grammar_run.rs`, `crates/nikaia-std/src/grammar.rs`, their lowered files and the new tests; a form beyond ADR-grammar-2; an error in another agent's file. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/j4-1019-members.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
