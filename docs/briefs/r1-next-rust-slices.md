# R-1: order of the remaining Rust (contracts/, parser/, type_rules, modules, fold_bound, comptime_run)

Method: `grep` of `fn|struct|enum|impl` with line numbers and of `tools::` per file (17,356 lines in all);
spans approximate. Existing `.nika` names taken from `crates/nikaia-std/src/tools/*.nika`.

## State per file (lines; `tools::` refs = how much already runs in Nikaia)
| File | Lines | tools refs | Verdict |
| :--- | ---: | :--- | :--- |
| contracts/maps.rs | 1213 | 0 (none!) | all pure analysis: the biggest unmoved block of contracts/ |
| contracts/mod.rs | 1439 | 26 (ledger*, declared, ty) | `Ledger` impl 206-905 is the adapter (stays); free fns 931-1439 (~500) movable |
| contracts/send.rs | 652 | 6 | Crossing walk 126-210 moved to `check_send`/`threads`; rest ~450 walk |
| contracts/sync.rs | 556 | 5 (tools::sync, 484 lines) | adapter plus `visit_*` walkers (used by fold_bound) |
| contracts/keep.rs 449, keeps.rs 414, touch.rs 381, tether.rs 355, ty.rs 365, throws.rs 248, sharing.rs 210, order.rs 211, locks.rs 145 | 2778 | 3-14 each | mostly adapters already; each keeps a `Plan`/`Source` glue |
| contracts/trust.rs | 287 | 4 (trust.nika 965) | adapter; `analyse*` (82-110) and `render`/`trusted_maps` thin; stay |
| parser/mod.rs | 4403 | 17 | 1007-~4300 is `grammar! {}` (winnow_grammar): adapter by design, stays. Movable: Trivia 69-280 (~210), number_at 279-430, Parsed 431-725 (~300), errors 793-1000 (~210) |
| type_rules.rs | 1382 | 0 | pure AST rule rewriter, no host needs |
| modules.rs | 1392 | 7 (modules.nika 219) | `Program` impl 624-1183 (560) is the core; collect/read_unit file IO = host |
| fold_bound.rs | 1626 | 1 (prover_arena) | Widths/Walk/Merge/Uses, pure; needs `sync::visit_*` first |
| comptime_run.rs | 1628 | 6 (comptime_driver) | `computed` 501-661, `worked_out_ahead`, bundle/cargo/fingerprint 1180-1628 = process + file host |

## Adapters by design (stay Rust)
`grammar! {}` block in parser/mod.rs (~3300 lines) and its `Parsed` accessor glue; `Ledger` impl in contracts/mod.rs
(thin calls into ledger*.nika); contracts/{trust,locks,order,throws,sharing}.rs (<300 each, call tools); comptime_run.rs
`cargo`, `waited`, `bundle_*`, `stamp_of` (process, files, clock). Realistic movable total: about 6,500 of 17,356.

## What each needs from the host/world
* maps, type_rules, fold_bound, Trivia, Parsed: nothing; AST and `Parsed` by `ref`, `BTreeMap` -> Nikaia maps. fold_bound
  needs callbacks for `visit_expr(.., fn(ref Expr) -> i64 sync)` until sync.rs's walkers move.
* modules.rs: file reads, directory listing, package paths (`World`/fs functions already used by `libraries.nika`).
* comptime_run.rs: process spawn, temp dirs, content hash, cargo; keep it Rust, move only `computed`/`worked_out_ahead`.

## Name clashes (one namespace; already defined in tools/*.nika)
`Escape`, `Source`, `Declared`, `Program`, `Uses`, `Pause`, `Crossing`, `Reason`, `Ty`, `Unit`, `Root`, `Item`, `Block`,
`Function`, `Expr`, `Found`, `Group`, `Index`-like names. Prefix by module: maps -> `Mp`/`maps_` (`MpRoot`, `MpShape`),
type_rules -> `Tr`, fold_bound -> `Fb` (`FbWalk`, `FbMerge`, `FbUses`), modules -> `Md`, comptime -> `Ct`, parser -> `Pr`.
Free fns named `join`, `find`, `expr`, `block`, `place`, `infer`, `check`, `render`, `locate` exist in several files and
in dsl/interpreter/threads/trust: always `maps_join`, `maps_expr` and so on.

## Order, leaves first
1. maps.rs 1213 (no callees outside ast/types; union-find leaves first). One module, three steps.
2. type_rules.rs 1382 (no tools, self-contained; `Ruled`, `Methods`, `Declared` -> `Tr...`).
3. parser non-grammar leaves: number_at/Trivia/error helpers (~600; `parse_numbers`, `parse_errors` exist).
4. contracts/mod.rs free fns 931-1439 (~500; calls `declared`, `ledger_*` already moved).
5. contracts/send.rs and sync.rs `visit_*` walkers (prerequisite of 6).
6. fold_bound.rs 1626 (Widths/Index leaves, then Walk 490-839, Merge 839-1069, Uses 1079-1253, entries).
7. modules.rs `Program` core (560) after 3-4; then comptime_run `computed`/`worked_out_ahead` (~300).

## First slice: R-2 maps.rs (contracts/maps.rs, tools/maps.nika), chunking
| Step | Functions | Lines |
| :--- | :--- | ---: |
| R-2a leaves | is_map 91, Shape/read/of 123-240, unknown, join, find, union, new_root, doubt, add, sym (238-320) | ~250 |
| R-2b classify | locate, classify, place, roots_of, map_made, deps, set_local (320-460) | ~140 |
| R-2c walk | block, iterated, returned, flows, expr, field_or, expr_base (459-850) | ~400 |
| R-2d tail | resolve, own, fixed, count, lines, analyse, analyse_with, infer, keyed_functions (849-1213) | ~360 |
Rust keeps `analyse`/`infer`/`keyed_functions` as the adapter (pattern `libraries.rs`). Start from the rs2nika draft;
the doc-comment 1-90 stays on the Rust adapter. Run `lower-std` once with the old binary before `cargo build` (new file).
Check first: whether `contracts/maps.rs` uses `Shape::exact` from `ty.rs` (already `tools::ty`) and `Ledger` types.
