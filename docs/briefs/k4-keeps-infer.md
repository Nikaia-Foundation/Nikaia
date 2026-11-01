---
agent: builder
model: sonnet
max_turns: 60
budget_usd: 10
effort: medium
---
# Brief: K4, `keeps::infer`, `the_impls_own` and `uses_of` into `keeps.nika` (Part #125)

Score: self-hosting lane, no plan row (K1-K3 moved the walk: `keeps.nika` has `uses_in` :63 and everything under it; `lends.nika` the fixpoint `kept_by` :154 and `keeps_its` :198). About 610 lines of Rust move, by hand.

Cut: crates/nikaia/src/contracts/keeps.rs:107-542
Cut: crates/nikaia/src/contracts/keeps.rs:598-779
Shape: faithful

Facts: Where things live, Traps already met, Moving Rust into Nikaia, Commands

## Facts
* **What moves**, into `crates/nikaia-std/src/tools/keeps.nika` (pattern `lends.nika`; doc comments carried over): `by_value`, `no_uses` (:107-117); `infer` (:119-379) as `pub fn keeps_infer[H: KeepsHost](mut ledger: Ledger, units: ref Vec[ParsedView], library: ref Ledger, resolved: ref collections::BTreeMap[String, CheckMethodCalls], host: ref H) -> KeepsColumn`; `Column`, `Keeper`, `ImplRow` (:381-415) as `KeepsColumn`, `KeepsKeeper` (`unit: i64`), `KeepsImplRow` (a struct may hold `method: ref Spanned[Item]`, as `ParsedView` does); `the_impls_own` (:417-542, the `NK1264` finding is a `CheckFinding` with `CheckSeverity`); `uses_of` (:598-774) as `pub fn keeps_uses_of(...)`; `holds_either` (:776-779). Everything they call is Nikaia already: `ty.nika` `method_index` :1992, `tether.nika` `declare_unit` / `a_parse_that_views` :74 / `INPUT_POSITION` :23 (Rust `INPUT`), `views.nika` `fields_held` :1802 and `holds_view` :412, `declared.nika` `written_ty` :23, `lends.nika` `kept_by` / `keeps_its`. `parsed.text(sym)` is `view.text(sym)`, `parsed.interner` is `view.words`.
* **The host**: `pub trait KeepsHost` with `fn holes(ref self, unit: i64, expr: ref Expr) -> Vec[Expr] sync` (Rust: `crate::emit::literal_expressions(units[unit], expr)`, handed to `uses_in` as `fn(e) { host.holes(unit, e) }`) and `fn keeping_statement(ref self, unit: i64, method: ref Spanned[Item], target: String, parameter: String, own_kept: ref collections::BTreeMap[String, collections::BTreeSet[String]], own: ref Ledger, library: ref Ledger) -> Span sync` (Rust: the kept `keeping_statement`). The ledgers go as arguments, not in the host: `keeps_infer` holds `ledger` as `mut`.
* **Probe** (supervisor, this round, `target/fast/nikaia run`, both run with the right output): `k4_infer.nika` (a struct with a `ref` enum field in a `Vec`, a map keyed `(String, String)` read with `[("T", "go")] ?? return`, a nested `BTreeMap[String, BTreeMap[String, Vec[K]]]` grown by the `add_kept` pattern of `lends.nika` :182, a lambda capturing a `fn` parameter handed on as `fn(ref E) -> Vec[E] sync`) and `k4_host.nika` (a generic `[H: Host]` captured in such a lambda, a host method taking `mut Vec`). Write `"T"`, not `"T".to_string()` (NK1301); `print` writes no newline.
* **Rust keeps**: `keeping_statement` (:544-596, it clones the item and truncates its statements), taking `method: &Spanned<Item>, target: &str` in place of `row` and calling `keeps_uses_of`; the `impl KeepsHost` (holding `units` and the two method indexes); `pub use` of `KeepsColumn as Column` and `KeepsKeeper as Keeper`; and `pub fn infer(ledger: &mut Ledger, units: &[&Parsed], library: &Ledger, resolved: &BTreeMap<String, crate::check::MethodCalls>) -> Column` as a forward building the views with `crate::check::parsed_view_of`. Untouched callers: the 2 `keeps::infer(` calls in `infer_package_noted` (`contracts/mod.rs:465`, `:476`). Touched, because the lowered struct has no `Default` and `unit` is `i64`: `contracts/sync.rs:182` (`column: Default::default()`) and `modules.rs:405`, `:772`, `:784` (`as usize` / `from` without the cast).
* **#1060 guard**: after `lower-std`, only `lowered/keeps.rs` and `lowered.rs` are yours. `lowered/emit_core_leaves.rs` differs in the tree from another agent's uncommitted `emit_core_leaves.nika`: neither stage nor restore it.
* Do not touch: `docs/briefs/*` but your report, the uncommitted files `git status` shows at start.

## To build
1. The move, the host, the forwards, `lower-std` and its fixpoint; `nikaia.contracts` / `nikaia.derived` of `std` byte-identical. Tests: `cargo test -p nikaia --lib contracts::`, `cargo test -p nikaia --test it -- keeps:: lending:: handed_over:: tether_keep:: contracts:: modules::` (anchored `grep -E '^test result:|FAILED|panicked'`), and the two `NK1264`/`NK1265` programs `tests/compiler/tests/a_foreign_impl_keeping_beyond_the_column_is_refused` and `a_trait_column_change_warns` in a scratch project (builder-facts, *One program alone*). No plan row; self-hosting share in `Body:`. Commit `contracts: keeps' inference moves into keeps.nika (K4)`, `Part: #125`.

## Rules
* Read `docs/builder-facts.md` first. `develop` in the main checkout; other agents' uncommitted files are not yours: never touch or stage them.
* Commit locally after each step with `target/fast/nikaia run scripts/commit.nika -- REPORT` (it adds exactly the report's `Files:`); the driver pushes.
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches.
* Version lines, `CHANGELOG.md`, badges and NK codes are the supervisor's. New tests in Nikaia only.
* A form Nikaia, `std` or the toolchain lacks stops the work, unless an ADR or Part I-III refuses it (name it): `Result: blocked`, `Found:` the gap with a probe, `Needs: supervisor decision`.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at the end delete your own clone's `target/` and `target/release/nikaia-scratch`.
* No cause after about 40 turns: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/k4-keeps-infer.md.report.md`)
Fields of `docs/supervisor.md`, *Briefs and reports*: Result, Files, Commit, Body (optional), Fixes, Pushed, Tests, Gate, Deviation, Found, Needs, Changelog, Owner (a question for the owner), Disk freed, and last `Attribution:` with your commit trailer lines below it.
* `Result:`, `Gate:` (`green` or `red <check>`), `Deviation:`, `Found:` and the other fields with nothing to say hold the bare word: `done`, `none`. Anything after it reads as content.
* `Deviation:` only for a step of the brief built otherwise than written, or not built. `Found:` only for a defect outside your change. Everything else (your choices, how you solved the step, why a test's expected output changed) goes into `Body:`; how you verified goes into `Tests:`.
* Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).
