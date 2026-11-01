---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 10
effort: medium
---
# Brief: S2, the `--bounds` report's walk into `bounds_report.nika` (Part #125)

Score: self-hosting lane, no plan row (the reasons are `bounds_reasons.nika` already, the proof `bounds_walk.nika`). About 275 lines of Rust move, by hand.

Cut: crates/nikaia/src/bounds_report.rs:31-40
Cut: crates/nikaia/src/bounds_report.rs:84-181
Cut: crates/nikaia/src/bounds_report.rs:184-354
Shape: faithful

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **What moves**, into a new `crates/nikaia-std/src/tools/bounds_report.nika` (every function `bounds_report_`-prefixed, doc comments carried over): `Site`; the body of `report_checked` from `let mut walk` (:95) to `out` (:181), the item and grammar loop, the sort, the header and the rows; `Walk` and its methods (:184-323); `symbol` (:325-331). `Within` (:333-354) goes: call `bounds_reasons.nika` `blocked_index` :15 / `blocked_operation` :37 directly. The Rust helpers are Nikaia already: `check::written` is `written.nika` `written_expression` :11, `value_node` is `ast.nika` `node_key` :843, `span.at()` is `span.start`, `visit_stmt_blocks` is `check_visit.nika` `check_visit_stmt_blocks` :227, `visit_stmt` is `check_visit_stmt` :157 then `check_visit_children` :28 in preorder with the holes after each node (`contracts/sync.rs:268-305`), `fold_bound::folds` is `fold_bound.nika` `fb_folds` :357, `BoundsChecks::name` is `emit_build.nika`.
* **Shape in Nikaia**: `pub fn bounds_report_checked(program: ref Program, words: ref winnow_grammar::InternerContext, source: String, path: String, facts: ref BoundsReportFacts, bounds: BoundsChecks, overflow: OverflowChecks, holes_of: fn(ref Expr) -> Vec[Expr] sync) -> String`. `BoundsReportFacts` is owned and built by the Rust forward (pattern `BoundsContext`, `bounds_walk.nika:19`): `lists`, `arithmetic`, `negations` from `checked.facts`, and the three `proven` sets as `BTreeSet[i64]` (Rust's are `HashSet`). `Walk` holds only what it owns (`seen`, `sites`); the walk's `args`/`body` go as parameters. `holes_of` is the `buffers.nika` :210 pattern (Rust: `&|e: &Expr| crate::emit::literal_expressions(parsed, e)`, as `contracts/keep.rs:353`).
* **Probe** (supervisor, this round, `target/fast/nikaia run`, right output): `s2_walk.nika` (a recursive walk with `mut walk` and a `ref` facts struct, an or-pattern of the operator as a guard `E::Bin { op, lhs, rhs, at } if checked(op)` since a field pattern does not bind an or-pattern (`@` is undecided, ADR-291), the holes visited after each node, the line as a count of byte 10 in `source.bytes[0..<end]`, `walk.sites.sort_by_key(fn(s) { s.line })`, `match blocked { null => ..., why => ... }`, padding with `" ".repeat(n)`). Rust pads `{:>4}`/`{:<width$}` by chars but takes `width` from `len()` in bytes: keep both. Two defects met, fixed by bug brief b35 before this brief runs (write the faithful form; if it still fails, stop): `let mut sites = walk.sites` then `sites.sort_by_key(...)` fails in `rustc` ("behind a `&` reference", #1142), and `pad(site.line.to_string(), 4)` with `pad` returning its parameter fails in `rustc` ("temporary value dropped", #1143).
* **Rust keeps** `report` (:41-69) unchanged, and `pub fn report_checked(parsed: &Parsed, source: &str, path: &str, checked: &crate::check::Checked, own: &Ledger, library: &Ledger, bounds: BoundsChecks, overflow: OverflowChecks, through_loops: bool) -> String` with its first lines (:84-94, `with_published` and `bounds::proven` stay Rust), then the facts struct and the call. Untouched callers: `report` at `project.rs:1729` (`explain_report`) and in 5 test files of `tests/it/` (`bounds_report.rs`, `field_paths.rs`, `facts_of_the_walk.rs`, `expression_functions.rs`, `fold_counters.rs`); `report_checked` at `sysroot.rs:580` (`bounds_tools`).
* **New tools file**: one `lower-std` with the old binary before the `cargo build`, and a row in `scripts/self_hosting.py` after the `bounds_*` rows (:108-131).
* **#1060 guard**: after `lower-std`, only `lowered/bounds_report.rs` and `lowered.rs` are yours. `lowered/emit_core_leaves.rs` differs in the tree from another agent's uncommitted `emit_core_leaves.nika`: neither stage nor restore it.
* Do not touch: `docs/briefs/*` but your report, the uncommitted files `git status` shows at start.

## To build
1. `bounds_report.nika`, the Rust forward, `lower-std` and its fixpoint; every other `lowered/*.rs` unchanged. Tests: `cargo test -p nikaia --lib bounds`, `cargo test -p nikaia --test it -- bounds_report:: field_paths:: facts_of_the_walk:: expression_functions:: fold_counters:: self_hosting::` (anchored `grep -E '^test result:|FAILED|panicked'`), and `target/fast/nikaia lower-std --bounds` before and after with the same output. No plan row; self-hosting share in `Body:`. Commit `bounds: the --bounds report walks in bounds_report.nika (S2)`, `Part: #125`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/s2-bounds-report.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
