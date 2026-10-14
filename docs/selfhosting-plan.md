# Self-hosting plan: the core of `emit` and `check` (#125, #558)

Decided by read-only architecture reviews on 2026-10-11. No owner decision is
open. A builder reads only its own section.

A first draft of a Rust function comes from the converter
`crates/rs2nika/convert` (`nikaia build` there; `target/nikaia/bin/rs2nika
--profile check|emit --prefix X_ F.rs`, `--file` for a whole tools file). It
names on stderr the callees not yet in Nikaia and the functions tools/*.nika
may already have; check those before moving a duplicate. Rust's types for it
come from rust-analyzer (ADR-001 D5, `crates/rs2nika/facts`).

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

## E9 in units (host-trait strangler, decided 2026-10-11)

Not one commit and not a file per agent: the core moves through a
`trait EmitHost` in 13 sequential units, each of about 100 turns, each ending
with `lower-std`, tests and a push. Cutting only `expr` and
`block_opening_with` turns the 47-method cycle (7,286 lines) into a DAG, so the
host needs those two plus the Rust-only helpers until U10 closes the cycle.
Call-graph scripts: `scripts/e9/` (`callgraph.py`, `units.py`, `arms.py`).

* **Method form:** `emit_core_<name>[H: EmitHost](ref self, world: ref EmitWorld,
  mut walk: EmitWalk, mut out: EmitOut, host: ref H, .., depth: i64,
  flow: EmitFlow, scope: ref EmitScope) sync throws`, `impl EmitState`. A moved
  callee is `self.emit_core_*`, an unmoved one `host.<name>`. The Rust method
  becomes a one-line stub through `self.nikaia(..)` (`emit/host.rs`).
* **Adapter (`emit/host.rs`, trait in `tools/emit_host.nika`):** the walk is a
  baton, `w: RefCell<Option<EmitWalk>>`: a stub takes it, passes `&mut`, puts it
  back; every host method swaps it in and out; a forgotten swap panics. `Flow`
  converts only at the crossing (`to_emit` / `from_emit`). An anyhow error from
  a `refused_at!` is stashed in `failed: RefCell<Option<anyhow::Error>>` and a
  marker error returned; the stub returns the stash, else maps `EmitRefused`.
  `expr_as_written` and `a_stmt` move one variant per Nikaia method; guarded
  arms of one variant move together in their original order; the Rust match
  stays the dispatcher until U10.
* **Oracle per unit:** `lower-std` with the previous unit's binary and with the
  new one on the same tree: `package.rs` byte-identical; then the NK tests the
  moved arms touch and `examples.rs`.
* **Units:** U0a compiler: a trait declaration's `mut` parameter lowers to
  `&mut T` (today by value in the declaration, `&mut` in the impl; see
  `trait_method`), with a test. U0b scaffolding (`emit_host.nika`,
  `host.rs`, baton, flow conversion, error stash, `Build::for_emit()`, pilot
  `operand` / `bare_range` / `index_expr`, test that a refusal keeps its line).
  U1a types (`ty_counted`, `ty`, `pointer_for`, `hull_new`, ...). U1b the
  remaining below-core code (`held_in`, `filled_at_once`, `options_of`,
  `caught_*`, keeps, ...). U2 leaves (`emit_core_leaves.nika`: `key`,
  `string`, `template*`, `block`, `if_expr`, `grammar_entry`...). U3 reads
  (`overlap`, `select`, `receiver`, index reads). U4 match (`pattern_written`,
  `arm_body*`, `match_over_a_sum`). U5 `args`, `called`. U6 `call_inner`,
  `call`, `method_call`. U7 places (SafeMethod, Field, Index, Cast, StructLit,
  With, Tuple, ListLit). U8 flow (Closure, Binary, Coalesce, Try*, Return,
  Break, Throw, Spawn). U9 all `a_stmt` arms. U10 the remaining arms and
  `expr` / `block_opening_with` / `stmt` / `a_stmt`; every `host.expr(` becomes
  `self.emit_core_*` and those trait methods are deleted.
* **Stay Rust:** `take_lend`, `insert_after_first_brace` (no `String::pop` /
  `insert_str` in `std`); ~10 queries on Rust-only code (`prove::call_key`,
  `fold::constant_of`, `order::block_mentions`, `throws::error_type`,
  `keep::root_of`, `dsl::type_name`, `sync::visit_expr` or Nikaia's
  `visited_in_*`).
* **Risks:** no callgrind before U10 (each crossing allocates an `EmitScope`);
  an error that loses its identity across a crossing breaks diagnostics
  silently; arms reordered; stack depth of generic frames (keep a deep-nesting
  test); findings, `next_choice` and id order; units must not overlap in time
  (`package.rs`).

## C-6 in units (host-trait strangler, decided 2026-10-11)

This replaces C-6's "in ONE step". It is the E9 pattern applied to check: the
core moves through `CheckHost` in 12 sequential units of about 100 turns each.
Before them come four below-core units (C-5e to C-5h). Every unit ends with
`lower-std`, the oracle, the touched tests and a push.

**The cycle, measured at 8164b5d.** Call graph of `check/mod.rs`, comments
stripped (`scripts/c6/cg.py`): the SCC holds **25 functions and 6,703 lines**.
The figure of 32 counted false edges from comments such as "is walked (". The
SCC is `value_of` (2,968 lines), `stmt` (857), `call_on` (585), `call` (476),
`comptime_binding` (381), `list_literal`, `arguments_given`, `an_assert`,
`locks`, `a_safe_call`, `returns`, `holes`, `expr`, `a_called_field`,
`a_kept_lambda`, `a_field_of_a_borrowed_subject`, `lambda`,
`a_block_over_a_grammar`, `a_field_of_a_lent_place_handed_back`,
`a_mut_parameter_given_away`, `a_compound_write_to_a_map_slot`, `block`,
`an_instantiation`, `grammar_call` and `literal_by_use`.
`program`, `function`, `grammar`, `fold`, `folds_in`, `item_constants`,
`computed_*`, `a_field_default`, `walked`, `unroll` and `instantiations_*` sit
**above** the SCC (C-7). Below it there are **100 Rust functions that are not
stubs yet (3,591 lines)**.

**The cut is `expr` and `block`.** No single function cuts the cycle. Four
pairs do: {expr, block}, {expr, stmt}, {value_of, block}, {value_of, stmt}.
`expr` (82 lines, 19 callers in the core) and `block` (27 lines) are thin
entries, so they become the two hub methods of the host. The other 23
functions then form a DAG of depth 4. `block` stops being a hub in U-c7, and
`expr` in U-c11.

**The arms of `value_of`.** There are 39 arm heads for 37 variants, listed by
`scripts/c6/arms2.py`. `MethodCall | SafeMethod` is one arm of 638 lines, and
`Field | SafeField` is one arm too. The three guarded `Dsl` arms move together
in their original order, and so does `Break | Continue`. Each variant becomes
one Nikaia function `check_core_<variant>`. The Rust match stays the
dispatcher until U-c11, and a moved arm becomes
`Expr::X { .. } => self.nikaia(|s, w, h| check_core_x(s, w, h, expr, span))`.
`stmt` has 8 `Stmt` arms. It moves whole in U-c7, one function per variant.

### Form of the moved code

**Free functions, not methods.** The form is
`pub fn check_core_<name>[H: CheckHost](mut state: CheckState, world: ref CheckWorld, host: ref H, ..) -> Ty sync`.
A probe on the 8164b5d binary showed why:

* `host.expr(self, ..)` inside a `ref mut self` method is refused with NK1102
  ("expects `CheckState`, passing `ref CheckState`").
* A free function's `mut state` is accepted, and it is handed on to the trait's
  `mut` parameter as `&mut CheckState`.
* A `ref mut self` method may call `check_core_f(self, ..)`.

A moved callee is called as `check_core_*`, and code below the core as
`state.<method>(world, ..)`. A hub (`expr`, `block`) and the few Rust-only
queries are called as `host.<name>(state, ..)`. Nothing throws (check returns
`Ty`; findings are state), so there is **no error stash**: this is simpler than
emit.

**Baton (`check/core.rs`; prototype `baton3.rs` compiled and run).**

* **`Checker.world` becomes `&'a CheckWorld<'a>`.** `walked` builds it as a
  local, and nothing assigns it after that. A stub can then hand `world` to
  Nikaia while `Core` holds `&mut Checker`.
* **`Checker.s` becomes `Baton(Option<Box<CheckState>>)`**, with
  `Deref`/`DerefMut` that panic while the state is out. The 262 `self.s.` sites
  stay as they are, and a forgotten swap is a panic, never a silent empty
  state.
* **A stub** (`Checker::nikaia`) takes the box out, builds
  `Core { c: RefCell<&mut Checker>, h: host::Host, spare: RefCell<Option<Box<CheckState>>> }`,
  runs the Nikaia function on `&mut *s` and puts the box back.
* **A host method** (`Core::hosted`) swaps the state it is handed with the heap
  spare (`mem::swap`), lends the box to the checker, runs the Rust and swaps it
  back. The spare comes from a pool `Checker.spares`. When the pool is empty, a
  `#[cold] #[inline(never)]` constructor makes one, from
  `CheckState::hollow()` (all empty, `CheckFlow::start(false, false)`).
* **Measured with a 4 KB state on an 8 MB stack:**
  * `Option<CheckState>` with `mem::replace`: 8.6 KB per crossing (max depth
    977).
  * The boxed baton with a cold spare: 192 bytes (max depth 43,593).
  * A box built inside the stub's frame: 4.3 KB.
* **Never build a `CheckState` in a stub's or host method's frame.**
* **`impl CheckHost for Core`:** the 13 existing methods delegate to `self.h`.
* **New in `CheckHost`:**
  * U-c0:
    `fn expr(ref self, mut state: CheckState, node: ref Expr, span: ref Span) -> Ty sync`
    and `fn block(ref self, mut state: CheckState, block: ref Block) -> Ty sync`.
  * U-c1 (built): `holes_read(literal) -> Vec[(Expr, Vec[(String, String)])]`,
    for `holes`: an `f"…"`'s holes as `emit::hole_as_read` reads them, and a
    template's with the names its `<for>` binds (`literal_expressions_bound`).
  * C-5f: `newly_throws(key) -> Vec[String]?`.
* **`host::Host` implements `expr` and `block` as `unreachable!`:** code below
  the core never calls a hub. U-c7 deletes `block` and U-c11 deletes `expr`.

**Order stays identical by construction.** The findings, `read_seq`,
`flow.next_choice` and `next_binding` live in `CheckState` and travel with the
baton. `finished` sorts the findings stably by span, so the push order inside
an arm must be kept. `NodeId::fresh` has four sites: the closure of the
`Spawn` arm and the three in `a_block_over_a_grammar`. They become
`ast::fresh_id()`, on the same global counter, in the same order of
evaluation. Interning goes through `host.intern` in the same order. Check has
no `HashMap`.

### Before the cycle (can start now, leaves first, ~800 lines per commit)

`scripts/c6/below2.py` lists them by level. `scripts/c6/units_c.py g.json mod.rs below.txt`
gives what each core unit calls.

* **C-5e: 55 functions, 1,213 lines** (level 0, no Rust helper;
  `self.parsed.text` becomes `world.words`), in two commits:
  * First: `a_value_of_no_known_type`, `a_member_this_type_does_not_have`,
    `a_sequence_that_cannot_do_this`, `a_length_that_fits_its_buffer`,
    `a_lambda_that_pauses_at_an_entry`, `a_cleanup_that_moved`,
    `raised_bounds`, `a_variant_matched_on_a_nullable`, `reached_method`,
    `a_number_read_through_a_lent_binding`, `the_one_error`, `payload_of` ...
  * Then the small ones: `hands_over`, `several_*`, `arms_meet_at_null`,
    `tail_of`, `local_free`, `specialised`, `written`, `is_hull`, `a_map`,
    `unpromised` ...
  * Also: `MultiLock` becomes the `CheckMultiLock` enum.
* **C-5f: 11 functions, 576 lines, together with the Rust-only helpers they
  call:**
  * `sync::visit_expr`, `visit_stmt` and `visit_stmt_blocks` become a Nikaia
    `check_visited_in_*`: `emit_walk::visited_in_*`, plus the holes of a
    literal through `host.literal_holes`, because sync's visitor reads holes.
    Check that both visit the same expressions on the oracle corpus before
    deleting the Rust.
  * `sync::reached` becomes `calls::callee_of` called directly.
  * `order::operation` becomes `order::accounted`.
  * `send::names_used` becomes `names::names_in`.
  * `emit::interpolated_holes` and `emit::visit_expr` become Nikaia.
  * `build_time::an_escape_nothing_names` becomes `escapes`.
  * The functions: `a_member_nothing_writes_down`,
    `branches_meet_on_nothing`, `gives_back`,
    `an_update_block_reads_what_it_writes`, `a_name_in_text`,
    `made_from_text`, `newly_reaches_a_handler`,
    `a_guard_reading_inside_a_box`, `pauses_here`,
    `a_number_that_does_not_fit` and `an_escape_nothing_names`.
* **C-5g: 28 functions, 1,198 lines** (levels 1-2), in two commits:
  `the_compiler_writes_the_reference`, `may_fail_here`, `expect`,
  `expect_kept`, `arithmetic_on_*`, `an_operation_that_overflows`,
  `a_claim_reaches`, `a_value_with_a_cleanup_*`, `parts_beside`,
  `widened_slot`, `a_type_that_does_not_compare`, `a_write_that_skips_the_door`,
  `walk_the_body`, `what_the_body_does`, `a_pending_coalesce_*`, `unwritable_in`,
  `written_below`, `a_table_that_crosses`, `two_number_types` ...
* **C-5h: 4 functions, 586 lines:** `arguments` (390), `a_map_key`,
  `a_view_kept_where_the_receiver_says_text` and
  `a_task_holds_these_across_a_pause`.
* **What stays Rust:** the adapters `host()` and `others()`, and `value_node`,
  which Nikaia code replaces with `id_of`.

### Units of the core (in order; `units_c.py` checks that every callee was moved before or is a hub)

| Unit | File (`tools/`) | What moves | Lines |
| :--- | :--- | :--- | ---: |
| U-c0 | `check_core_pilot.nika` | Scaffolding: `world` as a reference, the boxed baton, `core.rs`, the hubs `expr` and `block`, `scripts/c6/`, the oracle, `check_deep_nesting.rs`, the callgrind baseline. Pilot: `grammar_call`, `a_compound_write_to_a_map_slot`, `an_instantiation`, `a_mut_parameter_given_away` (needs `specialised` and `written`). U-c0 may come before C-5e. | 115 |
| U-c1 (done, 0.0.907) | `check_core_leaves.nika` | `a_field_of_a_borrowed_subject`, `a_field_of_a_lent_place_handed_back`, `a_kept_lambda`, `literal_by_use`, `holes`, `lambda`, `returns`, `an_assert` | 537 |
| U-c2 (done, 0.0.910) | `check_core_comptime.nika` | `comptime_binding`, `a_block_over_a_grammar` | 429 |
| U-c3 (done, 0.0.912) | `check_core_args.nika` | `arguments_given`, `locks`, `list_literal`, `a_called_field` | 524 |
| U-c4 (done, 0.0.916) | `check_core_call.nika` | `call`; the arms `Call`, `ListLit`, `Tuple` | 499 |
| U-c5 (done, 0.0.922) | `check_core_call_on.nika` | `call_on`, `a_safe_call` | 688 |
| U-c6 (done, 0.0.924) | `check_core_method.nika` | the arm `MethodCall \| SafeMethod` | 638 |
| U-c7 (done, 0.0.927) | `check_core_stmt.nika` | `stmt` (one function per `Stmt` variant) and `block`; the host method `block` is deleted | 884 |
| U-c8 (done, 0.0.943) | `check_core_flow.nika` | the arms `Match`, `If`, `Select`, `Overlap`, `TryCatch`, `Block`, `Unsafe` | 609 |
| U-c9 (done, 0.0.930) | `check_core_places.nika` | the arms `Field \| SafeField`, `StructLit`, `With`, `Index`, `Cast`, `Range` | 709 |
| U-c10 (done, 0.0.948) | `check_core_ops.nika` | the arms `Binary`, `Coalesce`, `Closure`, `Spawn` | 683 |
| U-c11 | `check_core_expr.nika` | the literals, `Variable`, `Path`, `Try`, `Throw`, `Return`, `Break \| Continue`, `Dsl` (3 arms), `Asm`; the dispatcher `value_of` and `expr`. Every `host.expr(` becomes `check_core_expr(`, the host method `expr` is deleted, and callgrind runs | 387 |

**Oracle per unit:**

1. **`lower-std`** with the previous unit's binary and the new one, on the same
   tree: `package.rs` must be byte-identical. Check runs over the 173 `tools`
   files, and its facts drive emit.
2. **`scripts/c6/oracle.py OLD NEW .`**: both binaries lower every
   `tests/**/*.nika` and `examples/**/*.nika` (142 files, about 35 s) with
   `--no-cache`. The exit code, the diagnostics (text and order of the findings)
   and the lowered `.rs` must be byte-identical. Two runs of the same binary
   came out identical.
3. **Tests:** the NK tests whose codes the moved code raises (grep them in
   `crates/nikaia/tests/*.rs`), plus `specification.rs`, `examples.rs`,
   `check_deep_nesting.rs` and `self_hosting.rs`.

**Callgrind:** a baseline at 8164b5d of `lower examples/foreign-runtime/overlaps.nika`:

| Count | Ir |
| :--- | ---: |
| Whole run | 42,643,414 |
| `check::walked`, inclusive | 20,462,992 |

Measure again after U-c11 only.

**The deep-nesting test (`check_deep_nesting.rs`, in U-c0).** It runs at the
default 8 MB main stack. Today's limits:

| Shape | Limit today | Where it stops |
| :--- | :--- | :--- |
| `id(id(..))` calls | passes 800, overflows at 900 | the parser |
| `[[..]]` lists | passes 500, overflows at 1,000 | the parser |
| `.abs()` method chain | passes 1,600, overflows at 1,800 | after parsing (check + emit) |
| `?? ` chain | passes 500 | |

The test uses calls 500, lists 500, `??` 500 and the chain at 1,000. It must
pass after every unit. Nested parentheses, `if`s and blocks are **exponential
in the parser** (16 levels take 0.6-1.7 s, and 16 blocks with a missing `}`
take more than 20 s), so they are not usable here; file an issue for them.
If a unit fails the test, make its frames smaller: one function per arm, no
`CheckState` or large `Ty` temporaries in a frame. Only if that fails, a
compiler thread with a larger stack, decided in that unit.

**What stays Rust:** `check/core.rs`, `host.rs`, the stubs called by the layer
above (C-7), and `value_node`/`host()`/`others()` until C-7.

**Risks:**

* **The crossing costs.** Each one swaps a `CheckState` twice (heap to heap)
  and copies the `Host`. No callgrind before U-c11.
* **Arms reordered,** or a finding pushed in a different order (`finished`
  keeps ties in push order).
* **`fresh_id` and `intern` calls reordered.**
* **Units overlapping in time.** `mod.rs` and `package.rs` are shared, so
  strictly one unit at a time, C-5e to C-5h included.
* **`Checker.spares`** must not leak a state that is still lent: assert when it
  is pushed back.
