---
agent: builder
model: opus
max_turns: 80
budget_usd: 20
---
# Brief: #1041, a function's parameter handed to a grammar entry's value parameter stays lent (rustc)

Score: a defect that blocks `yaml` (#928) step 2b's `parse(input, limits)` (`y2b-keys-directives-limits.md` runs after this brief); found by the architect probing y2b. It changes `crates/`: the gate of `.claude/agents/builder.md` applies in full, plus `target/fast/nikaia test --both-settings --project tests/compiler` (a change to `tools/check_*.nika` or `tools/emit_*.nika`), and the package gate below.

Read `docs/builder-facts.md`. Run the probe on the unfixed binary first and read its lowered file (`~/.cache/nikaia/run/<name>-*/target/nikaia/gen/`), then compare with the `keep(lim)` variant's lowering: the difference is where the checker records an argument taken by an ordinary call's owned parameter. Decide in the checker (CLAUDE.md, `docs/architect.md`: a missing or extra `&` is the checker's, not the emitter's); the emitter only writes what the checker decided.

## Issue #1041 (copied)

## Found while

The architect probing `yaml`'s next brief (y2b step 2, `parse(input, limits)` handing a `Limits` to the entry rule, ADR-yaml-1 D8; develop 6064835c, 2026-10-31). Blocks y2b step 2.

An entry rule takes values after its input (ADR-grammar-1 D2, Part II 10; `tests/compiler/tests/grammar_an_entry_takes_values_after_its_input.nika` passes only literals and locals). A **parameter** of the calling function handed there is lowered as a lent `&Limits`, and the entry's `parse_doc(lim: Limits)` wants it owned:

```nika
pub struct Limits {
	pub depth: i64,
}

grammar G {
	rule WS = "" -> { }
	entry rule doc(lim: Limits) -> i64 = "x" if(lim.depth > 0) eof -> { lim.depth }
}

fn parse(input: String, lim: Limits) -> i64 {
	return G::doc(input, lim) catch {
		return -1
	}
}

fn main() {
	println(f"{parse("x", Limits { depth: 3 })}")
	println(f"{parse("x", Limits { depth: 0 })}")
}
```

Today: rustc `Mismatched types` at `G::doc(input, lim)` ("Consider dereferencing the borrow"); the lowered `fn parse(input: &str, lim: &Limits)` calls `G::parse_doc(lim)`. Wanted: `3` then `-1`. The same with a local (`let lim = Limits { depth: n }` in `parse`) prints `3` / `-1` today, and an ordinary function `fn keep(l: Limits) -> Vec[Limits] { return [l] }` called as `keep(lim)` makes `parse`'s `lim` owned (`fn parse(input: &str, lim: Limits)`): only the grammar entry's value parameters are not seen as a position that takes its argument. Over `ref Bytes` input the same (`parse_bytes(&*input, GOverBytes::parse_doc(lim))`).

A defect: the entry is "an ordinary call" (ADR-296 D24, the comment in `check_core_method.nika:852`), so its value parameters take their arguments as an ordinary call's do.

## To build

1. A grammar entry's value parameter of a type that moves takes its argument as an ordinary call's owned parameter does: the caller's parameter becomes owned (as with `keep`), or, where the caller cannot give it up, the checker's existing rules for an ordinary call apply (copy for a `Copy` type, a refusal with the code an ordinary call gets). No new NK code. An `i64` or other copy value is unchanged.
2. Test `tests/compiler/tests/grammar_an_entry_takes_a_parameter_of_its_caller.nika` + `.stdout`: the probe above, plus the same with `ref Bytes` input (`Bytes("x".bytes().collect())`); it fails on the unfixed binary.

## Locations

Checker: `check_core_grammar_call` (`crates/nikaia-std/src/tools/check_core_pilot.nika:33`), reached from `check_core_method.nika:854` (`grammar_entry`, `check_decls.nika:261`); how an ordinary call marks an argument taken: `an_owned_parameter` (`check_fields.nika:260`), `a_lent_parameter` (`check_fields.nika:223`), `the_compiler_writes_the_reference` (`check_references.nika:124`). Emitter: `emit_core_grammar_entry` (`emit_core_leaves.nika:467`), called from `emit_core_calls.nika:622`. Lowered twins in `crates/nikaia-std/src/tools/lowered/`.

## Done when

The probe prints `3` / `-1` over `String` and over `ref Bytes`; the new test passes and fails on the unfixed binary (say so under `Tests:`); `grammar_an_entry_takes_values_after_its_input` and every `grammar_` test green in `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --both-settings --project tests/compiler > out.txt 2>&1`; the package gate below green; committed and pushed with `Fixes #1041`. Files you may change: the files under *Locations* with their lowered twins (regenerate them as `docs/builder-facts.md` says), the predicate they call where the decision lives (name it in `Body:`), the new test. Name under `Changelog:` what the change decided.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).
* **Package gate** (this lane): `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --project packages/yaml` green, and `cargo test --profile fast -p nikaia --test it -- fmt::every_nika_file` (it formats every `.nika` under `packages/`). For a change only under `packages/yaml/` this is the gate the line above asks for; a change under `crates/` runs the gate of `.claude/agents/builder.md` (#852) as well. `target/fast/nikaia` is built by the session hook; if it is missing, `cargo build --profile fast -p nikaia` once.
* A package change raises no toolchain version (CLAUDE.md table; ADR-yaml-1 D1): `Changelog: none`, unless the brief changes `crates/`.
* Files not to touch: anything outside `packages/yaml/` and the brief's own report, unless the brief names it; never `docs/briefs/batch.txt` or a brief of another lane.

* **Stop about 15 turns before `max_turns`**: commit and push every finished step, then report `Result: partial` with what is left under `Needs:`. Uncommitted work at the end is lost work (y4 ran out at 110).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y17-1041-entry-takes-a-parameter.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
