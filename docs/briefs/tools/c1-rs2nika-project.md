---
agent: builder
model: opus
max_turns: 150
budget_usd: 30
---
# Brief: the converter writes `project.rs` (Part #125)

Goal: `rs2nika` (`crates/rs2nika/convert`) writes drafts of `crates/nikaia/src/project.rs` that the committed Nikaia code keeps to a high share. P5c kept 35 of 207 lines; the hand edits it named were a schematic `impl Project` rebuilt as a free function over host calls, `ref` parameters and views, and `let` bindings and control flow restructured around host results. The work is the converter; `project.rs` moving into `tools/project_*.nika` is what proves it.

## The cycle
For each part of `project.rs` that still is Rust (the `Settings` impl, `lower_reading` with its `LowerHost`, `check`, `write_proofs`/`write_ledger`, `explain`, the `Project` methods before `drive`), in the order you judge cheapest first:
1. Convert it (`--profile check-free`, a `--prefix project_<part>_`, `--facts` from rust-analyzer), finish it by hand into a `tools/project_<part>.nika` with its host trait as `project_drive.nika` and `project_check.nika` do, and score the draft against what you kept (`--score`).
2. For each hand edit that recurs or that a rule could make, change the converter: a rule in `src/*.nika`, a row in `rs2nika.names`, a case in `crates/rs2nika/convert/tests`. Re-convert and check the share rose.
3. Commit the converter change and the move as separate commits.

The share is measured on a draft written before the hand edits of that part; a part converted after a converter change shows whether the change carries. A part whose move is costly but teaches the converter nothing may stay Rust; say which in `Body:`.

## Facts
* Shape of a moved part: `project_drive.nika` (P5c) and `project_check.nika` (P5d). The forwards in `project.rs` keep the Rust signatures.
* After `lower-std`, only the lowered files of your own `tools/*.nika` and `lowered.rs` differ from `HEAD`; build `target/fast/nikaia` from `HEAD` before the first `lower-std`.
* Tests: the converter's own (`crates/rs2nika/convert`, `--corpus`), and for a move `cargo test -p nikaia --test it -- project build run_ library explain ledger` and `cargo test -p nikaia --lib project::`.

* `Rs2nika:` in the report: per part, `kept N of M lines` of the first draft and of the last, then the three costliest hand edits left.
## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/tools/c1-rs2nika-project.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score` and the three costliest hand edits, else `none`), and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
