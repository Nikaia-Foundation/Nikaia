---
agent: builder
model: sonnet
max_turns: 100
budget_usd: 15
---
# Brief: a text literal where text of its own is needed is owned (ADR-282 D31)

Score: S=3 (a correct program is refused; blocks #853), O=0.5 M, C=6 M, bug lane 1 (`docs/priorities.md`).

## Decision (owner, 2026-10-26)
ADR-282 D31 and Part I 2.2 (the sentence after "as `[1, 2]` is a `Vec` where one is wanted"): a literal in a slot declared `String` that is changed, or handed to a `mut` or owning parameter (also through a `match` binding or a lambda's last expression beside a `String` branch), is text of its own. The compiler makes the copy; the program writes no `.clone()` and no `f"..."`. NK1105's help (`tools/check_exprs.nika` ~334) already writes `let mut {name}: String = "{literal}"`: keep it, check it stays true.

## Facts (probes, target/fast/nikaia at bf655248)
* Works today: `let mut y: String = "hi"` + `y.push_str("!")`; `let mut y = "hi"` + `add(y)` into `fn add(mut s: String)`; a lambda `fn(n) { if n.len() > 1 { n.clone() } else { "short" } }` in a `Vec[String]` map.
* Fails today (NK1102 *expects `String`, passing `ref String`*):
  `fn add(mut s: String) { s.push_str("!") println(s) }` / `fn main() { let mut s: String = "hi"  match s { t => add(t), } }`
* More fail once #853's `.clone()` removal is applied: patch `/tmp/claude-0/-home-user-Nikaia/84cbb105-595e-5391-82e1-8947de490af1/scratchpad/b853/b853-wip.patch` (270 files; drops `.clone()`/`.to_string()` on literals). With it, `it::handed_over`, `it::result_source`, `it::nullable`, `it::lent_arguments` fail: those are the probes. Apply it in a scratch clone only (`git apply` there), run those four, copy each failing source into a `.nika` test; do not commit the patch.

## Locations
* Text tiers: `crates/nikaia/src/text_tiers.rs`, `tools/check_*` (`grep -rn 'NK1102' crates/nikaia-std/src/tools`), match binding typing (`tools/check_arms.nika`, `check_binds.nika`), emitter's literal construction (`tools/emit_core_*`, `emit_expr.nika`). Names by `nikaia run scripts/where.nika -- NAME`. Known facts: `docs/builder-facts.md`.

## To build
1. Tests first, in `tests/compiler/tests/`: `a_literal_bound_by_a_match_goes_to_a_mut_parameter.nika` (the probe above) and one `.nika` + `.stdout` per failing case from the four `it::` tests; each runs and prints.
2. Checker: a literal-filled `String` binding whose use needs text of its own (changed, a `mut`/owning parameter, a `match` binding passed on, a lambda's last expression beside a `String` branch) is owned, not `ref String`; no NK1102.
3. Emitter: construct the literal as owned there (`String::from`), and nowhere else (views stay views: run `it::text_tiers it::text_views`).
4. `cargo build --profile fast -p nikaia`, `target/fast/nikaia lower-std`, commit the lowered diff; run `nikaia test` on tests/compiler, `it::handed_over it::result_source it::nullable it::lent_arguments it::text_tiers`, `sysroot`. Last commit `Fixes #856`; then #853 is unblocked (say so in the report).

## Rules (CLAUDE.md)
* `develop` in the main checkout; `git add <files>` only; push fast-forward, no rebase. Commit after each step.
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite.
* Do not touch other agents' uncommitted files (`git status` at start).
* `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` (the scratch clone counts; delete its `target/` at the end); delete `target/release/nikaia-scratch`; report GB freed. Scripts in files, small reads, no foreground sleep.

## Report (at most 15 lines, `<brief path>.report.md`)
Result, Files, Commit, Fixes, Pushed, Tests, Deviation, Found, Needs, GB freed, Owner (form: `docs/architect.md`, *Briefs and reports*).
