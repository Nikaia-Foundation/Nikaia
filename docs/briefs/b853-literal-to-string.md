---
agent: builder
model: sonnet
max_turns: 90
budget_usd: 12
---
# Brief: refuse `.to_string()` / `.clone()` on a text literal (#853, NK1301)

Decided: ADR-282 D30, Part I 2.3 (sentence after NK1189), Part III code table
(`NK1301`). Read those three lines, not the whole files.

## Steps
3. Checker: refuse a method call `to_string` or `clone` (no arguments) whose
   receiver is a plain string literal (not an f-string with holes, not a local).
   Message names the call; help: drop `.to_string()` / `.clone()`. Find where
   NK1189 is raised (`grep -rn NK1189 crates/nikaia/src`) and add beside it.
   Test: `tests/compiler/literal_to_string_refused.nika` + `.stderr` (both
   calls refused; `x.to_string()` on an `i64` and on a `String` local accepted).
4. Migrate the sites: `grep -rnE '"[^"]*"\.(to_string|clone)\(\)' --include=*.nika crates scripts tests packages`
   (about 51). Drop the call. `tests/compiler/tests/to_string_of_a_view_fills_a_string.nika`:
   keep its point (D8) with a local as receiver. rs2nika (`crates/rs2nika/convert`):
   write the bare literal for Rust `"x".to_string()`, `String::from("x")`,
   `"x".to_owned()`; `--lint` flags the Nikaia form.
5. `cargo build --profile fast -p nikaia`, `target/fast/nikaia lower-std`,
   commit the lowered diff; run `nikaia test` on tests/compiler and the
   touched `it::` tests only. Last commit: `Fixes #853`.

## Rules (CLAUDE.md)
* Work on `develop` in the main checkout; `git add <files>` only, never `-a`/`-A`;
  push fast-forward (`git push origin develop`), do not rebase; supervisor rebases.
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`,
  `scripts/pre-push-check.sh`.
* Do not touch version lines, `CHANGELOG.md`, badges; NK code is only `NK1301`.
* New tests in Nikaia only; no new `#[test]`. Never the full suite locally.
* Compiler defect met on the way: fix with a test, or file a `ready` issue; no workaround.
* Do not touch other agents' uncommitted files (`git status` at start; leave what you did not change).
* `NIKAIA_CACHE_LIMIT=2G`. Disk: at most two clones with `target/`; delete
  `target/release/nikaia-scratch` after tests; report GB freed.
* Scripts in files, not heredocs; small reads; no foreground sleep.

## Report (at most 15 lines)
Commits, sites migrated, tests run, GB freed, `Owner:` line if needed.
