# Self-hosting plan: the core of `emit` and `check` (#125, #558)

Decided by read-only architecture reviews on 2026-10-11. No owner decision is
open. A builder reads only its own section.

## Done

* `CheckFacts` (`tools/check_facts.nika`) is the one set of tables the checker
  leaves for the emitter: `Checked.facts`, `CheckState.facts`, `Emitter.facts`.
  Keys are `i64`.
* `EmitOut`, `EmitState`, `EmitWalk`, `EmitWorld` (`emit_out.nika`,
  `emit_state.nika`); `EmitFlow` and `EmitScope` declared in `emit_kinds.nika`;
  `grammars`, `keep_plans`, `methods` are tables of `EmitState` (E1-E6).
* Check: C-1 (no address keys, `Integer` for `i128`, `enter_boundary` /
  `leave_boundary`), ADR-291 D25 (struct pattern `field: pattern`).

## Facts that settle old questions

* No `i128`, `split_once` or integer `parse` is needed: `tools::integers::Integer`,
  `text::parse_i64/u32`, `splitn(2, sep).collect()`, `before_last`.
* `(A, B)?` is not written (ADR-278 D1): use a named struct.
* NK2302: a function with more than one view parameter cannot return a view;
  clone, or a free function with one view parameter.
* A method's `fn` parameter lowers to `impl Fn` by value (recursion fails,
  E0382); free functions lower to `&impl Fn`. Fix in the lowering.
* A by-value parameter that is only read lowers as `&T`; `p with {..}` then
  writes `..p`. Write derivations as `self` methods.
* `HashSet`/`HashMap` fields lower to `TrustedSet`/`TrustedMap`: use
  `BTreeSet`/`BTreeMap`. `Symbol` is not `Ord`: key by text.
* Function names are global across `tools/*.nika`: prefix per module.

## Emit (module `emit/`)

* **E7** `emit_below*.nika`: the layer below the core (103 methods) as
  `impl EmitState` (`ref self`, world, `mut walk`); the 13 methods that take a
  flow take `flow.function` and `flow.statement` instead.
* **E8** `emit_entry.nika`: the unconnected group (`entry_point`,
  `trait_method`, `held_structs`, `user_main`), not `with_contracts`.
* **E9** the 47-method cycle (`expr_as_written`, `a_stmt`, `block`, `args`,
  `method_call`, `call_inner`, `called`, `overlap`, `select`,
  `template_segments`, `match_over_a_sum`, `pattern_written`,
  `grammar_entry`...) in ONE step, `emit_core_*.nika`. `EmitFlow` (11 bools +
  `statement`, `Copy`, derived forms `self with {..}`) and `EmitScope` (owned
  text and lists, passed `ref`; the 17 scope-changing sites use
  `outer.clone() with {..}`). `Build`, `Target`, `BoundsChecks` stay Rust:
  `EmitBuild` (Copy, bools) filled by `Build::for_emit()`. Hand-over tables
  (template holes, wrapped holes, kept calls) are filled from `Parsed` after
  the check with the same ids; a missing key is refused, never rebuilt.
* **E10** the layer above (`function`, `item`, `function_body`, `program`,
  `grammar`, `fold`). **E11** `emit/mod.rs` shrinks to the adapter
  (`with_contracts`, entry points, `SourceMap`, `Build`, `EmitRefused` to
  anyhow).
* `take_lend` / `insert_after_first_brace` stay Rust (`OutText`) until `std`
  has `String::pop` / `insert_str` (owner), or `EmitOut` writes lazily.

## Check (module `check/`)

* Compiler prerequisites, before C-6: a method's `fn` parameter as
  `&impl Fn`; tests for both.
* **C-2** `parsed_view.nika` `ParsedView { words, aliases, items, package }`;
  `CheckWorld` gains `here`, `beside`, `here_at`, `fields_by_base`, `newly`;
  the 15 item-reading functions to `check_items.nika`.
* **C-3** the remaining `Checker` fields into `CheckFlow` (`check_flow.nika`).
* **C-4** `trait CheckHost` (`check_host.nika`: `hole`, `kept_call`, `intern`,
  `wrap_hole`, `evaluate`, `compute_comptime`; ids stay with the host),
  implemented in `check/host.rs`; the 7 host-needing functions.
* **C-5** the layer below the core, leaves first: 183 plain functions
  (~3,270 lines), then 42 that read fields (~1,490), commits of ~800 lines.
  A Nikaia block visitor may reuse `emit_walk.nika::visited_in_block`.
* **C-6** the 24-function cycle (`value_of`, `stmt`, `call_on`, `call`,
  `arguments_given`, `block`, ...) in ONE step as
  `impl CheckState` `[H: CheckHost](ref mut self, world: ref CheckWorld,
  host: ref H, ..)`; `value_of` (2,953 lines) split into one method per
  `Expr` variant; `check_core_*.nika`. A deep-nesting stack test passes before
  and after. Callgrind counts (not time) before C-1's successor and after.
* **C-7** the layer above (13) and the drivers (58); `check/mod.rs` is the
  adapter plus `host.rs`.

## Risks

Stack depth of a huge `match`; findings order, `read_seq`, `next_choice` and
fresh ids must come out in the same order or emit and check disagree silently;
`RefCell<EmitWalk>` borrows (never hold one across a call back into the
emitter); clones where Rust returned views (measure by callgrind).
