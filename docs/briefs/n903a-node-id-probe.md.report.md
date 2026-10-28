Result: done
Files: crates/nikaia-std/src/node_id.rs crates/nikaia/src/ast/mod.rs
Commit: node_id: a parsed id carries its span until numbered (#903)
Body: `NodeId` becomes number + span (two u32, start u32::MAX where none); `NodeId::parsed()` (number `PARSED` = FIRST_BUILT - 1), `at(start, end)` keeps the first span set (innermost first), `span()`, `is_parsed()`; `numbered`/`fresh` carry no span; D3 equality and `Debug` unchanged; `ast::spanned(expr, span)` puts it on a node (set_id now shares `id_slot`). Not wired into the grammar yet (steps 3-4). size_of::<Expr>() 80 -> 88, Stmt 200 -> 208.
Part: #903
Tests: step 1 probe (no commit): Trivia gave ids in reverse order (spans kept), then nikaia test --both-settings --project tests/compiler 194 ok, it errors:: parse_error:: samples:: diagnostics:: assert:: assume:: facts_at_calls:: bounds_report:: errors_lowering:: accepted_compiles:: examples:: type_rules:: text_tiers:: claims:: narrowing:: overflow:: all ok, lower-std unchanged; only expr_ids::ids_follow_parse_order failed (it asserts the old order itself; steps 3-4 update it); no reader depends on the order, so no fix commit. Step 2: compiler project 194 ok both settings, it errors:: expr_ids:: parse_error:: samples:: diagnostics:: ok, nikaia-std node_id:: ok, lower-std unchanged; no new test (Rust-only API, nothing a program sees yet)
Gate: green
Deviation: issue #903 text not in the brief: built from ADR-383 and the brief; node_id.rs is in every program's std (no `tools`), so the span is two u32, not `ast::Span`; no described.contracts entries for parsed/at/span (step 3's Nikaia pass adds the signatures it needs)
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: none
Pushed: 39ebd8393ef28e3799e7893546926cbb8d42ceaa
