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
| U-d0 (done, 0.0.959) | `check_narrow.nika` | `narrow.rs` whole (ADR-350, #621 done): `proved_names`, `proofs_for`, `proved`, `reads_of_the_maybe`, `read_as_proved`, `a_proof_taken_back`; the four `Checker` fields become `CheckState.narrowing`; 6 host methods go | 257 |
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
