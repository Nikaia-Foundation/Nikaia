# Self-hosting plan: the core of `emit` and `check` (#125, #558)

Decided by read-only architecture reviews on 2026-10-11. No owner decision is
open. A builder reads only its own section.

A first draft of a Rust function comes from the converter
`crates/rs2nika/convert` (`nikaia build` there; `target/nikaia/bin/rs2nika
--profile check|check-free|emit --prefix X_ F.rs`, `--file` for a whole tools file). It
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
* `(A, B)?` is written (ADR-364) once built; until then a named struct.
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
  new one on the same tree: `lowered/*.rs` byte-identical; then the NK tests the
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
  (`lowered/*.rs`).

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
| U-c11 (done, 0.0.954) | `check_core_expr.nika` | the literals, `Variable`, `Path`, `Try`, `Throw`, `Return`, `Break \| Continue`, `Dsl` (3 arms), `Asm`; the dispatcher `value_of` and `expr`. Every `host.expr(` becomes `check_core_expr(`, the host method `expr` is deleted, and callgrind runs | 387 |

**Oracle per unit:**

1. **`lower-std`** with the previous unit's binary and the new one, on the same
   tree: `lowered/*.rs` must be byte-identical. Check runs over the 173 `tools`
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

After U-c11 (0.0.954, same file, `lower --no-cache`, release, no LTO
change), against the binary of 0.0.948 (U-c10) on the same day:

| Count | 0.0.948 | 0.0.954 |
| :--- | ---: | ---: |
| Whole run, Ir | 48,890,123 | 48,531,833 |
| `check::walked`, inclusive, Ir | 24,801,374 | 24,421,249 |
| Mispredicted branches (Bcm + Bim), whole run | 409,901 | 419,213 |

The arms now call each other directly instead of crossing back through
`host.expr`; a name read crosses only when its type is a `T?`.

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
* **Units overlapping in time.** `mod.rs` and `lowered.rs` are shared, so
  strictly one unit at a time, C-5e to C-5h included.
* **`Checker.spares`** must not leak a state that is still lent: assert when it
  is pushed back.

## C-7 in units (decided 2026-10-14)

After U-c11 the core is Nikaia, and what is left in `check/` is the layer
above it, the drivers, and the Rust the host still crosses back into. Measured
at f813401 (`scripts/c6/cg.py ... json`, stubs left out): 62 non-stub
functions, 2,098 lines in `mod.rs`, plus `narrow.rs` (257) and
`attributes.rs` (595). Leaves first; the same form as C-6 (free functions over
`mut state: CheckState`, `world`, `host`), the same oracle (`lower-std`
byte-identical, `scripts/c6/oracle.py` 0 differ) and the same tests. Each unit
deletes the host methods it makes unneeded, from `check_host.nika`, `core.rs`
and `host.rs`.

| Unit | File (`tools/`) | What moves | Lines |
| :--- | :--- | :--- | ---: |
| U-d0 (done, 0.0.959) | `check_narrow.nika` | `narrow.rs` whole (ADR-382, #621 done): `proved_names`, `proofs_for`, `proved`, `reads_of_the_maybe`, `read_as_proved`, `a_proof_taken_back`; the four `Checker` fields become `CheckState.narrowing`; 6 host methods go | 257 |
| U-d1 (done, 0.0.968) | `check_crossings.nika` | What the core still crosses back for: `a_bound_handle_is_taken` with `handle_path`, `handle_expr_path`, `handle_loop_body`, `handle_named_in`; `a_value_with_a_cleanup_made`/`_bound`; `a_type_that_does_not_compare`, `a_standard_bound_not_met`, `language_impls` (`Comparable` needs `parts_beside`); `a_call_that_hands_its_number_back`, `a_lambda_that_pauses_at_an_entry`, `a_scope_that_pauses`, `a_task_of_a_scope`; `arguments` (`newly` moves into `CheckWorld`); 11 host methods go | ~480 |
| U-d2 | `check_attributes.nika` | `attributes.rs`: `attributes`, `attribute_marks`, `the_mark`, `attribute_arguments`, `an_attribute_read`, the three `an_attribute_*` findings, `literal_misfit`; `Mark` stays the ledger's (`mark_of` is emit's too) | 595 |
| U-d3 | `check_defaults.nika` | `computed_value`, `compiled_build_time_value`, `unwritable_in`, `payload_of`, `computed_default`, `a_computed_default`, `a_field_default`, `item_constants`, `comptime_binding` stub, `fold`, `folds_in`, `lambda` stub | ~420 |
| U-d4 | `check_function.nika` | `function` (322, one function per phase: parameters, body, ways out, findings at the end), `grammar`, `widened_slot`, `local_free`, `grown_text`, `expect`/`expect_kept` callers become `state.expect_*`, `reached_method`, `collect_comparisons`, `field_walks_in`, `collect_field_walks`, `walks_the_fields_of`, `shape_member` | ~640 |
| U-d5 | `check_program.nika` | `program`, `unroll`, `instantiations_in`, `instantiations_beside`, `collect_types`, `written_by`, `a_static_integer`, `kept_function_call` | ~420 |
| U-d6 | `check_walked.nika` | The pure half of `walked` (what it does after `CheckWorld` is built), `finished`, `unsigned_slots_proved` (`bounds::proven` as a host query), `unrolling_report`, `claims_report`. What stays Rust: building `CheckWorld` from `Parsed`, `check_against`, `check_for_the_ledger`, `propagation*`, `fallible_loops*`, `parsed_view_of`, `host()`, `others()`, `core.rs`, `host.rs` | ~330 |

Notes:

* **U-d0 first:** it is a leaf, touches no file the concurrent work edits
  (#614 views, #620 throws), and frees `Checker` of four fields.
* **U-d1 before U-d4:** `function` calls the crossings; once they are Nikaia,
  `Core::hosted` has only the hubs' Rust-only queries left.
* **`arguments` is a host method today** only because it reads
  `Checker.newly`; `newly` belongs in `CheckWorld` (C-2 named it), which U-d1
  adds.
* **U-d6 ends C-7:** `Checker` keeps `parsed`, `beside`, `reads`, `newly`,
  `world`, `s` and `spares`; `mod.rs` is the adapter.

## What is left of C-7 and E10/E11 (re-planned 2026-10-24, after daf562f3)

The adapter stays Rust by design: building `CheckWorld` from `Parsed`, the DSL
and the ledger, `check_against`, `check_for_the_ledger`, `propagation*`,
`fallible_loops*`, `parsed_view_of`, `host()`, `others()`, `core.rs`, `host.rs`.
What is left to move, as builder orders, each with what it waits for:

| # | Order | Waits for |
| :--- | :--- | :--- |
| C7-a | `finished` (findings sorted by `span.at()`: `Vec::sort_by_key` exists and is stable, probed), `a_static_integer` with its callers in Nikaia, `kept_function_call` | nothing |
| C7-b | `claims_report`: the colour decision (`NO_COLOR`, terminal) stays in the Rust caller and comes in as a `bool` parameter; the check stays free of I/O | nothing |
| C7-c | `unsigned_slots_proved`: `bounds::proven` as a callback `fn(..) -> bool sync` (the `bounds_basic.nika` pattern) until `bounds` moves | nothing |
| C7-d | `written_by` | #804 (`String::quoted`, Rust's `{:?}` of a string) |
| E10-0 | research brief (at most 80 lines): the call graph of the 215 functions left in `emit/mod.rs`, leaves first, with the names that clash with `check_*` (`function`, `item`, `program`) | nothing (read-only) |
| E10-1.. | `grammar`, `fold`, `function_body`, `function`, `item`, `program`, in the brief's order | #791 (methods matched by bare name), E10-0 |
| E11 | `emit/mod.rs` shrinks to the adapter (`with_contracts`, entry points, `SourceMap`, `Build`, `EmitRefused`) | E10; `take_lend` / `insert_after_first_brace` stay Rust until `std` has `String::pop` / `insert_str` (the owner's, not decided) |

C7-a to C7-c touch `check/mod.rs` and `check_walked.nika`; E10-0 writes only a
brief. They do not overlap.

**State 2026-10-24:** C7-a..c built (`claims_report`, `unsigned_slots_proved`,
`kept_function_call`); `finished` and `a_static_integer` stay Rust as adapter
code. Left: C7-d (#804), E10-0, E10-1.. (#791), E11. `panic(..)` in a tools file
does not lower yet (#805).
Later the same day: #791, #804, #805 built (0.0.1216); E10-1 done (2018cf10,
`emit_grammar.nika`); E10-2 and C7-d running. Open on the way: #806 (`match`
on a field of a list element moves out of `&`; E10-1 worked around it, step 3
removes that), #807 (re-entrant crossing panics in `lower-std`).
#807 not reproduced: on 64661b76 plus 2018cf10's `.nika` and Rust, the old binary
lowers all 218 files without a panic; the nested crossing the builder met came from
a binary and sources that disagreed on a host signature. Both batons (`Emitter::nikaia`,
`Checker::nikaia`) now take through `baton::out_of`, whose panic names the place of
the second crossing and the rule (a host method Nikaia calls hands the walk back with
`hosted`, or takes it as a parameter) and says to rebuild after `lower-std`. Still
a panic, not a refusal: `nikaia` returns the closure's `R`, 24 callers.
C7-d done (90843776, `written_by` with `String::quoted`): **C-7 is finished**;
`check/` (2,205 lines) is the adapter. The check side goes on outside this plan:
`contracts/` (6,925 lines), `parser/` (4,403) and the top-level modules
(`type_rules.rs`, `modules.rs`, `fold_bound.rs`, `comptime_run.rs`), each after a
research brief that orders it leaves first.
Then: #806 built (c4b6d01a), E10-2 built (efbc784e); R-2 (`contracts/maps.rs`)
and #807 running; #808 (a `match` arm's view of a `ref` path, ADR-282 D13) filed.
R-2 built (e0003acd, `contracts/maps.rs` to `tools/maps.nika`); its defects
filed as #809 (`v[i]?.clone()`), #810 (a matched scrutinee counted as kept),
#811 (`'a` on every `ref` parameter of a method of a struct with a `ref` field;
`maps.nika` passes `words` instead until it is fixed), #812 (`?? []` for a list
view). Next slice: **R-3 `type_rules.rs`** (1,382 lines).
Then: #807 built (bd535e67: a named diagnosis; the panic did not reproduce);
#811 steps 1-3 built (b7e98462), its last step blocked by #813 (NK2302 counts a
view stored into a local struct as kept); #808, #809, #810 and R-3 running.
**Note:** in the shared checkout the test binary sometimes does not compile
because of another builder's half-done files (seen: `Build::Release` in
`handed_over.rs`). Prove a change in a clean copy (a fresh worktree on the
commit) before reporting it, and say which.
R-3 chunk 1 built (f5504640, about 620 of 1,382 lines of `type_rules.rs` in
`tools/type_rules.nika`); **chunk 2** (the in-place rewriter, about 760 lines)
waits for #814 (`args[i]` to a `mut` parameter) and #816 (`let mut x = m[k] ??
continue`). Also filed: #815 (`bytes[a..<b].text()`), #817 (`m["k"]?.` on a
map). The reported `match b.stmts[i].node` move does not reproduce in a program
on a binary with #806; the R-3 builder retries it before chunk 2. #809 built
(4e5397df); #813 running.

**Next slices, in order** (from `docs/briefs/r1-next-rust-slices.md`):
R-4 the non-grammar leaves of `parser/mod.rs` (Trivia, `number_at`, the error
helpers, about 600 lines, prefix `Pr`; needs nothing), then R-5 the free
functions of `contracts/mod.rs` (lines about 931-1439, about 500), then R-3
chunk 2 once #814 and #816 are built, then R-6 the `visit_*` walkers of
`contracts/send.rs`/`sync.rs`, then `fold_bound.rs`, then `modules.rs`'s
`Program` core.
#813 built (d7164ae0) and closed; #818 (a parameter handed to a method on
`self` counted as kept; least fixpoint over the impl's methods) now blocks
#811's last step, and also carries #813's loosening to read-only arguments.
R-5 built: the free functions of `contracts/mod.rs` are in `tools/ledger_defaults.nika`
(`apply_attribute`, `set_default`, `write_result_sources` with per-unit callbacks) after #816.
E10-3 built: `function_body` is in `tools/emit_function_body.nika` (`emit_function_body`); the
host gained `block_plain`, `known_nonnegative`, `insert_after_first_brace` and
`insert_before_last_brace` (`OutText` stays Rust until `std` has `String::insert_str`). Dead in Rust
now: `never_returns`, `jumps`, `a_tail_that_enters_a_grammar`, `PANIC`, `ARM_VALUE`. Next: E10-4 `function`.

E10-4 switched: `Emitter::function` is a thin adapter over `emit_function`
(`tools/emit_function.nika`); 1326 Rust lines removed (the body and 16 items it alone used,
`check::specialised` among them), `lower-std` output unchanged. Next: E11.

**E11 plan** (call graph of `emit/mod.rs` at ae4bbb9d, 7691 lines, 212 fns; sizes are Rust lines
of the group, a helper counted where its only callers are). Stays Rust (the adapter):
`with_contracts` and its table builders (733), the entry points, `SourceMap`, `Build`,
`Target`, `EmitRefused`, `host.rs`, `take_lend`, `insert_after_first_brace`. Dead (no caller in
the repository): `emit_std`, `emit_std_items_against`, `emit_module_body`,
`holds_map_positions`, `BuildFlags::for_emit` (about 60 lines, delete with the slice that
passes them). Thin adapters already (3-20 lines over `nikaia_std::tools`): `ty`,
`ty_counted`, `count_at`, `function`, `grammar`, `handle_named`, `pointed_at`,
`writes_a_run`, `pauses`, `format_of`, `dsl_name`, `seeded` and most one-liners near the end.
Leaves-first slices:
1. E11-1 `item` and what only it calls (`trait_method(_written)`, `bridge`,
   `declared_wider`, `error_impl`, `opaque_type`, `foreign_ty`, `derives`, `boxed_ty`,
   `borrows`, `bounded`, `angled`): 1004 lines; calls only adapters.
2. E11-2 `pattern` / `pattern_body` / `patterns` / `fold` / `repeat_suffix` (169), called
   through the host's `pattern`.
3. E11-3 `program`, `items_only`, `entry_point`, `shadow_types`, `tether_wrappers`,
   `error_channel` (about 560): calls `item` (after 1).
4. E11-4 the core in one step: `block_plain`, `block_opening_with`, `stmt`,
   `stmt_written`, `a_stmt`, `expr`, `expr_as_written`, `expr_written` and their helpers
   (about 3600, the host's `expr` / `stmt` / `block_*` callbacks go with it).
5. E11-5 the template / literal-hole tables and `view_slots` / `borrowing_structs*`
   (about 350), then what is left of `with_contracts` that only feeds them.

E11-1 switched: `Emitter::item` is a thin adapter over `emit_item` (`tools/emit_item.nika`,
671 lines) that keeps only the refusal naming an unwritten item's tree; 1212 Rust lines
removed (`item`, the trait, bridge, `Error` and `extern` writers, `MethodOf`, `function`,
`grammar` and nine helpers only they used). The host gained `seeds_arity`, `place_*` and
`set_floor`; `lower-std` output unchanged. Next: E11-2 (patterns).

E11-2 switched: `pattern`, `pattern_body`, `patterns` and the `fold` adapter are `emit_pattern`
(`tools/emit_pattern.nika`, 141 lines), called by `emit_grammar` directly; the grammar's
bytes/raw flags moved from `Emitter` cells into `EmitWalk`, the host lost `pattern` and
`set_grammar_bytes`; 168 Rust lines removed (with `LIST_RULE`, `repeat_suffix`). `lower-std`
output unchanged beyond the four files of the slice. Next: E11-3 (`program`).

E11-3 switched: `program` and `items_only` are one adapter `unit` over `emit_program`
(`tools/emit_program.nika`, 229 lines: preamble, shadow types, unrolled copies, handle types,
`entry_point`, the allocator text); Rust keeps the source map, the empty abort table and the
refusal naming an unwritten item's tree (host `refuse_unwritten_item`; the host also gained
`main_params`). 519 Rust lines removed, with the dead `emit_std`, `emit_std_items_against`,
`emit_module_body`, `holds_map_positions`, `BuildFlags::for_emit` and the now uncalled
`item`, `error_channel`, `write_error_sums`, `walks_a_shape`, `walked_parameter`,
`uses_driver`, `held_structs`, `MAIN`, `ALLOCATOR`. `lower-std` output unchanged beyond the
slice's files. Next: E11-4 (the core).

E11-4 split (architect, 2026-10-25, at d9d3f921: mod.rs 5809 lines; the core calls itself
about 2100 lines deep, too much for one builder run): **4a** the expression family (`expr`,
`expr_unmixed`, `expr_as_written`, `expr_written` and the helpers only they call, about
1100-1300) into `tools/emit_expr.nika`; statements and blocks are reached through the host,
which already has `stmt` / `block_*` callbacks. **4b** `a_stmt`, `stmt`, `stmt_written`,
`block_plain`, `block_opening_with` (about 1300). **4c** the host's `expr` / `stmt` /
`block_*` callbacks and the adapters go, once both sides are Nikaia. Brief `e11-4a.md`.

E11-4a switched: `Emitter::expr` is a thin adapter over `emit_expr` (`tools/emit_expr.nika`,
527 lines): the held receiver, the narrowed read, the mixed, `Some` and nullable wraps,
the copy out of a map's read, the kept text bytes and the dispatch over every expression kind
to its `emit_core_*` writer (blocks and `if` through `emit_core_block` / `emit_core_if_expr`
directly). The host gained `char_literal`. 1212 Rust lines removed (`expr_unmixed`,
`expr_after_the_copy`, `breaking_out`, `expr_as_written`, `expr_written` and the stubs only
they called: `call`, `overlap`, `select`, `match_expr`, `string`, `method_arm`, `nested`,
`template`, `path`, `read_name`, `slices`, `copied_in_place`, `constructs_by_name`,
`boxed_future_in`, `integer_literal`, `unary_op`). Found on the way and fixed first: `m.get(k)`
lent to a `ref T?` parameter was opened twice (`map_gets`). `lower-std` output unchanged
beyond the slice's files. Next: E11-4b (statements and blocks).

E11-4b part 1 switched: `Emitter::block_plain`, `block_opening_with` and `stmt` are thin adapters over
`emit_stmt_block_plain`, `emit_stmt_block_opening_with` and `emit_stmt` (`tools/emit_stmt.nika`):
the settle point of a block (ADR-297), the one-line block, `vec![v; n]` for a list made `n` long, the
statement order, the `plain_tasks` note of a spawned `let`, and the fast-hash and unpinned map of a
`let` (`stmt_written`, `makes_a_map`). The host gained `a_stmt` and `order_of`. Removed from Rust:
`filled_at_once`, `settle_kind`, `makes_a_map`, `stmt_written` and the three bodies (about 290 lines).
Still Rust: `a_stmt` (about 950 lines, reached through `host.a_stmt`) and its callees (`handed_back`,
`around_value`, `returns_either`, keeps, `map_key`, `index_expr`...). Next: E11-4b part 2, `a_stmt`.

E11-4b switched: `Emitter::a_stmt` is `EmitState::emit_stmt_a_stmt` in `tools/emit_stmt.nika` (let,
comptime, the five kinds of assignment, `while`, `for`, `break`/`continue`, `return`, expression
statements); `EmitHost::a_stmt` is gone, and the host gained `given_up_unmuted` and `given_up_rebound`.
The Rust helpers only it called (`around_value`, `handed_back`, `map_key`, keeps, `tuple_pattern`...) are
deleted. Next: E11-4c.

E11-4c + E11-5 (#855) switched: Nikaia calls `emit_expr`, `emit_stmt` and `emit_stmt_block_*` itself; `EmitHost::expr`,
`stmt`, `block_plain` and `block_opening_with`, their `host.rs` impls, the Rust `Emitter::expr`/`stmt`/`block_*` and
the `Flow` struct with its `to_emit`/`from_emit` are gone. `tools/emit_tables.nika` holds `names_outside`,
`names_borrowing`, `borrowing_structs_beside` and `view_slots`; `mod.rs` keeps one-line adapters. Left in Rust:
`template_hole_tables`, `fold_callees`, `read_view_slots`, `tethered_types` (they read Rust-side `Parsed` caches and
the ledger).

Seeds writer switched (#867): `Place` and `Seeds::write` are `EmitSeedsPlace` and `emit_seeds_write` in `tools/emit_seeds.nika`; `emit/seeds.rs` keeps the reader and passes `arity` as a callback.
Seeds tables switched (#867 part 2): `Seeds` holds `EmitSeedsTables` (types, calls, barriers, fixed, private, excluded) and the queries (`arity`, `count`, `distinct`, `key`, `mentions_a_struct`, `barrier_names`) are Nikaia's; `emit_seeds_write` takes the tables. Left in Rust: the reader walks (`read_with`, `read_barriers`, `hands`, `self_handers`, `count_types`), `root_name` (walks `Expr`) and `shapes` (the ledger's `Shape`).

## C-7 rest and E12: what is left

Read at 62a07a1 (`check/` 2,199 Rust lines, `emit/` 3,684). Most of it is the adapter
and stays; what can move is about 2,450 lines in five slices. The rows are in build
order; C7-r1 and E12-1 touch disjoint files and may run side by side, the rest one
after the other. Line ranges are at 62a07a1: re-grep the names before cutting.

| Id | Rust (file:lines) | Target (`tools/`) | Host | Prerequisite | Rust lines |
| :--- | :--- | :--- | :--- | :--- | ---: |
| C7-r1 | `check/mod.rs`: `refuse_unproved_claims`, `refuse_assumes_in_std` (1181-1281, `ASSERT` is dead); the `CheckState` seeds `walked` reads off the item tree: `grammars`, `std_in_scope`, `std_modules`, `opaque_handles`, `foreign_names` (393-459); `payload_of` (77-91). `check/host.rs` `place_of` (355-386, `files()` becomes `world.beside`). `check/attributes.rs` `Mark`, `PLACES`, `mark_of` (40-87; `contracts/mod.rs` keeps calling an adapter). Delete the dead `a_byte`, `byte_literal`, `an_or_a`, `boxed_member`, the two empty `impl Checker<'_> {}` and the orphaned doc comments above them (607-672, 1135) | new `check_claims.nika` (the two refusals); `check_walked.nika` (`check_walked_seed`); new `check_attributes.nika` | removed: `CheckHost::payload_of`, `place_of`, `written` (Nikaia calls `check_walked_written` with `world.words`); none added | none | ~420 |
| E12-1 | `emit/mod.rs`: the item loop of `with_contracts` (`grammars`, `structs`, `methods`, `by_name`, `uses_std`, `fails`, 1254-1300), `structs_beside`, the `declared_errors` chain, `tethered_types`, `read_view_slots` (1950-1982), `described_maps` (806-823), `package_borrowing` (2039-2074), `Needs` with `of`/`join`/`preamble` (825-960) and `foreign_traits` (978-986; `comptime_run.rs`, `modules.rs` keep `emit::Needs` as a re-export), `abort_table` (461-529), `split_lifetimes` (1758-1827); `check/attributes.rs` `inline_written` (16-38) | `emit_tables.nika` (tables over `Ledger` and items); new `emit_needs.nika` (`Needs`, `abort_table`) | removed: `EmitHost::split_lifetimes`, `inline_written`; `abort_table` takes `SourceMap::rows` as a list | none (`Ledger` is Nikaia's, `ty.nika`) | ~470 |
| E12-2 | `emit/mod.rs`: `template_hole_tables` (1837-1886), `literal_expressions_bound`, `hole_as_read`, `interpolated_holes`, `literal_expressions`, `template_holes` (2159-2298), `fold_callees` (1888-1944), `built_at_build_time` (1102-1170); `emit/host.rs` `names_read` (269-294) | new `emit_holes.nika` | added: `hole(site, text) -> Expr?` with its error text, `cached_holes`, `cached_kept_calls` (the parser's caches stay Rust); removed: `EmitHost::names_read`, then `CheckHost::literal_holes`/`holes_read` call Nikaia | R-6 (the `contracts/sync.rs` walkers in Nikaia), or a probe that `emit_walk::visited_in_*` visits the same nodes as `sync::visit_*` | ~350 |
| E12-3 | `emit/seeds.rs`: `calls_of`, `impls_of`, `root_name`, `read_with`, `read_barriers`, `body`, `positions`, `shape`, `is_fixed`, `trait_maps` (52-500); `emit/host.rs` `seeds_begin`, `seeds_end`, `seeded` (296-388) | `emit_seeds.nika` (`EmitSeeds` holds the tables and the shapes); `Emitter.place`/`floor` move into `EmitState` | `contracts::maps::analyse_with` as a callback; removed: `EmitHost::seeds_begin`, `seeds_end`, `seeded`, `seeds_arity`, `place_*`, `set_floor` | none. (Corrected 2026-10-27: `contracts/maps.rs:55` `Shape` is not a twin of `ty.nika:104`, which is ADR-293's sequence shape; the maps `Shape` stays Rust with `contracts::maps`. Split: E12-3a the read walks, E12-3b the host's `seeds_*`/`place_*`/`floor`) | ~560 |
| E12-4 (4a done: `UserParallelism`, `ReentrancyCheck`, `RUST_TARGETS` in `emit_build.nika`; 4b built: `Target`, `BoundsChecks`, `OverflowChecks`; `Build` stays Rust: its default target is the compiler's own host triple, and `impl Default` for a `nikaia_std` type cannot be written in `nikaia`) | `emit/mod.rs` `Target`, `UserParallelism`, `ReentrancyCheck`, `Build` and their methods (52-430); `emit/rust_targets.rs` whole (327, a list) | new `emit_build.nika`, re-exported from `emit` as `check_state`'s `Finding` is | none | `bounds::BoundsChecks`/`OverflowChecks` move with it (two data-less enums); the lowering must derive `Copy`, `Default`, `PartialEq` for a data-less enum and a struct of them: probe first, else a compiler fix | ~700 |

**Stays Rust** (each until the named module moves, not by itself):

* `check`, `check_program`, `check_against`, `check_for_the_ledger`, `instantiations_in`,
  `propagation*`, `fallible_loops*`, `unrolling_report`, `claims_report`, and the `emit_program*`,
  `emit_std_*`, `emit_module_body_at`, `branch_starts_first`, `barrier_names` entry points: the
  driver's API; they own `Parsed` and the ledgers' lifetimes.
* `walked`'s `CheckWorld` (borrows `Parsed`, finds the unit by `ptr::eq`) and its run of Rust
  analyses (`prove`, `views`, `view_changes`, `keep`, `tether`, `traits`, `types`, `dsl::check`);
  `with_contracts`' run of `bounds`, `sharing`, `trust`, `keep`, `view_changes`, `propagation`:
  each line moves with its analysis (`contracts/`, research brief first).
* `dump_accepted`, `accepted_dump_dir`: process entry (environment, `current_exe`, file writes).
* `core.rs` (`Baton`, `Core`), `Checker`, `Emitter`, `emit/host.rs` `nikaia`/`crossed`/`stash`/
  `hosted`: the crossing itself, with the `anyhow` error that may not cross.
* `check/host.rs`, `emit/host.rs` methods not named above: each calls a Rust module
  (`build_time`, `comptime_run`, `text_tiers`, `contracts`, the parser's holes) and goes with it.
* `CodeOps` (leaks a `&'static str` for Rust callers), `MultiLock`, `HULLS` (read only by Rust
  tests: they go with #821's move of those tests), `From` for `Constant`.
* `OutText` (`take_lend`, `insert_*_brace`): until `std` has `String::insert_str` / `pop` (the
  owner's, undecided). `SourceMap`, `Lowered`: driver data the diagnostics read in Rust.
* One-line adapters over Nikaia (`visit_*`, `view_slots`, `borrowing_structs*`, `pausing_reach`,
  `format_of`, `template_holes_bound`, `dsl_name`, `escaped`, `nonnegative_*`, `ty`,
  `common_integer`, `written`, `char_literal`): they go when their last Rust caller goes.

**First slice to brief: C7-r1** (needs nothing). Known risk per slice (`docs/builder-facts.md`):

* C7-r1: the map loop over `((start, _), claim)` is a nested tuple pattern in a `for`; and
  `n if n <= 0 => continue` inside a `match` that makes a value: bind the count first.
* E12-1: `split_lifetimes` returns early with `?` from an `Option` function, splits on a char set
  (`split(['|', ','])`, two splits in Nikaia) and runs a union-find over a `mut Vec[i64]` with a
  nested `fn`; `Needs::join` takes `mut self`.
* E12-2: a different walker changes which calls are seen; the oracle is `lower-std`
  byte-identical, so a missed node shows as a lowered diff, not a test failure.
* E12-3: `Option<usize>` fields of `Shape` (`within`) cross as `-1`; the callback returning
  `Vec[(String, Shape)]` from Rust is the first of its kind in `emit_seeds.nika`.
* E12-4: a list literal of about 300 strings in a tools file (lowering cost, not seen yet);
  `Result<Target>` becomes `throws`, with `anyhow` text kept byte for byte in the adapter.
  Split 2026-10-27: **E12-4a** `UserParallelism`, `ReentrancyCheck`, `RUST_TARGETS` (no `Default`
  from the lowering needed: `Build` gets a hand `impl Default` while it is Rust); **E12-4b**
  `Target`, `Build`, `BoundsChecks`/`OverflowChecks`: first the lowering writes `impl Default` for a
  struct whose every field has a default (ADR-331 D5; 343 `Build::default()` callers), and
  `Target::Rust(&'static str)` keeps `Copy` (an index into the list, or what 4a exposes).
  4a built: the list is `emit_build_rust_targets() -> Vec[ref String]`, lowered to
  `Vec<&'static str>`, so `Target::Rust` keeps its `&'static str` with no index.
  4b built: a `ref String` payload lowers with a lifetime parameter on the enum and no `Copy`,
  so `Target::Rust(i64)` carries the index into that list; `host_target()` and
  `default_target()` stay Rust free functions in `emit`.

## Large files: project.rs and parser/mod.rs

Read at 6f5b548 (`project.rs` 4,851 lines, `parser/mod.rs` 4,351). Line ranges are
at that commit: re-grep the names before cutting. *Draft* is the share of code lines
`rs2nika --profile check` drafted in items without a TODO, over a scratch cut of
exactly the listed ranges.

**`parser/mod.rs` is mostly what stays Rust for now**, so the next file by size,
`comptime_run.rs` (1,699), is planned with it (`K` rows):

* The `grammar!` block (926-4351: 3,426 lines, 1,334 of them comments, 358 rules) is
  the parser. ADR-339 decided it moves as a Nikaia grammar, but #557's *To build*
  (`span(p)`, `here`, `same_line`, the `doc` rule) is not built, and ADR-339 D3
  forbids a grammar state while 71 actions draw node ids from `Trivia`
  (`_state.user().id()`) and 27 record spans (`.at()`). Decided (owner,
  2026-10-28, ADR-383): the grammar gives no ids; a numbering pass right after
  the parse, before `one_variant::expand`, numbers the tree in input order and
  writes the span table. Built by #903 (*To build* 1-4), in Rust's grammar first;
  the `R` rows wait for #903 and #557.
* R-4 (the old plan's non-grammar leaves) is built: `PrTrivia` is in
  `parser_leaves.nika`, `number_at` in `parse_numbers.nika`, the error texts in
  `parse_errors.nika`. What is left in 1-925 is winnow glue (`doc_here`,
  `same_line`, `new_line`, `float_end`, `number_lit`, `stops_at`, the `Trivia`
  adapter), `Parsed` (the hole and kept-call caches behind `Mutex`es, `keeping`,
  `growing`) and the entry points `parse_to_ast`, `parse_expression_from`. The pure
  remainder (`number_prefix`, `byte_of`, `char_of`, `failure_of`, `a_parse_finding`,
  `fits`, `flat_shape`, `kept_function_text`, about 110 lines, draft 90%) is below a
  slice: it goes with the grammar.

| Id | Rust (file:lines) | Target (`tools/`) | Host | Prerequisite | Rust lines | Draft |
| :--- | :--- | :--- | :--- | :--- | ---: | ---: |
| P2 | `project.rs`: the refusal summary of `check` (1989-2233: one count per NK family, `plural`, the `others` remainder), `let_refuted_claims_through`, `let_unproved_claims_through`, `lint_where_nothing_crosses` (2262-2323), the `--locked` texts of `write_proofs` and `write_ledger` (2385-2465) | new `project_check.nika`: `project_check_summary(findings: ref Vec[Finding], violations: i64) -> String`, the three filters over `mut Vec[Finding]`, the two texts | none: `check` keeps its calls (`check_against`, `foreign::check`, `sync::check`, the rendering) and passes `Finding`s (`findings.nika`) and the violation count | none | ~420 | 100%, built (`project_check.nika`) |
| K1 | `comptime_run.rs`: `VALUE_FN`, `sub_program`, `constants_as_lets`, `unit_without_its_constants`, `without_readers`, `reads_any`, `constants_of` (36-287); `a_function`, `literal_of`, `held_as`, `written_type` (914-1103) | new `comptime_tree.nika` (prefix `comptime_`: `a_function`, `literal_of`, `written_type` exist in `ty.nika`, `ledger_items.nika`, `views.nika`) | Nikaia takes the items and `ref InternerContext` (as `names_in_block` does) and returns the items to add; `Parsed::keeping` stays Rust, `known`/`typed` are callbacks `fn(String) -> BuildValue? sync`, `fn(String) -> Ty? sync` | none | ~440 | 48%, built (`comptime_tree.nika`) |
| P1 | `project.rs`: `refuted_claims`, `unproved_claims`, `optimizations`, `prove_through_loops`, `skip_free_at_exit`, `renamed`, `artifact`, `symbol_prefix` (113-314), `cpu_config` (435-449), `choices`' key and `panic_strategy` (564-638), `encoded_with_cpu` (4660-4686) | new `project_settings.nika`: `ProjectWords` (the strings of `Settings`); `Settings` keeps `build: Build` (Rust, E12-4) and holds the words | `cpu_key` stays Rust (`sysroot::native_cpu`) and is passed in; `Build` is filled in Rust from the Nikaia `BoundsChecks`/`OverflowChecks` | the converter drafts `choices`' 12-hole `format!` (TODO: *fewer arguments than holes*, comments between the arguments): fix it first or the draft is under 60% | ~320 | built (`project_settings.nika`; `symbol_prefix` left for #924) |
| P3a | `project.rs`: the test runner's discovery and verdicts: `Outcome`, `OutputTest`, `files_under`, `output_tests`, `copy_into`, `ProgramTest`, `program_tests`, `is_refusal`, `does_not_compile`, `named_by`, `difference`, `verdict` | `project_tests.nika` (`std::fs` reads, `walk`, `read_dir`; the three structs hold `Bytes`, re-exported in `project.rs`) | `project.rs` keeps thin `output_tests`/`program_tests` that `block_on` the Nikaia ones; `shown_path` moved after it (`fs::canonical` and `Path.strip_prefix` added to `std` for it) | none | ~330 | 24%, built (`project_tests.nika`) |
| P3b | `project.rs`: the run loop: `run_tests`, `outcome_of`, `run_program_test`, `bless`, `bless_program`, `Blessed`, `explain_output` (`shown_path` built) | `project_tests.nika` | added: `run_test_binary(binary, args, stdin, cwd) -> Outcome`, `run_program_test(test, setting) -> Outcome` (process spawns, `current_exe`, the `Sandbox` directory stay Rust); `Suite` holds the binaries as names | P3a (the `std::fs` prerequisite, `fs::canonical` and `Path.strip_prefix`, met) | ~390 | 19%, built (`project_tests.nika`: each test's outcome, bless, explanation and lines; the loop over tests and settings, the spawns, `Sandbox` and the failures' report stay Rust) |
| P4a | `project.rs`: `packages_of`, `renames_in`, `Member`, `dependencies_first`, `members_of`, `canonical`, `crate_root_of` | `project_members.nika` (`ProjectMembersManifest`, a view the Rust `Manifest` hands over through `ProjectMembersHost::manifest_at`; `ProjectMembersDependency` is `modules::Dependency`) | `project.rs` keeps `rooted` (`toml::Value`), `Member` (it holds the Rust `Manifest`), `manifest_view`, the host `Manifests` and the adapters that turn `ProjectMembersRefusal` back into its error | P3 (`fs::canonical`, `Path.strip_prefix`) | ~400 | built (`project_members.nika`) |
| P4b | `project.rs`: `Foreign`, `crates_beneath`, `Foreign::of`/`packages`, `sources_that_moved`, `proofs_digest`, `description_at` | `project_members.nika` | `Foreign::library` stays Rust as `foreign_library` (`contracts::std_library`); `project.rs` keeps `foreign_of` and the host `ForeignHost` (SHA-256, `Ledger::read_beside`/`absorb`/`empty`, `describe::crate_root`); `Foreign` is `ProjectMembersForeign` | P4a | ~260 | built (`project_members.nika`) |
| K2 | `comptime_run.rs`: `Computed`, `NOT_COMPUTED_YET`, `as_built` (440-485), `undeclared_callee`, `culprit` (761-861) | `comptime_tree.nika` | `Ledger` is Nikaia's; the `AHEAD` `thread_local` stays Rust with `worked_out_ahead` | K1; below 300 lines, so it goes with K1 if that builder has room, else with the move of `computed` | ~170 | 54%, built (`comptime_tree.nika`) |

**Stays Rust** (each until the named module moves, not by itself):

* `project.rs` `lower`, `lower_reading` (1040-1590), `read_file`, `packages_of_file`,
  `reads_for*`: the driver; they own `Program`/`Parsed`, the build cache, `cexport` and the
  merge of published contracts into the ledger.
* `project_for_file` (1136-1212), `Project` with `open`, the directory paths, `drive`,
  `drive_to` (3009-3480), `report`, `record_resolved_dependencies`, `program_name`,
  `root_tag`, `own_copy`, `libraries_in`, `executable_in`: Cargo invocation, the file
  system, `CARGO_TARGET_DIR`, the lock file.
* `cargo_member`, `runs_without_cargo`, `cargo_workspace`, `crate_named`, `codegen`,
  `runtime_dependencies*`, `runtime_features` (2815-2992, 4438-4518): they build
  `orchestrator`'s `CargoProject`/`Workspace` out of `toml::Value`s; they go when
  `orchestrator` does.
* `wrapper_main`, `switches_beside`, `trace` (4519-4670), `Settings::from_env`,
  `as_env`, `cpu_rustflags`, the `*_VAR` names: the `rustc` wrapper's process entry
  (environment, `args_os`).
* `check`'s calls into the analyses (1894-1988), `explain` (2524-2657, one `print!` per
  Rust report), `Around`, `Lowered`, the `WARNED`/`SHOWN_AS` `thread_local`s: each goes
  with its analysis or with the driver.
* `Sandbox`, `outcome_of`, `run_program_test`'s spawns: P3's callbacks.
* `mod tests` (4687-4851): Rust tests; they move to Nikaia with #821, not with a slice.
* `comptime_run.rs` `compute`, `computed`, `worked_out_ahead` (486-760), `lowered_units`,
  `without_foreign`, `dependency_modules`, `by_package` (288-439), `with_driver`: they
  call the emitter, `trust::analyse_package`, `Ledger::infer_package` and the workshop.
* `comptime_run.rs` `Bundle`, `bundle*`, `run_fingerprint`, `stamp_of`, `run`, `waited`,
  `cargo`, `rustc`, `toolchain`, `target_libdir`, `write_if_changed`, `unique_beside`
  (1131-1699): `rustc` invocation, child processes, dynamic libraries on disk.
* `parser/mod.rs`: the grammar, `Trivia`, `Parsed` and the entry points, as said above.

Of `project.rs` about 2,120 lines are in P1-P4; of `comptime_run.rs` about 610 in K1-K2.

**First two slices to brief: P2, then K1** (different files, one after the other; both
add a row to `scripts/self_hosting.py` and a module to `tools/lowered.rs`, so not side
by side). Known risk of each:

* P2: the summary text is byte for byte in many `.stderr` of `tests/compiler` and the
  `it` tests; the filters change `severity` and push to `notes` in place
  (`iter_mut().filter(..)`), which is a `mut Vec[Finding]` with an index loop, and the
  `count`/`plural` closures become functions.
* K1: the draft is 48%, under the 60% rule: three of its four TODO items are
  *`collect` into an `Option`* (the fourth is a `Some(` in a struct pattern, #886); the
  converter should write that loop itself before K1 is briefed, else it is hand work.

## Large files 2: sysroot.rs, modules.rs, emit/mod.rs

Read at bcb60d1 (`sysroot.rs` 1,575 lines, `modules.rs` 1,398, `emit/mod.rs` 1,411,
`contracts/mod.rs` 1,326). Line ranges are at that commit: re-grep the names before
cutting. *Draft* is as in the section above: the share of code lines (not comments, not
blank) that `rs2nika --profile check --file` drafted in items without a TODO, over a
scratch cut of exactly the listed ranges (a code block inside a function wrapped in a
scratch `fn` of its free variables).

**`emit/mod.rs` is mostly what stays Rust**, so `contracts/mod.rs` is planned with it
(`C` rows). Of its 844 code lines, `with_contracts` (710-1033) is the run of the Rust
analyses (`propagation`, `bounds::proven`, `sharing`, `trust`, `views::carried`,
`keep`, `view_changes`) that fills `EmitState`; the rest are the driver's entry points
(`emit_program*`, `emit_std_*`, `emit_module_body_at`, `branch_starts_first`,
`barrier_names`), `Build` with `host_target()`/`default_target()` (E12-4), `Emitter`
and its crossing (`unit`, `ty`, `seeded`), `OutText`, and one-line adapters over
Nikaia. What is pure is `SourceMap` and the table expressions of the `EmitState`
literal: about 160 lines (E13-1), below a slice.

| Id | Rust (file:lines) | Target (`tools/`) | Host | Prerequisite | Rust lines | Draft |
| :--- | :--- | :--- | :--- | :--- | ---: | ---: |
| M1 | `modules.rs`: `Program::of` (675-835: the dependency ledgers merged into `library`, `contracts`, `as_its_own`), `shown_paths`, `could_promise`, `lost_sync` (865-936), `contracts_as_promised`, `package_names`, `is_single_file` (836-966); `declared_once`, `declared_names` (299-336), `groups` (411-426), `as_promised` (497-529) | `modules.nika` (exists: `could_promise`, `pause_path`, `lost_sync_unplaced` are there) | added: `package_ledger(group) -> Ledger` (contract inference and the shipped ledger stay Rust), `render_finding(finding, path, source) -> String` (`diagnostics`); the own groups' loop stays Rust (it holds `Vec<Checked>`), Nikaia merges the ledgers it returns; `Program`/`Unit` stay Rust structs (they hold `Parsed`) and pass `package`, `renames`, paths | none | ~380 | 71%, built (`modules.nika`) |
| S2 | `sysroot.rs`: `Sysroot` paths, `std_modules`, `tool_modules`, `nika_in` (88-199), `cpu_levels` and the parsing half of `native_cpu` (223-274), `tools_sources` (459-472), `Package` with `committed`, `inlined`, `contains`, `drift`, `stale`, `pack`, `unpack` (777-878), `LoweredBefore` `key`, `get`, `put` (1320-1418), the `interface`/`file_key` keys of `lower_tools_files_cached` (599-633) | new `sysroot_cache.nika` (`std::fs` reads, writes, `read_dir`, as `sources.nika`) | added: `sha256_hex(ref Vec[u8]) -> String` (`std` has no SHA-256), `rustc_cpus(triple, args) -> String throws`, `use_entry(dir)` and `user_cache_dir()` (`orchestrator::cache`), `unique_beside(path)` (`comptime_run`); the `ASKED` memo of `native_cpu` stays Rust around the Nikaia parse | P3's probe that `std::fs` in `tools/*.nika` reads and writes bytes (build it here if P3 has not) | ~445 | 90%, built (`sysroot_cache.nika`) |
| M2 | `modules.rs`: `test_function`, `is_a_test_function` (607-621), `with_tests_as_functions` (1180-1306), `names_in` (1312-1375) | `modules.nika` (`test_dispatcher`, `test_function` are there) | added: `parse_to_ast(text) -> Parsed` for the entry re-parsed with its dispatcher; Nikaia rewrites `Item::Test` into `Item::Fn` over `mut Vec[Spanned[Item]]` and takes `emit_walk::visited_in_*` for `names_in` | M1; a name for `__nikaia_test_K` interned from Nikaia (today `parser::parse_expression`): probe that `InternerContext` can intern from a tools file, else a callback. Below 300 lines: it goes with M1 if that builder has room | ~205 | 36%, built (`modules.nika`) |
| S1 | `sysroot.rs`: `Codegen` with `render` (293-327), `every_refusal` (404-439), `without_bodies` (487-538), `shared`, `raw_string` (887-1044), `facade` (1180-1229), the assembly of `lowered.rs` in `lower_tools_files_cached` (698-771: imports hoisted, `mod X_lowered`, the facades, the header) | new `sysroot_package.nika` (text only) | added: `render_finding` (as M1); `Codegen::new` stays Rust (`toml::Value`) and passes the rendered table; `escaped` is `emit_misc::is_reserved` | the converter first: `.get(range)` on a `String` (NK1171, twice in `shared`), `else if let`, and `facade`'s or-pattern of struct patterns with `Some(name)` (#886), where the draft also ran `raw_string` and `facade` into one line | ~405 | 44% |
| C1 | `contracts/mod.rs`: the declared-items loop of `infer_package_noted` (391-642: functions, impls, externs, traits, enums, structs and their literal defaults, grammars), the `stored` fold (678-689), `lookup`, `candidates` (791-844), `Computed`, `computed_defaults` (1070-1108), the walk of `attribute_values` (1109-1191), `expression_ensures` (1301-1326) | `ledger_items.nika` (the loop; `declared_*` are in `declared.nika`), `ledger_defaults.nika` (the attribute walk) | added: `computed_literal(unit, name, expr, ty) -> String?` (`build_time`, `comptime_run`), `reads_back` (`prove`), `mark_text` until `check/attributes.rs` moves (C7-r1); `Computed` holds a unit index and the item's position instead of `&'a Expr` | the converter first: `.entry(k).or_default().insert(v)` (one TODO line in the loop, one in the fold) | ~465 | 45% |
| E13-1 | `emit/mod.rs`: `SourceMap` (229-314); the table expressions of the `EmitState` literal in `with_contracts` (`order_proofs` 799-815, `carries_input`, `hole_wraps`, `believed_arithmetic`, `proven_*` 945-970) | `emit_out.nika` (`EmitMapEntry` is there), `emit_tables.nika` | `Lowered.map` becomes the Nikaia `SourceMap`; `modules.rs` and the diagnostics call its `locate`, `placed`, `extend`, `rows` | below a slice: not briefed; it goes with the move of `with_contracts`' analyses, or with C1 if that builder has room | ~160 | 100% (`SourceMap`) |

**Stays Rust** (each until the named module moves, not by itself):

* `sysroot.rs` `lower_std_module`, `lower_std`, `lower_tools*`, the run of
  `lower_tools_files_cached` (544-697: parse, `declared_once`, the described ledger,
  `infer_package_checked`, `trust`, `check_against`, `emit_std_items_against_at`):
  the driver; they own `Parsed` and the ledgers.
* `in_parallel` (scoped threads with a 256 MB stack, `AtomicUsize`), `Phases`
  (`Instant`, `NIKAIA_LOWER_TIMES`, `eprintln!` of durations): threads and clocks.
* `Sysroot::resolve` (`env!("CARGO_MANIFEST_DIR")`), `rlib_cache`
  (`env!("NIKAIA_RUSTC_VERSION")`, `orchestrator::cache::Key`), `COMPILER_IDENTITY`
  (`env!`), `rustc_cpus` (a `rustc` process), `tools_build` (`Build`, E12-4),
  `impl Display for Package`, `mod tests` (Rust tests, #821).
* `modules.rs` `collect*`, `package_at`, `collect_one`, `read_unit`, `check_imports`,
  `one_namespace`: the file system with the parser and `diagnostics::refuse`, `anyhow`
  context; `Unit`, `Program`, `Dependency`, `TestCase` (hold `Parsed`, `PathBuf`).
* `modules.rs` `package_ledger*`, `shipped_ledger`, `sources_of` (contract inference
  returning `Checked`, SHA-256), `emit`, `emit_reading` (999-1188: `proofs::for_unit`,
  `emit_module_body_at`, `keyed_functions`, `barrier_names`, the abort table),
  `foreign_beside`, `described_beside` (`Manifest`, `project::Foreign`).
* `emit/mod.rs`: everything but E13-1, as said above.
* `contracts/mod.rs` the trait impls (`LedgerOps`, `SignatureOps`, `VariantOps`,
  `SyncOps`: one-line adapters over `ledger_ops`, `ledger_text`, `ledger`), the run
  of analyses in `infer_package_noted` (643-768: `check_for_the_ledger`, `sync`,
  `throws`, `touch`, `keeps` to a fixed point, `sharing`, `maps`, `locks`, `tether`),
  `evaluate_defaults`, `computed_literal` (`build_time`, `comptime_run`), `std_ledger`
  (`OnceLock`), `toolchain` (`env!`), `read_beside` (until `std::fs` is probed, then
  with S2's pattern).

Of `sysroot.rs` about 850 lines are in S1-S2, of `modules.rs` about 585 in M1-M2, of
`contracts/mod.rs` about 465 in C1, of `emit/mod.rs` about 160 in E13-1.

**First two slices to brief: M1, then S2** (different files, both at or above 60%;
S2 adds `sysroot_cache.nika` and its row in `scripts/self_hosting.py`, M1 adds to an
existing file, so they may run side by side). Known risk of each:

* M1: `library` grows group by group (a dependency's ledger is absorbed before the
  next is inferred against it), so the callback is called inside the Nikaia loop,
  not before it; and the draft marks every `Ledger` parameter `ADAPTER: bleibt in
  Rust` although `Ledger` is `ty.nika`'s: the converter's table should learn that
  before the brief, or the builder deletes the markers by hand.
* S2: the keys hash Rust `Debug` text (`{own:?}`, `{build:?}{provenance:?}`); the
  Nikaia text differs, so every cache key changes once (one cold `lower-std`, which is
  accepted) and must then be stable from run to run: a test lowers twice and expects
  every file from the cache. The draft wrote `interface` as one line of closures over
  a `mut Vec[u8]` (`part`): a nested `fn` there.
* S1 and C1 wait for the converter fixes named in their rows (both drafts are under
  60%: the rule stops the pipeline).

## Large files 3: lane 1's drivers

Read at a4b33a0 (`project.rs` 3,565 lines, `modules.rs` 1,088, `comptime_run.rs`
1,269, `grammar_run.rs` 522, `manifest.rs` 599, `build_time.rs` 387: 7,430). The
earlier tables' *Stays Rust* drivers are this week's work (`docs/bootstrap-week.md`
section 1): the question per row is which part is Nikaia over host callbacks and which
adapter lines it leaves. Line ranges are at a4b33a0: re-grep the names before cutting.
*Draft* is as above, `rs2nika --profile check-free --prefix X_ --file` over a scratch
cut of exactly the `Cut:` ranges (methods wrapped in `impl T { }`): the share of code
lines in items without a TODO. *Rust lines* count comments; about 45% of them are.

**How `Parsed` crosses, in every row** (the M2/K1 pattern, not a new one): `Parsed`
stays a Rust struct until lane 2's parser rows (#557 step 3, #903) make it Nikaia's.
Nikaia gets a file as items + `ref InternerContext` (or `ParsedView`), and where a row
makes or keeps a `Parsed` (`parse_to_ast`, `keeping`, `growing`) the host does it and
hands back an index into a Rust arena. So `Unit`, `Program` and `Project` stay Rust
structs (their fields are read about 150, 50 and 86 times, in `it/` too) and their
decisions move as functions over the fields they pass. A caller in a lane 2 file
(`check/host.rs`, `contracts/mod.rs`, `emit/mod.rs`, `emit/seeds.rs`) keeps a one-line
Rust forward.

**`std` against Part III 17** (each row names what it needs): `fs` reads, writes,
`read_dir` (sorted names), `canonical`, `create_dir` (recursive), `copy`, `rename`,
`remove`, `metadata` and `process::run` (`dir:`, `env:`, `within:`, both streams
captured) are there. Missing: environment variables and `temp_dir` (#922 step 2),
SHA-256 (#922 step 1), a file's modification time (#922 step 4), inherited streams and a
stdin text (#920 step 2), `exec` (none planned), `hard_link`, `current_exe`, a JSON
reader (none: cargo's messages stay with `diagnostics`' `serde_json`). Until an entry
exists, the row takes a host callback and the issue names it.

| Id | Rust (file:lines) | Target (`tools/`) | Host | Prerequisite | Rust lines | Draft |
| :--- | :--- | :--- | :--- | :--- | ---: | ---: |
| P5a | `project.rs`: `Settings`' `resolve`, `refutations`, `choose_cpu`, `cpu_key`, `cpu_config`, `cpu_rustflags`, `optimize`, `as_env`, `from_env`, `choices`, `panic_strategy` (185-460); `Project`'s `open`, `entry`, `build_dir`, `own_dir`, `gen_dir`, `switches_path`, `ledger_path`, `packages`, `members` (1782-1929); `program_name`, `root_tag`, `own_copy` (2740-2780); `switches_beside`, `trace` (3354-3392) | `project_settings.nika` (`ProjectWords` is there) | `Settings` keeps `build: Build` (`emit/mod.rs`, lane 2): Nikaia returns the words and the four optimisation values, Rust calls `Build::parse`; added `env_var(name) -> String?`, `native_cpu(triple)`, `sha256_hex`, `hard_link(from, to) -> bool`; `cpu_levels` is `sysroot_cpu_levels` | #922 steps 1-2 or the callbacks (named in #922) | ~505 | built in part (`project_settings.nika`, host `ProjectSettingsHost`: `env_var`, `native_cpu`); still Rust: `root_tag`, `program_name`, `own_copy` (a `String` from a host method, then viewed, lowers as `&'r str` of the parameters), `trace` (NK2605 though the file is closed by hand), `optimize` and `open`'s glue (they write `Build`, `Manifest`, `Layout`) |
| P5b | `project.rs`: `lower`, `reads_for`, `reads_for_file`, `read_file`, `packages_of_file`, `project_for_file`, `lower_reading` (663-1214), `foreign_library` (1271-1299) | new `project_lower.nika` (prefix `project_lower_`) | added: `program_read(input, packages, tests, in_project) -> i64`, `check_unit(program, at) -> String throws`, `emit(program) -> ProjectLowerEmitted`, `cache_lookup`/`cache_record` (`orchestrator::Cache` until Sunday's O rows), `cexport(program)`, `with_book`; the published-contract merge into `Ledger` (`ty.nika`) and the notes move | M3b (else the host carries `Program`'s eight reads too); converter: `match` on `Ok`/`Err` off R160's shape (2), `Option::map`, `.enumerate()` | ~580 | 36%, built: `lower_reading`'s steps (`project_lower_reading` over `ProjectLowerHost`, the published-contract merge `project_lower_published`); narrowed: `lower`, `reads_for`, `reads_for_file`, `read_file`, `packages_of_file` are adapters already, `project_for_file` waits for `is_ascii_alphanumeric` (#924), `foreign_library` for `std_library`; stays Rust: `Lowered`, the choices before the read, `Program`, `Cache`/`Artifacts`, the check and the emitter, `Book`/`with_book`, `cexport` and the export assembly after the call |
| P5c | `project.rs`: `drive`, `drive_to` (2110-2584), `libraries_in`, `executable_in` (2781-2844), `Project::report` (from P5d: it reads and emits a whole `Program`, which P5b's host gives) | new `project_drive.nika` | added: `lower(member) -> ProjectLowerOut` (P5b), `cargo_workspace` and `Cargo::messages`/`run` (orchestrator until O rows), `run_fast_compile`, `run_directly` (exec), `artifacts_in(messages)` (`serde_json`), `toolchain_*` checks; `libraries::*` are Nikaia (`libraries.nika`) | P5a, P5b; #920 step 2 for `cargo build`'s passed-through stderr; converter: `drive_to` is one item, so one TODO (a `Some(` in a tuple pattern, `Ok`/`Err` shape, `.cloned()`, `Option::map`) zeroes it | ~540 | 2% |
| P5d | `project.rs`: `Around`, `check`, `newly`, `keeps_that_moved`, `write_proofs`, `write_ledger`, `changed_lines` (1301-1594), `Explain`, `explain` (1596-1780), `report` (2586-2691) | `project_check.nika` (the summary and filters are there, P2) | added: `check_against(unit) -> Vec[Finding]` (`check/`, lane 2), `foreign_check`, `sync_violations`, `render_finding`/`render_sync_violation`/`render` (`diagnostics`), `explain_report(kind, unit) -> String` (one per analysis: `order`, `sharing`, `tether`, `claims`, `bounds_report`, `trust`, `unrolling`), `translate_units(messages)`; `Book` writing stays behind `write_book` | P2 (built); converter: a write through a match binding (#576), `.cloned()` | ~585 | built (`project_check.nika`: `project_check_check`, `project_check_gained`, `project_check_keeps_that_moved`, `project_check_explain`; hosts `ProjectCheckHost`, `ProjectCheckExplainHost`); still Rust: `Around`, `Explain` and `asked`, the walks and reports behind the hosts, `write_proofs` (`Book`), `write_ledger`, `changed_lines`, `foreign_library`; `report` moved to P5c (it reads and emits a whole `Program`); draft kept 12 of 79 lines |
| P5e | `project.rs`: `cargo_member`, `runs_without_cargo`, `cargo_workspace`, `crate_named`, `codegen` (1931-2108), `record_resolved_dependencies` (2692-2737), `Suite`, `run_tests` (2906-3053), `runtime_dependencies_for`, `runtime_features`, `runtime_dependencies`, `wrapper_main` (3156-3352) | `project_drive.nika` | `CargoProject`, `Workspace`, `Invocation`, `Lockfile` are Nikaia after the O rows; a TOML value crosses as its text (MF); `env!("NIKAIA_RUNTIME_*")` passed in; the test spawns stay (`outcome_of`, `run_program_test`, `Sandbox`) | the orchestrator's O rows (#923, Sunday), MF, #922 step 2 (`args`, environment); converter: a `Some(` in a struct pattern (#886) | ~570 | 63% |
| M3a | `modules.rs`: `Unit::qualify`, `collect`, `collect_with`, `package_at`, `collect_one`, `Standing`, `read_unit`, `one_namespace`, `declared_once`, `declared_names`, `check_imports`, `sources_of`, `groups`, `packages_of`, `package_ledger*`, `shipped_ledger` (37-480), `foreign_beside`, `described_beside` (1066-1088) | `modules.nika` (`declared_once`, `a_use_refused`, `groups` are there) | `std::fs` reads and `read_dir`; added `parse_unit(path, source, package) -> i64 throws` (`parse_to_ast`, the refusal rendered by `diagnostics`; the `Parsed` kept in Rust's arena), `infer_package(units) -> Ledger` (`contracts`, lane 2), `read_beside`; Rust builds the `Unit`s from the indices | none; #922 step 1 for `sources_of`' digest (else `sha256_hex` callback) | ~470 | 95%, built (`modules.nika`) |
| M3b | `modules.rs`: `Program`, `TestCase`, `read*`, `of`, `shown_paths` .. `sources`, `emit`, `emit_reading`, `with_tests_as_functions`, `std_ledger`, `names_in` (482-1065) | `modules.nika` | `Program` stays a Rust struct (holds `Vec<Unit>`, `Checked`, `CheckFacts`); added `across_files(group)` (`type_rules`), `analyse_package*` (`trust`), `keyed_functions`, `barrier_names`, `needs_of`, `emit_module_body_at` (emitter, lane 2), `for_unit` (`proofs`) | M3a | ~585 | 100%, built: `of`, `emit_reading` (`modules_program_of`, `modules_emit_reading` over `ModulesProgramHost`, `ModulesEmitHost`); stays Rust: `Program`, `TestCase`, `read*`, `with_tests_as_functions`, the test-block `retain` and `across_files` (before the call), the inference, trust, emitter and source map behind the hosts |
| MF | `manifest.rs`: `Manifest` and its readers, `table_of`, `libraries`, `codegen`, `dependencies` (1-358), `mod tests` (359-599: 16 tests, each a `tests/compiler` program or a `test` block of `manifest.nika`); `project.rs` `rooted` (637-661) | `manifest.nika` (the `decide` functions are there): `ManifestFile` with `build`, `package`, `codegen` and `ManifestDependency` holding TOML values as their text | Rust keeps `Manifest::read`/`parse` turning `toml::Value` into `ManifestFile` (`as_str`, `as_integer`, `as_table` read there) and parses a value's text back where `orchestrator` needs a `toml::Value` (until the O rows). **Built** (MF): `manifest_parse` takes the document as `ManifestItem`s (section, name, key, is-table, text, `as_str`, `as_integer`, `machine` = `Target::parse`'s answer) and builds `ManifestFile` with every refusal; still Rust: `parse`'s walk (`items_of`), `Dependency` (derived once, `type` removed from a Rust crate's table), `codegen_for`'s `toml::Value` rebuilt from the text, `setting` (a borrowed answer), `find`/`read`, `rooted`, the 15 `#[test]`s | none | ~600 | 100%, built; draft kept 7% |
| K3a | `comptime_run.rs`: `lowered_units`, `without_foreign`, `dependency_modules`, `by_package` (119-271), `compute`, `computed`, `worked_out_ahead` (284-544), `undeclared_callee`, `culprit` (565-595) | `comptime_tree.nika` | `Parsed` arena: `sub_program`, `unit_without_its_constants`, `keeping` give indices; added `infer_package(indices, library)`, `emit_body(index, ..)`, `emit_program`, `dumper`, `run_comptime(program, name) -> ComptimeRan` (the `Wall`s as a Nikaia enum), `evaluate_folded` (`BuildTime`); `AHEAD` stays a Rust `thread_local` behind `enter(bound) -> bool`/`leave(bound)` | K1, K2 (built); converter: a `Some(` in a struct pattern (#886), `Ok`/`Err` shape | ~455 | 58% |
| K3b | `comptime_run.rs`: `Bundle`, `Flavour`, `bundle`, `run_bundle`, `run_fingerprint`, `read_record`, `bundle_text`, `bundle_dir`, `stamp_of`, `bundle_of`, `build_for_cpu`, `run`, `waited`, `cargo`, `rustc`, `toolchain`, `target_libdir`, `write_if_changed`, `unique_beside` (720-1269) | new `comptime_bundle.nika` | `process::run` for `cargo rustc`, `rustc`, `rustc -vV`, `--print target-libdir` (captured, `dir:`); `artifacts_in` as P5c (`serde_json`); the `OnceLock`, `CPU_LEVEL`/`NOT_BUILT` statics and `unique_beside`'s counter stay Rust | #920 (the child's progress note while it runs: `waited`), #922 steps 1, 2, 4 (digest, `CARGO`/`RUSTC`/`LD_LIBRARY_PATH`, `modified`); converter: `static` behind a lock, `.as_mut()`, `Result` read as a value | ~550 | 32% |
| BT | `build_time.rs` (1-387: `Refusal`, `BuildTime`, `Host` and its `BtHost` impl, `rule_of`, `char_of`, the escape forwards, `may_not_run`, `is_the_builds_own`); `comptime_run.rs` the defaults (597-718: `DEFAULTS`, `KNOWN`, `defaults_compiled_in`, `defaults_known_from`, `known_default`, `defaults_workshop`, `with_driver`, `Bounds`) | `build_eval.nika` (`bt_evaluate`, `BtHost` are there), `comptime_tree.nika` | `Refusal` becomes `BtRefusal`'s owner (the `Host(slot)` indirection goes); `BtHost`'s `constant`, `variant_of`, `rule_of` move to Nikaia over items; the `thread_local`s stay with one-line setters | K3a; converter: `thread_local!` cells, `match` on a kept `Result` | ~510 | 76% |
| GR | `grammar_run.rs` (1-522: `Wall`, `Ask`, `Workshop`, `compile_and_run`, `cargo`, `write_if_changed`, `shared_target`, `manifest`, `driver`, `dumper`, `shapes`, `decode`) | new `grammar_run.nika` (`grammar_decode.nika` has `gr_manifest`, `gr_main`, `gr_decode`) | `process::run` for `cargo build` and the parser (captured); added as K3a: `emit_program`, `emit_body`, `infer_package`, `analyse`; `shapes` walks items in Nikaia; `Workshop`'s `Mutex`/`OnceLock` stay Rust | K3a (`without_foreign`, `dependency_modules`, `by_package`), #922 steps 1-2; converter: `.entry(..)` (2), `Option::map` | ~520 | 51% |

**Cut/Convert** (ready to paste into a brief; ranges at a4b33a0):

* P5a: `Cut: crates/nikaia/src/project.rs:185-460`, `:1782-1929`, `:2740-2780`, `:3354-3392`; `Convert: rs2nika --profile check-free --prefix project_settings_ --file <cut>.rs`
* P5b: `Cut: crates/nikaia/src/project.rs:663-1214`, `:1271-1299`; `Convert: ... --prefix project_lower_`
* P5c: `Cut: crates/nikaia/src/project.rs:2110-2584`, `:2781-2844`; `Convert: ... --prefix project_drive_`
* P5d: `Cut: crates/nikaia/src/project.rs:1301-1594`, `:1596-1780`, `:2586-2691`; `Convert: ... --prefix project_check_`
* P5e: `Cut: crates/nikaia/src/project.rs:1931-2108`, `:2692-2737`, `:2906-3053`, `:3156-3352`; `Convert: ... --prefix project_drive_`
* M3a: `Cut: crates/nikaia/src/modules.rs:37-480`, `:1066-1088`; `Convert: ... --prefix modules_`
* M3b: `Cut: crates/nikaia/src/modules.rs:482-1065`; `Convert: ... --prefix modules_`
* MF: `Cut: crates/nikaia/src/manifest.rs:1-599`, `crates/nikaia/src/project.rs:637-661`; `Convert: ... --prefix manifest_` (draft over 1-358 and 637-661; the tests are rewritten, not converted)
* K3a: `Cut: crates/nikaia/src/comptime_run.rs:119-544`, `:565-595`; `Convert: ... --prefix comptime_`
* K3b: `Cut: crates/nikaia/src/comptime_run.rs:720-1269`; `Convert: ... --prefix comptime_bundle_`
* BT: `Cut: crates/nikaia/src/build_time.rs:1-387`, `crates/nikaia/src/comptime_run.rs:597-718`; `Convert: ... --prefix build_time_`
* GR: `Cut: crates/nikaia/src/grammar_run.rs:1-522`; `Convert: ... --prefix grammar_run_`

No row splits a function; M3b and MF are the two at about 600. Callers (`where.nika`):
`lower_reading` (`main.rs`, `project.rs`), `project_for_file`, `read_file`,
`reads_for_file`, `wrapper_main`, `run_tests` (`main.rs`), `drive_to`, `explain`,
`write_ledger` (`main.rs`, `project.rs`), `read_with` (`emit/mod.rs`, `emit/seeds.rs`,
`project.rs`), `read_for_tests` (`main.rs`, `project.rs`), `compute` (`check/host.rs`,
`contracts/mod.rs`), `known_default`, `defaults_workshop` (`contracts/mod.rs`),
`defaults_compiled_in` (`emit/mod.rs`, `project.rs`, `specbook.rs`), `may_not_run`
(`check/host.rs`), `run_bundle` (`run_fast.rs`), `runtime_dependencies_for`
(`comptime_run.rs`, `grammar_run.rs`), `decode` (`build_time.rs`, `comptime_run.rs`).
The `it/` tests read `Unit` fields in about 25 files, `Program`'s in 7, `Lowered`'s
in 6, `Manifest::` in 3: none of these structs is reshaped, so none breaks.

**Stays Rust** (the adapters these rows leave; each with why):

* `project.rs` the `Settings`, `Project`, `Lowered`, `Member` struct shells and their
  one-line forwards (~120): `Build` is `emit/mod.rs`'s and the fields are read in
  `main.rs` and `it/`; they go when `main.rs`' calls do.
* `project.rs` `WARNED`, `SHOWN_AS` (8): `thread_local`s, with one-line setters.
* `project.rs` `Sandbox`, `outcome_of`, `run_program_test`'s spawn (~110): stdin text
  and a test directory, until #920 step 2 (the bootstrap list's 250).
* `project.rs` `run_directly` (~25): `exec` replaces the process (#281); no `std`
  entry is planned.
* `project.rs`/`comptime_run.rs` `artifacts_in` (~35): cargo's JSON messages, beside
  `diagnostics`' `serde_json`.
* `project.rs` `foreign_of`, `ForeignHost` (~45): SHA-256 until #922 step 1, then the
  `Ledger` callbacks only.
* `project.rs` `mod tests` (171), `manifest.rs` `mod tests` (241): #821, with the code
  they test (MF converts its own).
* `modules.rs` `Unit`, `Program` (~60) and the `Parsed` arena (~25): until `Parsed` is
  Nikaia's.
* `manifest.rs` `Manifest::read`/`parse` over `toml::Value`, `rooted` (~150): `toml`.
* `comptime_run.rs` `AHEAD`, `DEFAULTS`, `KNOWN` (~30), `CPU_LEVEL`, `NOT_BUILT`,
  `toolchain`/`target_libdir`'s `OnceLock` (~25), `LIBRARY_PATH`'s `cfg` (6),
  `unique_beside`'s counter (8), `waited` (~40, until #920), `env!("NIKAIA_COMPILER")`.
* `grammar_run.rs` `Workshop`'s `Mutex`/`OnceLock` (~30), `shared_target`'s `env!` (5).
* `build_time.rs` the `BtHost` impl's reads of `Parsed` (`text`, `known`, `read`,
  `run_grammar`: ~50): until `Parsed` is Nikaia's.

So the six files leave about 1,000 adapter lines after these rows, about 600 of them
waiting for #920, #922, `Parsed` or the O rows; over the bootstrap list's 250 + 350 +
190 by the struct shells, which go with `main.rs`.

**Order:** P5a, M3a, P5d, MF, M3b, P5b, K3a, BT, GR, K3b, P5c, P5e (P5b after M3b so
its host does not carry `Program`; P5c after P5b; P5e after the O rows).
**First two rows to brief: P5a, then M3a** (different files, both drafts over 85%).
Known risk of each:

* P5a: `Settings` holds the Rust `Build` and `Project`'s fields are read 86 times
  (`project.rs` 56, `main.rs` 18, `sysroot.rs` 8): both stay Rust structs, so the
  Nikaia functions take and return the words, not the structs, and the draft's
  `impl Settings`/`impl Project` (which it wrote as Nikaia `impl`s) become free
  functions over `ProjectWords` and a `ProjectPaths { root, variant }`; `as_env`,
  `from_env`, `cpu_rustflags` read the environment before #922 step 2, through a
  callback.
* M3a: `Unit` holds `Parsed` and is read about 150 times in about 30 files (25 of
  them `it/`), so it is not reshaped: Nikaia returns the file list, the packages and
  the refusals, and Rust builds the `Unit`s from the arena indices. Every refusal text
  and its path display must stay byte for byte (`tests/compiler` `.stderr`), and
  `read_dir`'s order is Nikaia's sorted names where Rust sorted `PathBuf`s.

Rows under 60% (P5b, P5c, P5d, K3a, K3b, GR) are recorded, not stopped (bootstrap
L1); the converter repairs that lift most of them: `match` on `Ok`/`Err` off R160's
shape (P5b, P5c, K3a, K3b), `Option::map`/`and_then` (P5b, P5c, P5e, GR), `.cloned()`
of an iterator (P5c, P5d), a `Some(` in a tuple or struct pattern (#886: P5c, P5e,
K3a), `.entry(..)` (GR).
