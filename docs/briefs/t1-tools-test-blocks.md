---
agent: builder
model: sonnet
max_turns: 70
budget_usd: 12
effort: medium
---
# Brief: T1, test blocks in `tools/*.nika` (#1147)

Score: self-hosting lane. Opens the move of about 900 lines of the compiler's `#[cfg(test)]` Rust (`contracts/send.rs:197-652`, `contracts/ty.rs:146-365`, `manifest.rs:272-512`) into Nikaia; the decision and the *To build* list are in #1147.

Shape: feature

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **The seam**: `lower-std` lowers the tools package file by file in `crates/nikaia/src/sysroot.rs` (`lower_tools_files_cached`, the items reach `crate::emit::emit_std_items_against_at` at about line 714). Today a `test` block in a tools file goes nowhere useful.
* **Already in Nikaia**: `modules_tests_as_functions` (`tools/modules.nika`, about line 560, ADR-269 D1) turns each `test` block into a `throws` function `test_function(k)`; the program path wraps them in `main`. The tools path needs the same turn, then `#[cfg(test)] #[test] fn <snake_case_title>() { <test_function_k>().unwrap() }` (or the emit's equivalent for a `throws` call) in the lowered file.
* **What may be Rust**: the logic is Nikaia (a new function in `modules.nika` or a tools file of its own). The Rust side is a thin adapter of a few lines in `sysroot.rs` calling it, as every move has one. If the `#[test]` attribute or the wrapper can only be written by new Rust in `emit/`, stop before the first line: `Result: blocked`, `Found:` where, `Needs: supervisor decision`.
* `cargo test -p nikaia-std` runs the crate's tests; CI runs it already.
* Do not touch: `docs/briefs/*` but your report, the uncommitted files `git status` shows at start (others' work: `keeps.nika`, `sysroot_package.nika`, `emit_core_leaves*`, `emit_core_match*`), the branch `wip/r3c-type-rules-chunk2`.

## To build
1. Test blocks lower: `lower-std` turns each `test "..." { }` of a tools file into a `#[cfg(test)]` test of its lowered `.rs`. Probe and first test: one block in `threads.nika` asserting what `contracts/send.rs`'s first test asserts. `lower-std` and its fixpoint, `cargo build`, `cargo test -p nikaia-std threads` shows the test passing, and failing when you flip its assert (then flip it back). Commit `lower-std: test blocks of a tools file lower into its #[cfg(test)] tests (#1147)`, `Part of #1147`.
2. Move `contracts/send.rs`'s `#[cfg(test)] mod tests` into test blocks beside the code they test (`threads.nika` / `check_send.nika`), one block per Rust test, same asserts; delete the Rust tests. `cargo test -p nikaia-std` and `cargo test -p nikaia --lib contracts` green; the self-hosting share (`scripts/self_hosting.py`) in `Body:`. Commit `send: its unit tests move into Nikaia test blocks (#1147)`, `Part of #1147`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/t1-tools-test-blocks.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
