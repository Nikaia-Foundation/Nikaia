---
agent: builder
model: opus
max_turns: 150
budget_usd: 40
---
# Brief: a `match` on a kind in a grammar rule, and `refuse` (ADR-grammar-2 D2, D6, #1019 step 2)

Score: owner's direction (2026-10-31, "bauen ja"): #1019 step 2, after j1 (type parameters on rules). S=A, O=~3M tokens, C=~2M tokens, grammar checker/emitter.

## Problem (probe it with `target/fast/nikaia check` at your start)
```nika
grammar Lit {
    entry rule read[T] -> T = v:Value[T] eof -> { v }
    rule Value[T] -> T = match T::kind {
        Bool      => "true" -> { true } | "false" -> { false }
        Int       => d:text([0-9]+) -> { … }      // an action the existing grammar tests accept
        Maybe(A)  => "null" -> { null } | v:Value[A] -> { v }
        Text      => refuse("no strings in this test")
    }
}
```
`Lit::read[bool]("true")`, `Lit::read[i64?]("null")`; `Lit::read[String](…)` gives `NK1250` with the reason; `Lit::read[f64](…)` (no arm) gives `NK1250` naming the rule. Expected today: refused at `match` in a rule body. Confirm and say what you found. Use the kind names `T::kind` has today (Part I 4.7, ADR-330 D4); if a kind the sketch uses does not exist, take the one that does and say so in `Body:`.

## Decision (accepted records, no owner decision needed)
* [ADR-grammar-2](../../specification/adr/adr-grammar-2.md) D2 and D6, Part II 10.8 (`docs/specification/20-nikaia-advance.md`, *A rule may take types*, and the table row `refuse("why")`): each arm is a pattern with its own actions, checked under the kind its pattern names (ADR-330 D4: `List(A)` binds `A`); only the arm the type takes is emitted; the match need not name every kind; a type that reaches no arm is refused **at the call** with `NK1250`, naming the field's path and the rule; `refuse("…")` in an arm refuses with that reason; a `match` in a pattern on a value not known while building is `NK1310`. The codes are specified; add them to the code registry the checker uses if they are not there yet (no other new code).
* The build-time `match` on `T::kind` in function bodies (#859, 0.0.1317) is the model: reuse its kind matching.

## Locations
* Parser: `crates/nikaia/src/parser/mod.rs`, the grammar rule body (`rule grammar_rule`, ~line 1999): a `match EXPR { Kind(binds) => alternatives … }` as a rule body or alternative.
* Checker: `check_grammar.nika`; the kind match of #859 in `check_buildtime.nika` / `check_core_*.nika`; emitter `emit_grammar.nika`, `emit_pattern.nika`; the per-type copies j1 built.
* Lowered files and `lower-std` fixpoint.

## To build
1. `match T::kind` in a rule body with per-arm patterns, only the taken arm emitted per copy, `refuse("…")`, `NK1250` (rule and, through nested `Value[A]`, the path of type arguments that led there), `NK1310`. Tests in `tests/compiler/tests/`: `grammar_a_match_on_a_kind_chooses_the_pattern.nika` + `.stdout`, `grammar_a_type_no_arm_takes_is_refused.nika` + `.stderr` (`NK1250`), `grammar_refuse_in_an_arm_says_why.nika` + `.stderr`, `grammar_a_match_on_a_runtime_value_is_refused.nika` + `.stderr` (`NK1310`). Commit `grammar: a match on a kind chooses a rule's pattern, refuse (ADR-grammar-2 D2, D6)`, `Part of #1019`.

## Done when
The new tests pass through `nikaia test --project tests/compiler`; each failed once against the compiler without your change (say so under `Tests:`); the existing `grammar_*` tests and j1's tests still pass; `lower-std` fixpoint clean; the cheap checks of Rules. Changelog: one line.

## Stop and report if
j1 is not on develop (its report says `Result: done`); `members[S]` is needed (step 3); an edit outside the parser, the AST, `check_*.nika`, `emit_*.nika`, `grammar_*.nika`, `grammar_run.rs`, the code registry, their lowered files and the new tests; a form beyond ADR-grammar-2; an error in another agent's file.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/j2-1019-match-on-a-kind.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDvLe1uS47UcqA9y1U2
