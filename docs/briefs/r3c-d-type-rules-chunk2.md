---
agent: builder
model: sonnet
max_turns: 70
budget_usd: 12
effort: medium
---
# Brief: R3c-d, type_rules chunk 2 from `wip/r3c-type-rules-chunk2` onto `develop` (Part #125)

Score: self-hosting lane, plan row R-3 (chunk 1 is on `develop`; chunk 2 waited on two compiler defects, fixed in 4f716b62, 39375a17 and 92fec4f3). About 620 lines of Rust leave `type_rules.rs`; the Nikaia is written already.

Cut: crates/nikaia/src/type_rules.rs (what 6465e74c removes)
Shape: faithful

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **What is there**: the local branch `wip/r3c-type-rules-chunk2`, one commit 6465e74c on top of 1c3497da, touching exactly three files: `crates/nikaia-std/src/tools/type_rules.nika` (+685: `tr_expand`, `tr_check_literals_in`, `tr_drop_given` and their walkers), `crates/nikaia-std/src/tools/lowered/type_rules.rs` and `crates/nikaia/src/type_rules.rs` (the thin adapter). Its report is `docs/briefs/r3c-type-rules-direct.md.report.md`: it lowered cleanly, its lowered Rust had 12 `rustc` errors, from the two defects now fixed and the four smaller ones it lists under `Found:`.
* **How to take it**: `develop` moved since 1c3497da, and `type_rules.nika` / `type_rules.rs` may have changed there. Take the wip's two source files by hand onto the current ones (`git show 6465e74c -- <file>` and `git diff 1c3497da develop -- <file>` side by side), never a checkout of the branch in the shared tree, never a rebase. The lowered `type_rules.rs` is not taken: `lower-std` writes it.
* **The smaller defects** the wip met (lambda-built fn field of a variant: fixed in 551938dc; `Vec[String].contains` in a lambda: d2da2e7b). If one of the others still breaks the build (a guarded taking arm, a drain element handed to `mut`, a narrowed in-place `Block?` arm, `Wrap { inner: "a" }` inferred as `ref`): a fix in the compiler that fits this round is yours, as its own commit with a test; otherwise stop at it.
* **Stays Rust for good**: `across_files` and `read_again` (the Rust parser's `read_expression`, the `&mut [(&mut Parsed, &str)]` slice).
* **#1060 guard**: after `lower-std`, only `lowered/type_rules.rs` and `lowered.rs` are yours. `lowered/emit_core_leaves.rs`, `lowered/emit_core_match.rs` and `keeps.nika` differ in the tree from other agents' uncommitted work: neither stage nor restore them.
* Do not touch: `docs/briefs/*` but your report, the uncommitted files `git status` shows at start, the branch `wip/r3c-type-rules-chunk2` (read it, never move or delete it).

## To build
1. The two files onto `develop`, `lower-std` and its fixpoint, `cargo build`; every other `lowered/*.rs` unchanged. Tests: `cargo test -p nikaia --lib type_rules`, `cargo test -p nikaia --test it -- type_rules:: given:: literal_rules:: self_hosting::` (anchored `grep -E '^test result:|FAILED|panicked'`; take the module names from `grep -ln type_rule crates/nikaia/tests/it/*.rs` if these differ), and the `tests/compiler` programs whose name holds `rule` in a scratch project (builder-facts, *One program alone*). Self-hosting share in `Body:`. Commit `type_rules: chunk 2 (expand, literals, given) moves into type_rules.nika (R-3)`, `Part: #125`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/r3c-d-type-rules-chunk2.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
