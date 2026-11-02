---
agent: builder
model: opus
max_turns: 80
budget_usd: 15
effort: medium
---
# Brief: M18b, the rest of describe.rs into Nikaia (Part #125)

M18 moved adopt's checks, the descriptions digest and the names walk into `tools/describe.nika` and stopped at two lowering defects. Both are fixed now: #1189 (5c6d0ec9, `get`/`first`/`last` of a list of text views copies the `&str` out) and #1190 (92f6761f, `filter` after `map` with `contains_key`). Move the two pieces they blocked, then the left glue that has Nikaia types. The shared binary `target/fast/nikaia` may come from a private copy: set `NIKAIA_SYSROOT=/home/user/Nikaia/crates` on every call of it. Build your changes on a private `git archive HEAD` copy (`/tmp/claude-0/-home-user-Nikaia/b7e2d91b-1a15-5371-a36e-8c201aeab2d2/scratchpad/b1166/flip3.sh` shows how) with the shared target dir and delete the copy after.

## Facts
* M18's report: `docs/briefs/m18-describe.md.report.md`. Resolve locations with `target/fast/nikaia run scripts/where.nika -- NAME` and `-- --outline crates/nikaia/src/describe.rs`. Keep in Rust only what touches Rust-only types (the parser handle, `anyhow`, file walking, process spawning) as forwards with unchanged signatures, so no caller changes. Shape: faithful (same output, same order, same texts).
* **A missing form stops that piece, not the brief**: commit what is done, then `Found:` the gap with a probe and `Needs: supervisor decision`; never write around it. A compiler defect you can fix in one small commit with a `tests/compiler` test, fix (at most two); else stop that piece and report it.
* **Parallel agents** run in this checkout (a parser-helpers builder owns `parser.rs`, `tools/tree_build.nika`, `tools/grammar_types.nika`). After `lower-std`, only the `lowered/*.rs` of your own `.nika` files and `lowered.rs` lines for new files are yours; never stage another agent's file. If `lower-std` fails in a file that is not yours, wait a few minutes and run it again; after three tries report `Found:`.
* **Commit after each finished piece**, the message names the Rust lines it moved; `Part: #125`. Self-hosting share before and after in `Body:` (`python3 scripts/self_hosting.py | tail -n 1`, measuring only).
* Do not touch: `docs/briefs/*` but your report, the uncommitted files `git status` shows at start, `sysroot.rs`, `modules.rs`, `contracts/keeps.rs`, `contracts/sync.rs`, `keeps.nika`, `parser.rs`. Disk is at 3.9G free: no second `target/`.
* **Never end your turn while a build or test runs**: run in the foreground (timeout up to 600000 ms) and wait.

## To build
1. Adopt's header parse (M18's defect A) into `describe.nika`, Rust a forward. Tests: `cargo test --profile fast -p nikaia --test it -- describ shipped digest adopt`.
2. The unanswered-names filter (M18's defect B) into `describe.nika`, then piece by piece the left glue M18 named (`Manifest`/`rust_dependency`, `crate_sources`, `crate_root`, `without_dots`, `cargo_version`, the writes) where its types exist in Nikaia, each its own commit, until only Rust-only glue is left (name it in the report).

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/m18b-describe2.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
