---
agent: builder
model: opus
max_turns: 70
budget_usd: 18
---
# Brief: #1013 step 4, a guard reads a name bound inside `text(..)`

Score: a defect of a decided form (owner, 2026-10-30: bindings inside `text(..)` are supported), found by this lane (y12); nothing in `packages/yaml` needs it today, it closes #1013. Runs after `y2c-properties.md`.

## Issue (#1013, copied)

Decided (owner, 2026-10-30): `s:text(parts:Part*)` binds `s` to the text `text(..)` covered (a view) and `parts` as it would be bound without the `text(..)` around it, in one pass over the input. Steps 1-3 are built (bdbe8a7a: bindings inside `text(..)` read by the action; 627af41e: hygienic type parameters). Since #1033 (y12, 6e843b68) a guard reads the `text(..)` label itself.

Step 4: a name bound **inside** `text(..)` is in scope for what follows in the sequence, a guard included. Today this is refused:

```
grammar G {
	rule Ch -> i64 = x:any if(x >= '0' && x <= '9') -> { 1 }
	entry rule plain -> String = s:text(d:Ch+) if(d.len() > 1) ";" -> { s }
}

fn main() {
	let r = G::plain("12;")
	println(r)
}
```

The architect ran it on today's develop: NK1117 "`d` isn't declared anywhere" at 3:45 (the guard). The action may read `d` (step 1), so the guard must too.

## Locations

* `grammar_values` in `crates/nikaia-std/src/tools/check_grammar.nika:310`: `Pattern::Bind { name, pat }` (line 318) walks `pat` with a cloned scope and pushes only the label; the `text(..)` call is `Pattern::Call` (line 371, `grammar_call`), whose inner bindings stay inside. The fix: what a `text(..)` call's argument binds flows out to the sequence's followers, inside a `Bind` too (as a `Seq` hands its bindings on).
* Unverified beyond the checker: y12's report says the lowered Rust already binds `d` before the guard (the helper rule `__nikaia_covered` and the prelude in `crates/nikaia-std/src/tools/emit_grammar.nika` / `emit_pattern.nika`), and y11 walks a guard's condition in `bind_grammar_pattern` (`crates/nikaia-std/src/tools/text_tiers.nika`) after the labels before it. If rustc or the tier pass refuses after the checker fix, those are the files; stop and report if it needs more than them.
* Precedent tests: `tests/compiler/tests/grammar_a_guard_reads_a_binding_text_label.nika` (y12) and the step-1 test of bdbe8a7a (`git show --stat bdbe8a7a`).
* Each `.nika` under `crates/nikaia-std/src/tools/` has its lowered twin in `tools/lowered/`: `nikaia lower-std`, commit both.

Read `docs/builder-facts.md` (its first trap: the winnow-grammar rev of `target/fast/nikaia`).

## To build (one commit)

1. The checker fix above, and test `tests/compiler/tests/grammar_a_guard_reads_a_name_bound_inside_text.nika` + `.stdout`: the probe's grammar, `G::plain("12;")` prints `12`, and `G::plain("1;")` with a `catch` that prints `error` (the guard fails); it fails on the unfixed binary (NK1117, say so under `Tests:`). Part II 10's sentence on bindings inside `text(..)` (added with step 1, find it with `grep -n "text(" docs/specification/20-nikaia-advance.md`) gets "a guard after it reads them too" if it does not say so already.

## Done when

The new test passes; `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --project tests/compiler > out.txt 2>&1` green with the `grammar_` tests in it; the package gate below green; committed and pushed with `Fixes #1013`. Files you may change: the files named above with their lowered twins, the Part II sentence, the new test. Name under `Changelog:` what the change decided.

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

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y14-1013-guard-inner-label.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
