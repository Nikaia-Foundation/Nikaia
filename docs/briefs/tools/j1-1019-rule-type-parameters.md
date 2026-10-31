---
agent: builder
model: opus
max_turns: 150
budget_usd: 40
---
# Brief: a grammar rule and entry that take type parameters (`entry rule read[T] -> T`, ADR-grammar-2 D1, #1019 step 1)

Score: owner's direction (2026-10-31, "bauen ja"): #1019 (`json::read[T]` through a grammar) is the next tools step; its blockers (#859 steps 1-4, D11) are built (0.0.1317-0.0.1321). S=A, O=~3M tokens, C=~2M tokens, grammar checker/emitter.

## Problem (probe it with `target/fast/nikaia check` at your start)
A program-local grammar whose rule takes a type:
```nika
grammar Num {
    entry rule read[T] -> T = v:Digits[T] eof -> { v }
    rule Digits[T] -> T = d:text([0-9]+) -> { T::parse(d) }   // or whatever action the existing grammar tests use to build a value of T
}
fn main() { let n = Num::read[i64]("42") … }
```
Expected today: a parse error or NK1230 at `[T]`. Confirm and say what you found; adapt the action to a form the checker already accepts (a `match T::kind` is step 2, not yours - pick a body where `T` only appears in the return type and in calls handing it on, e.g. `Wrap[T]` returning `T` from a sub-rule called with `[T]`).

## Decision (accepted records, no owner decision needed)
* [ADR-grammar-2](../../specification/adr/adr-grammar-2.md) D1 and Part II 10.8 (`docs/specification/20-nikaia-advance.md`, *A rule may take types*): types in brackets before the values; a call of the entry writes them (`Num::read[i64](text)`), a rule hands them on (`Value[A]`); a bound may stand on one (`[S: Struct]`); a value where a type belongs is `NK1230`. Emitted once per rule and list of type arguments, shared between calls, single-type lowered rules (`Value__i64`, the engine sees no generics); the generic rule itself is not emitted (ADR-304 D6); the copies count against the build budget (ADR-specialization-1 D5).
* Type arguments at a call by path: built in 0.0.1321 (`check_core_call.nika`, `emit_core_args.nika`) - reuse it for `Grammar::entry[T](…)`.
* No new NK code.

## Locations
* Parser: `crates/nikaia/src/parser/mod.rs`, `rule grammar_rule -> GrammarRule` (~line 1999) and the rule-call form inside patterns; AST `GrammarRule` in `crates/nikaia/src/ast/`.
* Checker: `crates/nikaia-std/src/tools/check_grammar.nika`; emitter: `emit_grammar.nika`, `emit_pattern.nika`; `grammar_decode.nika`; how `check_program` / `emit_program` reach them. `crates/nikaia/src/grammar_run.rs` only if the run path needs the copies.
* Monomorphisation of generic functions to copy from: how 0.0.1317/1319 emit one copy per type (`emit_core_args.nika`, `check_core_pilot.nika`).
* Lowered files: `crates/nikaia-std/src/tools/lowered/*.rs`, fixpoint with `target/fast/nikaia lower-std`.

## To build
1. Type parameters on a rule and an entry of a program-local grammar, with the substitution per list of type arguments and one emitted copy per list, the call `G::entry[T](input)` from code, the hand-on `Rule[T]` inside a pattern, a bound, and `NK1230` for a value in the brackets. Tests in `tests/compiler/tests/`: `grammar_a_rule_takes_a_type.nika` + `.stdout` (two types through one entry, both copies used), `grammar_a_value_where_a_rule_takes_a_type_is_refused.nika` + `.stderr` (`NK1230`). Commit `grammar: a rule and an entry take type parameters (ADR-grammar-2 D1)`, `Part of #1019`.

## Done when
The new tests pass through `nikaia test --project tests/compiler`; each failed once against the compiler without your change (say so under `Tests:`); the existing `grammar_*` tests still pass (run that subset by name); `lower-std` fixpoint clean; `cargo build --profile fast -p nikaia`; the cheap checks of Rules. Changelog: one line.

## Stop and report if
`match T::kind` in a rule body is needed (step 2, brief j2); `members[S]` (step 3); an edit outside the parser, the AST, `check_*.nika`, `emit_*.nika`, `grammar_*.nika`, `grammar_run.rs`, their lowered files and the new tests; a form beyond ADR-grammar-2; an error in another agent's file. Another session works on moving `grammar_run.rs` into Nikaia (GR): if `git log` shows a fresh commit there, rebase-free coexistence matters - say so under `Found:`.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/j1-1019-rule-type-parameters.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
