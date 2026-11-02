---
agent: builder
model: sonnet
max_turns: 70
budget_usd: 12
effort: medium
---
# Brief: T2, the unit tests of `contracts/ty.rs`, `contracts/touch.rs` and `manifest.rs` move into Nikaia test blocks (#1147)

Score: self-hosting lane, after T1 (test blocks of a tools file lower into `#[cfg(test)]` tests). About 610 lines of `#[cfg(test)]` Rust leave the compiler: `contracts/ty.rs:146-365`, `contracts/touch.rs:234-381`, `manifest.rs:272-512`.

Shape: faithful

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **Prerequisite**: T1 (`docs/briefs/t1-tools-test-blocks.md`) is on `develop`: a `test "..." { }` block in a tools file lowers into a `#[cfg(test)]` test of its lowered `.rs`, and `cargo test -p nikaia-std <name>` runs it. Read T1's report first. If T1's report is not `Result: done`, stop: `Result: blocked`, `Needs: T1`.
* **What the tests test is Nikaia already**: `Ty`'s text and `parse` are `tools/ty.nika` (`text` :175, `parse` :280, `split_args` :345); the touch vocabulary and `might_be_same` are `tools/touch.nika`; the manifest decisions are `tools/manifest.nika` (MF). So each Rust test becomes one `test` block beside the code it tests, with the same asserts and the same title (its doc comment carried over as the block's comment). A Rust test whose subject is still Rust (`TyOps` methods in `contracts/ty.rs`, `Manifest::read` over `toml::Value`) is tested through the Nikaia function the Rust forwards to; if there is none, that test stays Rust and `Body:` names it.
* **Helpers** of a test module (`reached` in `touch.rs`, `ledgers` and friends) become private functions next to the blocks.
* Do not touch: `docs/briefs/*` but your report, the uncommitted files `git status` shows at start (others' work), the branch `wip/r3c-type-rules-chunk2`.

## To build
1. `contracts/ty.rs` and `contracts/touch.rs`: their `mod tests` into test blocks of `ty.nika` and `touch.nika`; delete the Rust tests. `lower-std` and its fixpoint (only the two lowered files and the index change), `cargo test -p nikaia-std ty touch` green, `cargo test -p nikaia --lib contracts` green. Commit `ty, touch: their unit tests move into Nikaia test blocks (#1147)`, `Part of #1147`.
2. `manifest.rs`: its `mod tests` into test blocks of `manifest.nika`; delete the Rust tests. Same checks with `manifest`. Self-hosting share (`python3 scripts/self_hosting.py | tail -n 3`) in `Body:`. Commit `manifest: its unit tests move into Nikaia test blocks (#1147)`, `Part of #1147`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/t2-tests-ty-touch-manifest.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
