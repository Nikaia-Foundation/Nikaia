Result: done
Files: crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/check_core_expr.nika crates/nikaia-std/src/tools/emit_core_reads.nika crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/check_core_expr.rs crates/nikaia-std/src/tools/lowered/emit_core_reads.rs tests/compiler/tests/a_kept_fn_read_by_index.nika tests/compiler/tests/a_kept_fn_read_by_index.stdout
Commit: emit: a kept fn read by index shares the closure (#1018)
Body: The checker records a new fact, `kept_fn_reads` (check_facts.nika, keyed by the node_key of the index's base), for every index read whose type is a `fn`; emit_core_index_read then writes `get(&xs, i).clone()` (and `proven::read(..).clone()`) instead of `*get(&xs, i)`, a clone of the `Kept` Arc, so the copy shares the one closure (Part I, *A function type stands wherever a type does*). This covers `let f = fs[0]`, `fs.push(fs[0])` and `keep(fs[0])` to an owning parameter; a run-only parameter (`apply(fs[0], 2)`) already worked and now reads `(*get(..).clone())`.
Fixes: #1018
Pushed: d5995dbd (pushed by drive.nika)
Tests: a_kept_fn_read_by_index new, failed on the unfixed compiler (rustc `Cannot move out of a shared reference` at line 4), passes now; tests/compiler --both-settings 408 passed; probes push/keep/loop of fs[i] pass; lower-std fixpoint clean
Gate: green
Deviation: none
Found: the same false acceptance for a kept fn copied from a variable or a field: `let f: fn(i64) -> i64 = fn(x) { x + 5 }; let g = f; println(f(1))` (rustc `Borrow of moved value: f`), and `let a = h.f; let b = h.f` for `struct H { f: fn(i64) -> i64 }` (rustc `Use of moved value: h.f`); both lower to a move of the `Kept`, not a clone
Needs: none
Changelog: A function value read from a list by index is a copy that shares the closure, as Part I says, instead of a lowering rustc refused.
Owner: none
Disk freed: 0 (cache 1.8G, 4.6G free on /, no new entry over 100 MB)
Rs2nika: none
