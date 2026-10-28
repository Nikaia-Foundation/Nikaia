---
agent: builder
model: opus
max_turns: 110
budget_usd: 22
---
# Brief: M2, tests as functions of `modules.rs` into Nikaia (Part #125)

Score: self-hosting lane (`docs/priorities.md`, owner's direction 2026-10-27: moves first; `modules.rs` after M1, 1,239 lines). Plan row M2 (`docs/selfhosting-plan.md`, *Large files 2*). S=2 (~205 Rust lines, draft 36%), C~3-5 M (comparable: M1, 3136e7b). M1 is built (3136e7b).

## Facts
* Plan row M2 (copied): `modules.rs`: `test_function`, `is_a_test_function`, `with_tests_as_functions`, `names_in` into `crates/nikaia-std/src/tools/modules.nika` (`test_dispatcher`, `test_function` :474 are there already: the Rust `test_function` is a duplicate to delete). Host added: `parse_to_ast(text) -> Parsed` for the entry re-parsed with its dispatcher; Nikaia rewrites `Item::Test` into `Item::Fn` over `mut Vec[Spanned[Item]]` and takes `emit_walk::visited_in_*` for `names_in`.
* Locations at 67dfd5a, `crates/nikaia/src/modules.rs`: `test_function` :578, `is_a_test_function` :585, `with_tests_as_functions` :1030-1146, `names_in` :1157-1230. Re-grep before cutting.
* The plan's open prerequisite (a name `__nikaia_test_K` interned from Nikaia; today `parser::parse_expression(&interner, &test_function(k))`) is settled: a tools file interns through a callback `intern: fn(ref String) -> winnow_grammar::Symbol sync`, as `check_walked_kept_call` (`check_walked.nika` :101) and `CheckHost::intern` (`check_host.nika` :70) do. Use that callback (Rust passes `|t| interner.intern(t)` or the equivalent the interner offers), not `parse_expression`, for `main` and the test names; `parse_to_ast` stays the one Rust callback for the re-parse.
* What stays Rust: `Unit`, `Program`, `TestCase` (Rust structs: `Unit` holds `Parsed`); the loop over `units` may stay Rust if it needs `&mut Unit`; then Nikaia does the per-file work (the `Item::Test` to `Item::Fn` rewrite of one file's items, returning the titles and lines; the dispatcher text; `names_in` over one `Parsed`'s items). That cut is yours: name it in `Body:`.
* The converter's `Rs2nika:` costs are read by the converter lane: do not change `crates/rs2nika/`.

## To build
1. Run the converter on a scratch file holding exactly the moved Rust (`rs2nika --profile check --file`), report `--score` under `Rs2nika:`. Move the listed functions into `modules.nika` (prefix new public names `modules_` where `where.nika` finds a clash) with their doc comments; `modules.rs` calls the lowered functions; the moved Rust is deleted. `lower-std`: only `modules`' lowered file and its index lines change.
2. Mark M2 built in the plan's table (one line, as M1's row); report the self-hosting % under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*, `CLAUDE.md`); if it is more than one step, stop and report it under `Needs:`. Known open checker defect, not yours: #909.

## Done when
`nikaia test tests/compiler --both-settings` (every `test` block of it goes through this code); `it` tests filtered on `test_ dispatch names_in permission package` plus the guard set of `.claude/agents/builder.md`; clippy clean; lower-std fixpoint. Commit `modules: tests as functions into Nikaia (M2)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice beyond the cut left above; a `.stderr` that would change; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/m2-modules-tests.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
