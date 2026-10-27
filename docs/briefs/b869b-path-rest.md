---
agent: builder
model: opus
max_turns: 120
budget_usd: 30
---
# Brief: #869 steps 3, 5, 6: what is not a path, `if let` refused, the converter (ADR-382 D4, D6, D7)

Score: S 2 (finishes a decided, specified rule, 0.0.1227; b869a built D2/D3/D5 in 6f72dfa, f8c4bb6), O 1 M (the converter still writes `if let`, which D6 refuses), C ~6 M (b869a: one Opus run for the larger half). Bugs lane; `Fixes #869`.

## Issue text (#869, the steps left)
ADR-382 (`docs/specification/adr/adr-382.md`, read D4, D6, D7) decided that a `null` test narrows a path of fields from a local or a parameter. Built: proofs keyed by path, D5's ends, lowering, five tests (`tests/compiler/tests/a_path_of_fields_narrows_in_each_test_place.nika` and four beside it).

3. D4: `v[i].f`, `f().x`, `m.get(k).f` and a lock's value are not paths. A test of one proves nothing; a use that needs a `T` there is `NK1102`, with a note that the read is not a path.
5. D6: refuse `if let v = x { }`, `if x is Some(v)` and `if e is Leaf(v)` with a diagnostic that points to `match`, if any such form is still accepted. Probe each first; a form already refused needs only its test.
6. Converter (`crates/rs2nika/convert`, no version raise): `if let Some(x) = &self.f` becomes `if self.f != null`; the `let x = self.f` stays only where the body writes the field (D7). The converter must write no `if let` any more: R23 and R48 do today. Remove the field row from `docs/selfhosting-gaps.md` (:20) and set the Built cell of ADR-382 to built.
7. Tests (`tests/compiler/tests/`, `.nika` + `.stderr`): `v[i].f` does not narrow (NK1102 with the note); a refused `if let` (and `is`, if it parsed).

## Locations
* Proofs: `crates/nikaia-std/src/tools/check_narrow.nika` (path keys from b869a; `check_narrow_tested`, `check_narrow_read_as_proved`); refusal `check_refusals.nika:104-144`.
* Parser: `crates/nikaia/src/parser/mod.rs` has no `if let` rule: probe whether the condition `let`/`is` is parsed at all, and where it is refused.
* Converter: `crates/rs2nika/convert/src/main.nika`: rule list :76-77 (R23, R24), R48 `rs2nika_option_matches` :2700 (writes `if let` at :2761), R22/R23 :3041, `rs2nika_if_let` :3618; its test `crates/rs2nika/convert/tests/if_let_narrow.*` and `corpus.*`. How to build and test it: `docs/builder-facts.md`, *The converter*.

## To build (each its own commit)
1. Step 3 with its test (`Part of #869`).
2. Step 5 with its test (`Part of #869`).
3. Step 6: converter rules (R48 writes `match` or `!= null`; R23 per D7), its tests updated, gaps row, ADR Built cell (`Fixes #869`).

## Done when
All three built; `lower-std` clean; `nikaia test tests/compiler --both-settings`; `it` filters `nullable`, `narrow`, `views_kept`, `match_arms`, `parse` (a parser change runs the whole `it` binary, builder-facts) and the guard set; the converter's own tests. Changelog: one line for steps 3 and 5 (what is not a path; `if let` refused, if it was accepted).

## Stop and report if
D4/D6 leave a case open the tests need, or an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b869b-path-rest.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
