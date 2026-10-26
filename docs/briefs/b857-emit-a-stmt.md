---
agent: builder
model: sonnet
max_turns: 120
budget_usd: 20
---
# Brief: E11-4b part 2, `a_stmt` of `emit` moves to Nikaia (Part of #583, after #854 part 1)

Score: S=2 (about 950 Rust lines), O=1 M (the last statement callback into Rust), C=12 M (E11-4a 1212 lines in one run; part 1 moved 290), self-hosting lane 1 (`docs/priorities.md`).

## Facts
* Part 1 (d1fe8d04) moved blocks, `stmt`, `stmt_written`, `makes_a_map` into `tools/emit_stmt.nika`; `a_stmt` is still Rust, reached from Nikaia through the host method `a_stmt` (`tools/emit_host.nika:81`, `emit/host.rs:294`). Report: `docs/briefs/b854-emit-stmt.report.md`.
* `a_stmt` has about 25 callees: `keep_plan`, `ty_counted`, `tethered_type`, `map_key`, `index_expr`, `around_value`, `handed_back`, `returns_either`, ... Those already in Nikaia are called directly; the rest stay host methods (add them to the host trait).
* rs2nika kept 0 of 116 lines in part 1 (stops at the first let-chain): write by hand from the Rust, use the draft only where it converts.
* If 950 lines do not fit the run: move `a_stmt`'s arms by statement kind (let/assign first, then return/loop/the rest), the unmoved kinds behind the host, and commit after each kind. Report what is left.

## Locations (at bf655248; re-grep)
* `crates/nikaia/src/emit/mod.rs`: `a_stmt` 2398-3353, `around_value` 3354, `handed_back` 3366, `returns_either` 3393, `map_key` 3404, `index_expr` 3462, `ty_counted` 2096, `keep_plan` 4200, `tethered_type` 4226.
* Target: `crates/nikaia-std/src/tools/emit_stmt.nika` (prefix `emit_stmt_`). Pattern: `tools/emit_expr.nika`. Known facts: `docs/builder-facts.md`; names by `nikaia run scripts/where.nika -- NAME`.

## To build
1. `emit_stmt_a_stmt` in `tools/emit_stmt.nika` with the helpers only it calls; `emit_stmt` calls it directly; the host's `a_stmt` and Rust `a_stmt` go, with the helpers nothing else calls (dead locals deleted).
2. A lowering defect met: fix with a `tests/compiler/` test, own commit, or file a `ready` issue and stop.
3. `cargo build --profile fast -p nikaia`, `target/fast/nikaia lower-std`: output unchanged beyond the slice's files; `cargo test --profile fast -p nikaia --test it -- emit:: sysroot`, `nikaia test tests/compiler --both-settings`. Update `scripts/self_hosting.py`, add *E11-4b switched* to `docs/selfhosting-plan.md`. Last commit `Fixes #854, #857`, `Part of #583`.

## Rules (CLAUDE.md)
* `develop` in the main checkout; `git add <files>` only; push fast-forward, no rebase. Commit after each step.
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite.
* Do not touch other agents' uncommitted files (`git status` at start); #856's builder may work in `tools/check_*` and the emitter's literal writers.
* `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/`; delete `target/release/nikaia-scratch`; report GB freed. Scripts in files, small reads, no foreground sleep.

## Report (at most 15 lines, `<brief path>.report.md`)
Result, Files, Commit, Fixes, Pushed, Tests, Deviation, Found, Needs, rs2nika kept N of M, Rust lines removed, GB freed, Owner (form: `docs/architect.md`, *Briefs and reports*).
