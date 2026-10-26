---
agent: builder
model: sonnet
max_turns: 100
budget_usd: 15
---
# Brief: E11-4c + E11-5, emit's host callbacks go; tables and view_slots move (#855)

Self-hosting lane, Part of #583. Score S=2, O=1 M, C=6 M (`docs/priorities.md`).
Compiler change: the supervisor raises the version; you do not.

## Facts
* After E11-4a/4b expressions and statements are Nikaia; the host's `expr` /
  `stmt` / `block_*` callbacks into Rust only bounce back into Nikaia.
* Plan: `docs/selfhosting-plan.md`, items 4c and 5 of *What is left of C-7 and E10/E11*.
* Known facts `docs/builder-facts.md`; names `nikaia run scripts/where.nika -- NAME` (re-grep, lines shift).

## Locations
* `crates/nikaia/src/emit/host.rs`, `tools/emit_host.nika`: the callbacks.
* `crates/nikaia/src/emit/mod.rs`: `only_literals`, `proven_operation`, `binary_op`,
  `template_hole_tables`, `fold_callees`, `steps`, `declared_errors`, `tethered_types`,
  `read_view_slots`, `view_slots`, `borrowing_structs`, `borrowing_structs_beside`,
  `names_outside`; what of `with_contracts` only feeds them.

## Files (only these)
`crates/nikaia/src/emit/**`, `tools/emit_*.nika` (new `tools/emit_tables.nika`),
`crates/nikaia-std/src/tools/lowered/emit_*.rs` + `lowered.rs`, `scripts/self_hosting.py`,
`docs/selfhosting-plan.md`, a `tests/compiler/` test for a lowering defect.
Do NOT touch `crates/nikaia/src/check/**`, `tools/check_*.nika`, `tools/signature*`,
`described.contracts`, `scripts/agent.nika`: the #851 builder owns them. If `check`
calls a moved function, keep a thin Rust adapter in `emit/`. When `lower-std`
rewrites a non-emit lowered file, it is #851's change: leave it unstaged.

## To build
1. 4c: Nikaia calls `emit_expr` / `emit_stmt` / `emit_block_*` directly; delete the callbacks from the host trait and `host.rs`, and the adapters nothing calls.
2. E11-5: rs2nika draft (`--profile emit --prefix emit_tables_`), then `tools/emit_tables.nika` for the template / literal-hole tables, `view_slots`, `borrowing_structs*`.
3. A lowering defect met: fix with a `tests/compiler/` test, own commit, or file a `ready` issue and stop.
4. `cargo build --profile fast -p nikaia`, `target/fast/nikaia lower-std`, `cargo test --profile fast -p nikaia --test it -- emit:: sysroot`. Add the file to `scripts/self_hosting.py`, the *switched* paragraph to `docs/selfhosting-plan.md`. Last commit `Fixes #855`, `Part of #583`.

## Rules (CLAUDE.md)
* `develop` in the main checkout; `git add <files>` only; push fast-forward, no rebase. Commit after each step.
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite.
* `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/`; delete `target/release/nikaia-scratch`; report GB freed. Scripts in files, small reads, no foreground sleep.

## Report (at most 15 lines, `<brief path>.report.md`)
Result, Files, Commit, Fixes, Pushed, Tests, Deviation, Found, Needs, rs2nika kept N of M, Rust lines removed, GB freed, Owner.
