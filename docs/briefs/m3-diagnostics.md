---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
effort: medium
---
# Brief: M3, diagnostics/mod.rs and specbook.rs into Nikaia (Part #125)

Score: self-hosting lane, goal of the day 90 %. `crates/nikaia/src/diagnostics/mod.rs` (808 lines: `translate_units` :290-376, `render` :568-600, `a_stale_boundary` :390-417, `log_internal`, `shown`, `impl LineIndex` :678-703, and the rest that is logic) into a new `crates/nikaia-std/src/tools/diagnostics.nika` (row in `scripts/self_hosting.py`; one `lower-std` with the old binary for a new file); then `specbook.rs` (306: `reach`, `report`, `blocks`, `verdicts`) into `specbook.nika`. The `mod stale_boundaries` tests (:706-808) move to `tests/compiler/` as `.nika` programs with `.stdout`/`.stderr` where the behaviour shows through `nikaia`; what cannot be said that way stays and goes into `Body:`.

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **How to scope** (no supervisor probe this round, the owner wants the moves fast): resolve every location with `target/fast/nikaia run scripts/where.nika -- NAME` and `-- --outline FILE`. Move the logic, keep in Rust only what must touch Rust-only types (the `Parsed`/`Checked` handles, `anyhow`, process spawning) as thin host methods or forwards with unchanged signatures, so no caller changes. Shape: faithful (same output, same order, same texts). Write a probe of a few lines with `target/fast/nikaia run` before each non-obvious form.
* **A missing form stops that piece, not the brief**: commit what is done, then `Found:` the gap with the probe and `Needs: supervisor decision`; never write around it. A compiler defect you can fix in one small commit with a `tests/compiler` test, fix; else stop that piece and report it.
* **Parallel agents** run in this checkout (the driver's builder, two other move builders). After `lower-std`, only the `lowered/*.rs` of your own `.nika` files and `lowered.rs` lines for your new files are yours; never stage another agent's file. If `lower-std` fails in a file that is not yours, wait a few minutes and run it again; after three tries report `Found:`.
* **Commit after each finished piece** (a piece is one Rust function or impl moved, its tests green), so partial work is never lost. Each commit message names the Rust lines it moved; `Part: #125`. Self-hosting share before and after in `Body:` (`python3 scripts/self_hosting.py | tail -n 1`, measuring only).
* Do not touch: `docs/briefs/*` but your report, the uncommitted files `git status` shows at start, `sysroot.rs`, `modules.rs`, `contracts/keeps.rs`, `contracts/sync.rs`, `text_tiers*`, `bounds_report*`, `describe*`.

## To build
1. `diagnostics/mod.rs`, piece by piece, each its own commit. Tests: `cargo test -p nikaia --lib diagnostics`, `cargo test -p nikaia --test it -- diagnostic self_hosting::` (anchored grep), a sample of 20 `tests/compiler` tests with `.stderr` via `target/fast/nikaia test`.
2. `specbook.rs`, same way; tests `cargo test -p nikaia --lib specbook`, `cargo test -p nikaia --test it -- specbook`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/m3-diagnostics.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
