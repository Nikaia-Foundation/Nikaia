---
agent: builder
model: sonnet
max_turns: 45
budget_usd: 8
effort: low
---
# Brief: C1c, `LedgerOps::lookup` into `ledger_items.nika` (Part #125)

Score: self-hosting lane, plan row C1 of `docs/selfhosting-plan.md` (third and last part; C1a 5bd603f3, C1b 547e6462). Small: S=1, C~0.5 M. It waited on #1107 (b26, a062b45a): a `ref T?` map view unwrapped by `??` now stays a view for a struct that does not copy, and `FnContract` does not copy (`ty.nika:1082`, `Vec` fields). Touches no file of K3b (`comptime_run.rs`, `comptime_bundle.nika`).

Cut: crates/nikaia/src/contracts/mod.rs:542-546
Convert: --profile check
Shape: faithful

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **What moves**: the body of `lookup` (`impl LedgerOps for Ledger`, `contracts/mod.rs:542`, doc comment :523-541 carried over to the Nikaia function). New in `ledger_items.nika`, beside `ledger_items_stored` (:488):
  `pub fn ledger_items_lookup(ledger: ref Ledger, name: String) -> (String, ref FnContract)?` — `let contract: ref FnContract = ledger.functions.get(name) ?? return null`, then `return (name.clone(), contract)` (`Ledger` and `FnContract` are `ty.nika` :1240 / :1082, `functions` a `BTreeMap[String, FnContract]` :1265). Write `name: String`, not `ref String` (NK1261).
* **Probe** (architect, e1a88e49): this exact shape with a non-copy struct in a `BTreeMap`, read back through `let (k, c) = lookup(l, "a") ?? return`, runs and prints the right fields; a missing key gives `null`.
* **Rust keeps** the trait method and its signature `fn lookup(&self, name: &str) -> Option<(String, &FnContract)>` as a one-line forward: `nikaia_std::tools::ledger_items::ledger_items_lookup(self, name)` (adapt only the argument form the lowering asks for: `&str` or `&String`). The 46 callers in 19 files (`scripts/where.nika -- lookup`) are not touched.
* **#1060 guard**: after `lower-std`, only `lowered/ledger_items.rs` and `lowered.rs` are yours. `lowered/emit_core_leaves.rs` differs in the tree from another agent's uncommitted `emit_core_leaves.nika`: neither stage nor restore it.
* Do not touch: `crates/rs2nika/**`, `docs/briefs/*` but your report, the uncommitted files `git status` shows at start.

## To build
1. `ledger_items_lookup` in `ledger_items.nika`, the Rust forward, `lower-std` and its fixpoint. Tests: `cargo test -p nikaia --lib contracts::`, `cargo test -p nikaia --test it -- throws ledger contract trust prelude` (anchored `grep -E '^test result:|FAILED|panicked'`). Plan row C1: C1c built, row closed (what stays Rust: the rest of `infer_package_noted`, `evaluate_defaults`, `set_default`, `computed_literal`); self-hosting share in `Body:`. Commit `contracts: lookup moves into ledger_items.nika (C1c)`, `Part: #125`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: architect decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/c1c-ledger-lookup.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score` and the three costliest hand edits, else `none`), and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
