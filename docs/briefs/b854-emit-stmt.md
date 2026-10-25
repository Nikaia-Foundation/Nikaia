---
agent: builder
model: sonnet
max_turns: 120
budget_usd: 20
---
# Brief: E11-4b, statements and blocks of `emit` in Nikaia (Part of #583)

Score: S=2 (moves about 1500 Rust lines), O=1 M, C=15 M (E11-4a: 1212 lines in one run; check moves 8-15 k tokens per line), self-hosting lane 1 (`docs/priorities.md`).

## Facts
* E11-4a (ab603477) made `Emitter::expr` a thin adapter over `emit_expr` (`tools/emit_expr.nika`); statements and blocks are still Rust and reached from Nikaia through the host's `stmt` / `block_*` callbacks.
* Plan: `docs/selfhosting-plan.md`, *E11-4 split* and *E11-4a switched* (read those two paragraphs only). Pattern to copy: `tools/emit_expr.nika` + its adapter in `emit/mod.rs`.

## Locations (at a2f40f98; re-grep, lines shift)
* `crates/nikaia/src/emit/mod.rs`: `a_counted_turn` 2222, `block` 2226, `block_opening_with` 2257, `block_plain` 2314, `never_negative`, `filled_at_once`, `branch_pauses`, `hold_temporaries`, `stmt` 2567, `stmt_written` 2603, `makes_a_map` 2634, `a_stmt` 2656-3611, `around_value`, `handed_back`, `returns_either` (to 3662).
* Host: `crates/nikaia/src/emit/host.rs`, `tools/emit_host.nika`. Known facts: `docs/builder-facts.md`. Names: `nikaia run scripts/where.nika -- NAME`.

## To build
1. rs2nika draft of the range (`crates/rs2nika/convert`, `--profile emit`); report `rs2nika: kept N of M lines`.
2. `tools/emit_stmt.nika`: `emit_stmt`, `emit_block_plain`, `emit_block_opening_with` and the helpers only they call; prefix names `emit_stmt_`. A callee not yet moved goes through a host method.
3. Switch: `Emitter::stmt` / `block_plain` / `block_opening_with` become thin adapters; delete the moved Rust and the dead locals; add the file to `scripts/self_hosting.py`.
4. A lowering defect met: fix in the compiler with a `tests/compiler/` test, own commit (or file a `ready` issue and stop).
5. `cargo build --profile fast -p nikaia`, `target/fast/nikaia lower-std`: output unchanged beyond the slice's files; run `cargo test --profile fast -p nikaia --test it -- emit::` and `sysroot`. Add the *E11-4b switched* paragraph to `docs/selfhosting-plan.md`. Commit says `Part of #583`, `Fixes #<this>`.

## Rules (CLAUDE.md)
* `develop` in the main checkout; `git add <files>` only; push fast-forward, no rebase. Commit after each finished step.
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite.
* Do not touch other agents' uncommitted files (`git status` at start).
* `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/`; delete `target/release/nikaia-scratch` after tests; report GB freed. Scripts in files, small reads, no foreground sleep.

## Report (at most 15 lines, `<brief path>.report.md`)
Result, Files, Commit, Fixes, Pushed, Tests, Deviation, Found, Needs, rs2nika kept N of M, Rust lines removed, GB freed, Owner (form: `docs/architect.md`, *Briefs and reports*).
