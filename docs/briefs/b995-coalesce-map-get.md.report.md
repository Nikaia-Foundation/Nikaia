Result: done
Files: crates/nikaia-std/src/tools/check_core_ops.nika crates/nikaia-std/src/tools/lowered/check_core_ops.rs tests/compiler/tests/coalesce_map_get_fallback.nika tests/compiler/tests/coalesce_map_get_fallback.stdout tests/compiler/tests/coalesce_map_get_fallback_of_text_is_refused.nika tests/compiler/tests/coalesce_map_get_fallback_of_text_is_refused.stderr
Commit: check: a map's read as the fallback of `??` beside an owned `T?` is copied or refused (ADR-293 D15)
Body: `a ?? m.get(k)` with an owned `a: T?` now goes through `a_map_read_kept`, as a `return`, `let` or argument that keeps a `T?` does: a number or a value that copies is copied out (`.copied()`), text and a struct are refused with NK1102 naming `?.clone()` (ADR-293 D15, ADR-282 D7: a copy is written, never inserted). `a ?? m.get(k)?.clone()` already lowered and runs; K3a part 2 writes `known(name) ?? ahead.get(name)?.clone()`. The test's lambdas are handed straight to the call: a `let`-bound untyped lambda over a `String` parameter fails apart from this fix (see Found).
Fixes: #995
Tests: coalesce_map_get_fallback (numbers, data-less enum, text and enum with `?.clone()`, lambdas) and coalesce_map_get_fallback_of_text_is_refused, both failed on the unfixed compiler (rustc E0308 at :17, :21 and :7); tests/compiler --both-settings 354 passed
Gate: green
Deviation: the brief's `let wider = fn(n) { ... }` is tested as `ask(fn(n) { ... }, k)`; the let-bound form is blocked by the defect under Found
Found: (1) `let w = fn(n) { known(n) }` with `known(n: String)`, handed on as `fn(String) -> i64? sync`, reaches rustc: lowers `|n| known(&n)`, rustc infers `n: str` (E0277 Sized); same with `fn(n) { more.get(n) }`, which also misses the `.copied()` (Option<&i64>). (2) `fn a(got: BV) -> String { return match got { BV::Int(i) => i.to_string(), BV::Text(t) => t.clone() } }` is refused NK2302 "hands back a view of `got`"; with the Int arm `=> "n"` it passes
Needs: none
Changelog: A map's read as the fallback of `??` beside an owned `T?` is copied out where the value copies and refused with `?.clone()` otherwise, instead of reaching rustc (#995).
Owner: none
Disk freed: 0
Rs2nika: none
Pushed: 0cf0b173a66ebe82c02ec8c055a5dd83e3d88cd8
