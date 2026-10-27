---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: a struct whose every field has a default lowers with `impl Default` (E12-4b prerequisite, #125)

Score: self-hosting lane (`docs/priorities.md`; #125, `docs/selfhosting-plan.md` E12-4b): `emit/mod.rs` `Build` has 343 `Build::default()` callers on the Rust side; it can move to `emit_build.nika` only when the lowering writes the `Default` the Rust callers use. One compiler step, no move.

## Facts
* ADR-331 D5 (`docs/specification/adr/adr-331.md:114`): a field of any struct may have a default, a build-time value; a literal may leave such a field out (`NK1234` for one without a default).
* Probe (architect, 2026-10-27, on 14869e3): `enum M { Off, On }` + `struct B { m: M = M::Off, n: i64 = 3 }`, `nikaia lower`: `B` derives `Debug, Clone, Copy, PartialEq, Eq`, no `Default` and no `impl Default`. `B {}` runs (`3`).
* Plan text (`selfhosting-plan.md`, E12-4 notes): "first the lowering writes `impl Default` for a struct whose every field has a default (ADR-331 D5; 343 `Build::default()` callers)".

## Locations
* Struct emit: `crates/nikaia-std/src/tools/emit_item.nika:216` (`Item::Struct`, derives at :224, fields :240).
* Field default in the AST: `crates/nikaia-std/src/tools/ast.nika:1221` (`pub default: FieldDefault?`); how a literal fills a left-out field today is the model for the values (find it from `NK1234` with `nikaia run scripts/where.nika -- NK1234`).
* Reflected `field.default`: `emit_core_places.nika:76` `emit_place_default` (not to change).

## To build
1. A struct (not `extern`, no generics, not seeded with hidden type parameters) whose every field has a default gets `impl Default for S { fn default() -> Self { S { f: <default>, ... } } }` after its declaration, each default lowered as the literal fill-in lowers it. A struct with one field without a default gets none (unchanged output). Test `tests/compiler/tests/a_struct_of_defaults_has_default.nika` + `.stdout`: the probe's `B` plus a text field with a default and a nested struct field, checks that the lowered program still runs (`B {}`) and, if the language can call it, that it equals the literal of all defaults; if Nikaia cannot call `Default::default` (no such name in Part I), the test is `nikaia lower` output checked by a `.stdout` of the run only, and say under `Tests:` how the impl was seen.
2. `lower-std`: commit any tools file whose lowering gains an `impl Default` (a tools struct of all-default fields), and check no Rust host code now has two `Default` impls for one type (rustc E0119 in the build); a host `impl Default` made redundant is deleted in the same commit.

## Done when
The test passes; `nikaia test tests/compiler --both-settings`; `it` filtered on `struct default typecheck`; `cargo build` of the workspace (step 2); the guard set of `.claude/agents/builder.md`; clippy clean. Commits `Part: #125`; `Changelog:` one sentence.

## Stop and report if
An open design choice (a generic struct, a default that is not a build-time value); an error in another agent's file.

Read `docs/builder-facts.md`.

* Not yours (another lane's uncommitted files in the checkout, 2026-10-27): `crates/rs2nika/convert/**`. Never stage them.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/e12-4b0-struct-default-impl.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
