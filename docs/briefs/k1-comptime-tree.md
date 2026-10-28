---
agent: builder
model: opus
max_turns: 120
budget_usd: 25
---
# Brief: K1+K2, the `comptime` sub-program's tree work of `comptime_run.rs` into Nikaia (Part #125)

Score: self-hosting lane, bootstrap week lane 1 (`docs/bootstrap-week.md`, Thu 10-29 row: P1 settings, K1+K2 comptime tree, P5). Plan rows K1 and K2 of `docs/selfhosting-plan.md` (*Large files: project.rs and parser/mod.rs*), together: K2 is below 300 lines. S=2, C~3-5 M (comparable: P4b, M2). P1 (aab4d74) and P4b (8394790) are on `develop`, self-hosting 80.9 %.

Cut: crates/nikaia/src/comptime_run.rs:35-286
Cut: crates/nikaia/src/comptime_run.rs:439-484
Cut: crates/nikaia/src/comptime_run.rs:758-844
Cut: crates/nikaia/src/comptime_run.rs:914-1093
Convert: --profile check-free --prefix comptime_

The driver converts these before you start and names the draft (`.draft.nika`, `.notes`, `.rs`) in your prompt: start from it, do not cut or convert again. The architect's trial run of the same cut drafted 6 TODOs: four *`collect` into an `Option`* (write the `for` loop that returns `null` at the first miss), one `Some(` in a struct pattern (#886: bind the field, test `!= null`), one `Option::map` (`let x = e ?? return null`). The draft types every `&Parsed` as `ref CheckWorld`: that is the converter's guess and wrong, see the host shape below. `--prefix` was applied only to the three clashing names: prefix every function of the new file `comptime_`.

**If `crates/nikaia-compiler/src` exists, R0 (#921) has run**: read every `crates/nikaia-std/src/tools` below as `crates/nikaia-compiler/src`, and `nikaia_std::tools::` as `nikaia_compiler::`.

## Facts
* Plan row K1 (copied): `VALUE_FN`, `sub_program`, `constants_as_lets`, `unit_without_its_constants`, `without_readers`, `reads_any`, `constants_of` (36-287); `a_function`, `literal_of`, `held_as`, `written_type` (914-1103) into a new `comptime_tree.nika` (prefix `comptime_`: `a_function`, `literal_of`, `written_type` exist in `ty.nika`, `ledger_items.nika`, `views.nika`). Host: Nikaia takes the items and `ref winnow_grammar::InternerContext` (as `names_in_block` does) and returns the items to add; `Parsed::keeping` stays Rust; `known`/`typed` are callbacks `fn(ref String) -> BuildValue? sync`, `fn(ref String) -> Ty? sync`. Row K2 (copied): `Computed`, `NOT_COMPUTED_YET`, `as_built` (440-485), `undeclared_callee`, `culprit` (761-861) into `comptime_tree.nika`; `Ledger` is Nikaia's; the `AHEAD` `thread_local` stays Rust with `worked_out_ahead`.
* **The precedent for the host shape is M2** (`tools/modules.nika` :523 `modules_dispatcher_as_main`, :557 `modules_tests_as_functions`, called from `crates/nikaia/src/modules.rs` :1020-1037): `mut items: Vec[Spanned[Item]]` (Rust passes `&mut parsed.program.items`), `names: ref winnow_grammar::InternerContext` (Rust: `&parsed.interner`; a symbol's text is `words.resolve(sym)`, see `names.nika` :26 or `a_symbols_text`), `intern: fn(ref String) -> winnow_grammar::Symbol sync` (Rust: `&|text| interner.intern_string(text)`). New node ids: `ast::fresh_id()` (`tools/ast.nika` :848).
* **The Rust signatures stay, as thin adapters** in `comptime_run.rs`, so the callers do not change: `sub_program`, `unit_without_its_constants` (they call `parsed.keeping(..)` in Rust, then the Nikaia function over `kept.program.items`; `keeping`'s predicate may be a Nikaia `fn(ref Item, words) -> bool`), `without_readers` (the fixed-point loop over `&mut [Parsed]` may stay Rust around a Nikaia per-unit `comptime_drop_readers(mut items, words, gone) -> Vec[String]`; your choice, say it in `Body:`), `constants_of`, `as_built`. Callers: `comptime_run.rs` :527-560 (`computed`), :577, :623, :627, :1117 (`VALUE_FN`); `grammar_run.rs` :335-336 (`constants_of`, `without_readers`); `contracts/mod.rs` :1276 (`as_built`), :1290 (`Computed::Value`); `check/host.rs` :283-297 (the match over `Computed`), :353 (`VALUE_FN`). `contracts/*` and `check/*` are lane 2's files (`docs/bootstrap-week.md` *File ownership*): **do not edit them**; keep `pub(crate)` Rust names they reach (a re-export or a one-line forward).
* **`Computed` already has its Nikaia twin**: `CheckComputed` (`tools/check_host.nika` :32, variants `Value(BuildValue)`, `Forbidden { callee: String, because: String }`, `Stopped(String)`, `NotHere(String)`); the Rust one differs only in `because: &'static str`. Make `comptime_run::Computed` a re-export of the lowered `CheckComputed` (`pub(crate) use ...check_host::CheckComputed as Computed;`), so `check/host.rs`'s match and `contracts/mod.rs` compile unchanged (`because.to_string()` on a `String` still compiles); delete the Rust enum. If clippy refuses something in those two files, stop and report under `Needs:` instead of editing them.
* `crate::build_time::Value` is `nikaia_std::tools::build_values::BuildValue` (`build_time.rs` :28); `Ty` is `ty.nika`'s; `Ledger` is `ty.nika`'s (its `functions` map is readable from Nikaia).
* `literal_of`'s `Value::Text` uses `crate::build_time::written`, which is `nikaia_std::tools::escapes::as_a_literal` (`build_time.rs` :357): call `escapes::as_a_literal` from Nikaia. `Value::Float` writes `format!("{f:?}")`: check the Nikaia `f64` text against Rust's `{:?}` (`1.0`, `0.1`, `1e100`, `-0.0`) and, if they differ, keep a callback `fn(f64) -> String sync`; name it in `Body:`.
* `undeclared_callee` and `culprit` walk with `crate::emit::visit_expr` (`emit/mod.rs` :1301, pre-order); the Nikaia walk is `emit_walk::visited_in_expr(expr) -> Vec[ref Expr]` (`tools/emit_walk.nika` :64, the same order, this one included): loop over it and stop at the first hit. `culprit`'s `forbidden` reads `crate::build_time::may_not_run(contract)` (`build_time.rs` :365, Rust, K3's): pass it as a callback `fn(ref String) -> bool sync` built in Rust over `derived`/`library`, or move the three-line test if `FnContract`'s fields are readable in Nikaia; your choice.
* `std` counterparts: only `collections::BTreeSet`/`BTreeMap` (`insert`, `contains`, `len`, a `for` over it), `Vec`, text `split("::")`, `join`; no file system, no process. `std` has no `Seq.chain` (#924) and `BTreeSet` no `extend`: write loops (P4b did).
* Stays Rust: `DEFAULTS`/`KNOWN` `thread_local`s, `defaults_compiled_in`, `defaults_known_from`, `known_default`, `defaults_workshop` (845-912), `NOT_LOWERED`/`NOT_READ`/`NOT_CARRIED`/`NOWHERE` (746-756, `computed`'s), everything of 288-438 and 486-745 (K3), `with_driver` and below.
* Known open lowering defects, not yours: #909 (write `.clone()` where rustc asks), #916 (match on the field). Name each use in `Body:`.
* Do not touch `crates/rs2nika/` (the converter lane's uncommitted `convert/src/main.nika` is in the checkout); the converter's costs go under `Rs2nika:`.
* Tests that exercise this code: `crates/nikaia/tests/it/{comptime,comptime_compiled,comptime_item,comptime_order,build_time,build_time_dependencies,grammar_at_build_time}.rs`; `tests/compiler` programs with `comptime`.

## To build
1. Start from the driver's draft (report its `kept N of M` under `Rs2nika:`). Write `tools/comptime_tree.nika` with the K1 and K2 functions and their doc comments, in the host shape above; turn the Rust ones into thin adapters or delete them where no Rust caller is left; `Computed` becomes the re-export. `lower-std`: `comptime_tree.rs` is new and the index `lowered.rs` gains it; no other lowered file changes. Every printed message stays byte for byte.
2. In `docs/selfhosting-plan.md` mark K1 and K2 built (`comptime_tree.nika`), and add `comptime_tree.nika`'s line to `scripts/self_hosting.py` (beside `comptime_driver.nika`, :122, in the same form: `"comptime_run: ... (K1, K2)"`); report the self-hosting % under `Tests:`.

A lowering defect met is fixed in the compiler as its own commit with a test (*Fix, do not work around*, `CLAUDE.md`), except #909 and #916 above; if it is more than one step, stop and report it under `Needs:`.

## Done when
`nikaia test --project tests/compiler` once, no `--both-settings` (no `check_*`/`emit_*` file changes, bootstrap-week L4c); `it` tests filtered on `comptime build_time grammar_at_build_time` plus the guard set of `.claude/agents/builder.md` (not `nikaia_test::the_compiler_test_package_*`), once, at the end, beside clippy; `cargo test --profile fast -p nikaia --lib comptime_run::` passes; clippy clean; lower-std fixpoint. Commit `comptime_run: the sub-program's tree work into Nikaia (K1, K2)`, `Part: #125`. Changelog: none (a move).

## Stop and report if
An open design choice beyond the ones left above; a `.stderr` or message that would change; a missing `std` call; an edit needed in `contracts/*` or `check/*`; an error in another agent's file.

Read `docs/builder-facts.md`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/k1-comptime-tree.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
