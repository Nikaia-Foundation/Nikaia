---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
effort: medium
---
# Brief: M7, emit with_contracts and parse_unrefined into Nikaia (Part #125)

Score: self-hosting lane, goal of the day 90 %. `crates/nikaia/src/emit/mod.rs` `with_contracts` (:710-1051) into the `emit_*.nika` file that holds its neighbours (`emit_entry.nika` or a new `emit_contracts.nika`), then `crates/nikaia/src/parser/mod.rs` `parse_unrefined` (:787-886) into the parser `.nika` files. Callers in `modules.rs` and `sysroot.rs` keep their signatures (those files are not yours).

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **How to scope** (no supervisor probe this round, the owner wants the moves fast): resolve every location with `target/fast/nikaia run scripts/where.nika -- NAME` and `-- --outline FILE`. Move the logic, keep in Rust only what must touch Rust-only types (the `Parsed`/`Checked` handles, `anyhow`, process spawning) as thin host methods or forwards with unchanged signatures, so no caller changes. Shape: faithful (same output, same order, same texts). Write a probe of a few lines with `target/fast/nikaia run` before each non-obvious form.
* **A missing form stops that piece, not the brief**: commit what is done, then `Found:` the gap with the probe and `Needs: supervisor decision`; never write around it. A compiler defect you can fix in one small commit with a `tests/compiler` test, fix; else stop that piece and report it.
* **Parallel agents** run in this checkout (the driver's builder, two other move builders). After `lower-std`, only the `lowered/*.rs` of your own `.nika` files and `lowered.rs` lines for your new files are yours; never stage another agent's file. If `lower-std` fails in a file that is not yours, wait a few minutes and run it again; after three tries report `Found:`.
* **Commit after each finished piece** (a piece is one Rust function or impl moved, its tests green), so partial work is never lost. Each commit message names the Rust lines it moved; `Part: #125`. Self-hosting share before and after in `Body:` (`python3 scripts/self_hosting.py | tail -n 1`, measuring only).
* Do not touch: `docs/briefs/*` but your report, the uncommitted files `git status` shows at start, `sysroot.rs`, `modules.rs`, `contracts/keeps.rs`, `contracts/sync.rs`, `text_tiers*`, `bounds_report*`, `describe*`.

## To build
1. Piece by piece, largest first, each its own commit. Tests: `cargo test -p nikaia --lib emit`, `cargo test -p nikaia --lib parser`, `cargo test -p nikaia --test it -- self_hosting::` and 40 `tests/compiler` tests with `.stderr` via `target/fast/nikaia test` (pick across NK codes).

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/m7-emit-contracts.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
