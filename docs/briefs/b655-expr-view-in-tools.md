---
agent: builder
model: sonnet
max_turns: 100
budget_usd: 20
---
# Brief: #655 step 2b rest: an `Expr` element is viewed in `tools/*.nika`, then `check_arguments_at` goes

Score: bugs lane (`docs/priorities.md`): a false refusal (NK1257) on our own compiler types that keeps the detour `check_arguments_at`; one area (the checker's word test). No owner decision: ADR-354 D1 decides it (a read off a place under `ref T`/`ref T?` is a view where `T` is not a word), and D2 names the words by kind: numbers, `bool`, `scalar`, an enum of bare variants. `Expr` and `Ty` are none of them.

## Facts (from b655-null-arm-view's report, da5a86e)
* In a tools file, `let here: ref Expr = given[at]` (`given: ref Vec[Expr]`) is NK1257 *An Expr read out of a place is a copy*; `let h: ref Expr? = if c { given[at] } else { null }` too; as a struct field NK1106. With a user struct `E` in a program it works (test `a_null_arm_beside_an_element_view_is_an_optional_view`).
* `Expr` (`tools/lowered/ast.rs:75`) and `Ty` (`lowered/ty.rs:5`) derive `Clone`, not `Copy`; `described.contracts` marks only `Symbol`, `NodeId` as `copies = true`. So the claim *the ledger counts them as copied* is not established: find which term of the word test answers true.

## Locations
* `crates/nikaia-std/src/tools/check_boundary.nika:153` `a_word_by_its_kind` (terms: `a_number_name` `check_types.nika:11`, `a_payloadless_enum` 160, `a_ledger_copies` `lends.nika:67`); called at `check_core_stmt.nika:346` (`check_core_stmt_element_view`, NK1257 at ~564).
* The tools package's library: `crates/nikaia/src/sysroot.rs` ~573-588 (std ledger + described + `infer_package_checked`'s own).
* Detour: `crates/nikaia-std/src/tools/check_arguments.nika:23` `check_arguments_at`, one call at 219.

## To build
1. Probe: a throwaway `let here: ref Expr = given[0]` in a tools function, `nikaia lower-std` (`docs/builder-facts.md`, *Reproduce a lowering defect*); find the term; fix it at its source so `Expr`/`Ty` are not words (a type with a payload variant, or not `Copy`, is not one). If the cause is that a lowered enum is taken as payloadless or a `Clone` type as copying, fix that rule, not the tools code. A program test that reproduces it in plain Nikaia, if it reproduces outside tools (an enum with payload variants as the element); else the `lower-std` diff is the test.
2. Replace `check_arguments_at(given, at)` at its call with the `if` it stands for, delete the function and its comment, `lower-std`, commit the diff.

## Done when
Both steps built; `nikaia test tests/compiler --both-settings` and `it` filters `element_views`, `views_kept`, `nullable`, `match_arms`, `copies` pass. Commit(s) `An Expr read off a place is a view, not a copy (Part of #655)` and `check_arguments: the detour check_arguments_at goes (Part of #655)`. Changelog line: yes if step 1 changes what a program means.

## Stop and report if
The word test is right and ADR-354 contradicts the code (an open design choice): `Result: blocked`, the question under `Owner:`.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/b655-expr-view-in-tools.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word first; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed. `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
