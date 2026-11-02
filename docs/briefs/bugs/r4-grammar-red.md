---
agent: builder
model: opus
max_turns: 90
budget_usd: 18
effort: medium
---
# Brief: R4, red suite on develop: 17 grammar and sysroot tests (fix forward)

Blocks: every push (the driver stopped: suite red after 2 QA rounds).

## Problem
CI run 41408230519 on 07798a99 (ci/briefs), job arm64, `cargo test -p nikaia --test it`: 17 failed:
`build_time_dependencies::a_grammar_action_calls_a_dependency_at_build_time`, `…calls_another_file_of_its_package`,
`dsl_grammar::{text_the_grammar_refuses_is_refused_at_the_block, a_block_over_a_grammar_is_its_entry_s_value, the_block_has_the_entry_s_type, two_blocks_in_one_statement_are_two_values}`,
`grammar_at_build_time::*` (6), `language::the_build_time_package_passes_its_own_tests`, `project::a_grammar_at_build_time_reaches_a_project_build`,
`sysroot::{stds_nikaia_half_lowers_the_same_at_both_switches, the_committed_rust_is_what_this_compiler_lowers, the_tools_lower_the_same_on_one_thread_as_on_every_core}`.
Suspects (unpushed local commits since origin/develop): 0489f495 (M2 grammar_run.nika, GrammarDriverHost), b285603c + 0016f6ef (M1 comptime_tree, comptime_run RefCell), and a committed `lowered/*.rs` or `lowered.rs` that does not match what the compiler lowers from the committed `.nika` (sysroot tests).

## To build
1. Work ONLY in the clean detached worktree `/home/user/nikaia-red` (it is at develop HEAD; the main checkout holds other agents' half-done files and does not build). Use `CARGO_TARGET_DIR=/home/user/Nikaia/target` to share the dependency builds (disk is small). Reproduce one grammar test and one sysroot test, find the cause with `git log origin/develop..HEAD`, fix it forward as small commits there (never revert a whole move unless the fix is out of reach; then revert only the breaking commit and say so).
2. Run all 17 tests green, plus `cargo test -p nikaia --lib`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `lower-std` fixpoint if a `tools/*.nika` changed.
3. Your commits are on a detached HEAD in that worktree: name every sha in the report; the supervisor cherry-picks them onto develop. Do not touch the main checkout `/home/user/Nikaia` except writing your report there.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/bugs/r4-grammar-red.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
