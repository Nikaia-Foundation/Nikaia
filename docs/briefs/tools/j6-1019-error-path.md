---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: a failure inside `members[S]` or `list(...)` says the value's path, `at $.contacts[1].mail` (ADR-grammar-2 D3, Part II 10.8, #1019 step 3d)

Facts: Traps already met

Score: #1019 step 3d, the error form of the typed read (ADR-344 D3's `$.contacts[3].email`); S=A, O=~1.5M tokens, C=~1.5M tokens; grammar emitter and `nikaia_std::grammar`. Follows j4 (`members[S]`, 20a79c9); does not need j5 and touches none of its files (j5: checker, `comptime_*`, `build_values`, `dump`; you: `emit_grammar.nika`, `emit_pattern.nika`, `grammar.rs`). If j5 is still running beside you, `emit_*.nika` may hold its uncommitted edits: `git status` first, and stage by name.

## Problem (probe it with `target/fast/nikaia run` at your start, in your scratchpad)
```nika
struct Mail { addr: String, }
struct Contact { name: String, mail: Mail, }
struct Book { owner: String, contacts: Vec[Contact], }
grammar J {
	entry rule read[T] -> T = v:Value[T] eof -> { v }
	rule Ch -> i64 = x:any if(x != '"') -> { 1 }
	rule Key -> String = "\"" s:text(Ch*) "\"" ":" -> { s }
	rule Value[T] -> T = match T::kind {
		I64 => d:dec[i64](digit+) -> { d }
		Text => "\"" s:text(Ch*) "\"" -> { s }
		List(A) => "[" xs:list(Value[A], ",") "]" -> { xs }
		Struct(_) => v:Object[T] -> { v }
	}
	rule Object[S: Struct] -> S =
		"{" => m:members[S](Key, Value[_], ",") "}" -> { m }
}
fn main() {
	let b: Book = J::read[Book]("{\"owner\":\"a\",\"contacts\":[{\"name\":\"x\",\"mail\":{\"addr\":\"m\"}},{\"name\":\"y\",\"mail\":{\"adr\":\"m\"}}]}") catch {
		println(f"{error}")
		return
	}
	println(b.owner)
}
```
Today (architect's probe on 20a79c9): the headline `the member `adr` names no field at line 1, column 85`, the excerpt, then 21 `in` lines, 13 of them the compiler's own helpers (`in __nikaia_members_2_one`, `_all`, `_more`, `_value`, ...), and no path. For step 2, `J::read[Vec[i64]]("[1,2,x]")` (rules `I64` and `List(A)` of the probe) prints today `expected a digit; found unexpected token `x` at line 1, column 6`, the excerpt, then `in item 1`, `in Value__i64`, `in item 2`, `in Value__Vec_i64_`, `in read__Vec_i64_`: the `in item N` lines are the backend's own frames (find what they count before relying on them; they are not the list index 2). (An `NK1111` warning on the literal's `}}` comes first and its `help:` drops a brace: #1091, not yours.)

## Decision (architect, 2026-10-31, *Language decisions*: obvious; Part II 10.8 already says "the field names and list indices on its stack are the value's path")
* The rendered `ParseError` gets one line `at <path>` right after the excerpt and before the `in` lines, where `<path>` is `$`, then `.field` for each member being read (the field's name in the program; `name:` is step 3c and may later map it) and `[i]` for each list element being read, `i` counted from 0. Wanted for the probe: `at $.contacts[1].mail`.
* The path ends at the innermost member or element being read when the parse failed. A member given twice, missing, taking no null, or unknown is a failure of the struct, so its path is the struct's (`at $.contacts[1].mail` above; `{"y":2}` at the top: `at $`); a failure inside a member's value names the member (`"age":"x"` in a contact: `at $.contacts[0].age`).
* The `in` lines no longer list the compiler's own rules (any name starting `__nikaia_`); the program's rules, copies included (`in Object__Mail`), stay as they are.
* Whoever does not use it pays nothing: the path is recorded on the failure's return path only (as the rule stack is, `push_rule`), never on a parse that succeeds; a grammar with no `members[S]` and no `list(...)` renders exactly as today; with no member or element on the path there is no `at` line.
* The headline (`error.headline()`, the first line a program prints) is unchanged.

## Locations
* `crates/nikaia-std/src/tools/emit_grammar.nika` `emit_members_rules` (524): the helper rules `__nikaia_members_N`, `_all`, `_more`, `_one`, `_value(i)`; `_value`'s alternative `i` knows the field (`names[i]`). The template rule `list(item, sep)` lowers to: `emit_list_rule` (12) and where its body is written (`where.nika -- emit_list_rule`).
* `crates/nikaia-std/src/tools/emit_pattern.nika` `emit_pattern_ref` (272): `list(...)` lowered to `__nikaia_list`.
* `crates/nikaia-std/src/grammar.rs` `parse` (121), `parse_pieces` (145), `parse_bytes` (186), `parse_bytes_pieces` (199): where the backend's error is rendered (`error.render(source)`), so where the `at` line and the frame filter go. `member_error` / `member_unknown` (295, 301).
* The backend, `winnow-grammar` rev 4f8e6691 (a git dependency, read-only: `~/.cargo/git/checkouts/winnow-grammar-*/4f8e669/src/error.rs`): `ErrorCore.rule_stack: Vec<String>` (95, `pub`), `push_rule` (264, on the return path), `render` (495), the stack's lines (546-560). One way that needs no backend change: the generated helpers push a marked segment frame (`.mail`, `[1]`) on the error's way out, and `nikaia_std::grammar` turns marked frames into the `at` line and drops `__nikaia_` frames before rendering. Any way that meets the Decision and changes no file outside Locations is yours.
* Lowered files (`crates/nikaia-std/src/tools/lowered/emit_grammar.rs`, `emit_pattern.rs`) and the `lower-std` fixpoint.

## To build
1. A failure inside `members[S]` says the path of fields, and the helpers' frames are gone. Test `tests/compiler/tests/grammar_members_the_error_names_the_path.nika` + `.stdout`: the probe above printing the whole error, plus inputs for a value that fails inside a member (`"name":5` in contact 0: `at $.contacts[0].name`), a missing field one struct down (`at $.contacts[0].mail` or the struct the field belongs to), and a top-level unknown member (`at $`).
2. A failure inside `list(...)` in a typed rule says the index. Test `tests/compiler/tests/grammar_list_the_error_names_the_index.nika` + `.stdout`: `J::read[Vec[i64]]("[1,2,x]")` with a grammar like the probe's, printed whole: `at $[2]`.
3. An existing test whose expected output listed `in __nikaia_*` lines is updated (name it in `Body:`); `grammar_failing_action_in_alternative` and every other `grammar_*` test without `members`/`list` print exactly as before.
One commit for all is fine; it says `Part: #1019`.

## Done when
The new tests pass through `nikaia test` (one program alone: `docs/builder-facts.md`, *Traps already met*) with `--both-settings`, each failed once on the compiler without your change (say so under `Tests:`); all `*grammar*` tests pass; `lower-std` fixpoint clean; the cheap checks of Rules. Changelog: one line.

## Stop and report if
the path cannot be carried without a change to `winnow-grammar` (say which function and what it must carry, `Needs: architect decision`); an edit outside Locations and the new tests; an error in another agent's file. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/j6-1019-error-path.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (`green` or `red <check>`: your cheap checks, never `none`, which cuts the batch, j4 2026-10-31), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
