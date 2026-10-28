Result: done
Files: crates/nikaia-std/src/tools/node_numbers.nika crates/nikaia-std/src/tools/lowered/node_numbers.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/described.contracts crates/nikaia-std/src/tools/tree_build.nika crates/nikaia-std/src/tools/lowered/tree_build.rs crates/nikaia/src/parser/mod.rs crates/nikaia/tests/it/expr_ids.rs docs/specification/adr/README.md scripts/self_hosting.py assets/badges/self-hosted.svg
Commit: parser: the numbering pass gives every parsed node its id; the grammar draws none (ADR-383)
Body: tools/node_numbers.nika walks the finished tree by value (node before children, left to right) and numbers every parsed id from `first`, dense, collecting the spans the rules put on the ids by `id - first`; parse_to_ast runs it before one_variant::expand, parse_expression_from on the hole under the caller's lock. The 71 `_state.user().id()` are `NodeId::parsed()`, the 27 `.at(e, _span)` `ast::spanned(e, cut(_span))` (the trailing-trivia cut stays #557's); the folds take no `first`, `guarded` no id; Trivia loses next_id, spans, spans_from, id(), reserve(), at(). Contracts for NodeId::parsed/is_parsed/span. size_of::<Expr>() unchanged (88).
Fixes: #903
Tests: nikaia test --both-settings --project tests/compiler 195 ok; whole it binary 2886 ok + examples:: ok (after step 1's contract); expr_ids::ids_follow_parse_order updated to the input order (dense, pre-order); guard set + sysroot:: ok; clippy clean; lower-std fixpoint, only node_numbers.rs, tree_build.rs, lowered.rs (and step 1's keeps.rs) changed
Gate: green
Deviation: Parsed::expr_spans stays a BTreeMap (filled from the pass's list) because tools/parsed_view.nika reads it as a map and the brief keeps the other lowered files unchanged; a step 1 compiler fix commit (keeps) before this one
Found: lowering writes `let mut x = x;` for a match binding whose list is drained (`E::A(xs) => for x in xs.drain()`): `changed_bindings` counts the drain, the emitter writes `into_iter()`, rustc warns unused_mut (clippy -D warnings red); node_numbers.nika keeps such loops in functions taking the list. In a user program the same arm is refused NK1102 (`x` is `ref String`). Issue text: probe `enum E { A(Vec[String]), N }` + `fn g(e: E) -> Vec[String] { match e { E::A(xs) => { let mut out: Vec[String] = []; for x in xs.drain() { out.push(x) }; return out }, else => { return [] } } }`
Needs: rebase (push rejected: local develop carries 4e6b690 website, origin has 00451bf, d6fd9ba)
Changelog: The parser numbers the nodes of a file in a pass of its own, in input order and without gaps, right after the parse (ADR-383); ids no longer follow the order the grammar made the nodes in.
Owner: none
Rs2nika: none
Pushed: ee19152 (pushed by drive.nika)
Disk: target/nikaia-project-tests 3.8G, / 17G free; new entries over 100 MB: run-bundle/65d964130cc06fb2 300 MB; removed examples/access-log/target
