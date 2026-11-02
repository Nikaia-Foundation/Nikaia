---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
effort: medium
---
# Brief: M11, comptime_run.rs into Nikaia (Part #125)

Score: self-hosting lane, goal of the day 90 %. `crates/nikaia/src/comptime_run.rs` (1018 lines): the logic of `impl ComptimeComputeHost for ComputeHost` (:334-523, with `driven` :437-470), `compute` (:282-311), `artifacts_in` (:852-883), `waited` (:921-959) and what of `LowerHost` (:150-202) and `BundleHost` (:757-848) is not a thin process or file call, into the `tools/comptime_*.nika` file that already holds the neighbour (`where.nika` finds it). Rust keeps only the hosts that spawn processes, read or write files, and the types, with unchanged signatures.

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **How to scope** (no supervisor probe this round, the owner wants the moves fast): resolve every location with `target/fast/nikaia run scripts/where.nika -- NAME` and `-- --outline FILE`. Move the logic, keep in Rust only what must touch Rust-only types (the `Parsed`/`Checked` handles, `anyhow`, process spawning) as thin host methods or forwards with unchanged signatures, so no caller changes. Shape: faithful (same output, same order, same texts). Write a probe of a few lines with `target/fast/nikaia run` before each non-obvious form.
* **A missing form stops that piece, not the brief**: commit what is done, then `Found:` the gap with the probe and `Needs: supervisor decision`; never write around it. A compiler defect you can fix in one small commit with a `tests/compiler` test, fix; else stop that piece and report it.
* **Parallel agents** run in this checkout (the driver's builder, two other move builders). After `lower-std`, only the `lowered/*.rs` of your own `.nika` files and `lowered.rs` lines for your new files are yours; never stage another agent's file. If `lower-std` fails in a file that is not yours, wait a few minutes and run it again; after three tries report `Found:`.
* **Commit after each finished piece** (a piece is one Rust function or impl moved, its tests green), so partial work is never lost. Each commit message names the Rust lines it moved; `Part: #125`. Self-hosting share before and after in `Body:` (`python3 scripts/self_hosting.py | tail -n 1`, measuring only).
* Do not touch: `docs/briefs/*` but your report, the uncommitted files `git status` shows at start, `sysroot.rs`, `modules.rs`, `contracts/keeps.rs`, `contracts/sync.rs`, `text_tiers*`, `bounds_report*`, `describe*`, `emit/*`, `parser/*`, `check/*`, `check_*.nika`, `project.rs`, `project_*.nika`, `main.rs`, `cli_commands.nika`, `ledger_items.nika`, `grammar_run.nika` and its lowered Rust (other lanes; the bug lane R4 is fixing a red grammar build in a separate worktree). The main checkout may not build while another agent is mid-edit (e.g. `ledger_items`): wait and retry, never fix their file.

## To build
1. Piece by piece, largest first, each its own commit. Tests: `cargo test -p nikaia --test it -- comptime` and the test modules that cover what you moved (`build_time`, `dsl_grammar`, `grammar_at_build_time`; find them with grep), `cargo test -p nikaia --lib comptime`, `cargo test -p nikaia --test it -- self_hosting::` and 40 `tests/compiler` tests with `.stderr` via `target/fast/nikaia test` (pick across NK codes).

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/m11-comptime.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
