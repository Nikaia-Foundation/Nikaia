Result: done
Files: crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/check_core_expr.nika crates/nikaia-std/src/tools/check_core_call.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/check_core_places.nika crates/nikaia-std/src/tools/emit_expr.nika crates/nikaia-std/src/tools/emit_core_args.nika crates/nikaia-std/src/tools/emit_core_places.nika crates/nikaia-std/src/tools/emit_state.nika crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/check_core_expr.rs crates/nikaia-std/src/tools/lowered/check_core_call.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/check_core_places.rs crates/nikaia-std/src/tools/lowered/emit_expr.rs crates/nikaia-std/src/tools/lowered/emit_core_args.rs crates/nikaia-std/src/tools/lowered/emit_core_places.rs crates/nikaia-std/src/tools/lowered/emit_state.rs tests/compiler/tests/a_kept_fn_copied_from_a_name_or_field.nika tests/compiler/tests/a_kept_fn_copied_from_a_name_or_field.stdout
Commit: emit: a kept fn copied from a name or a field shares the closure (#1031)
Body: The checker records a name or field read of fn type that holds a Kept (a local, a field; not a run-only parameter in run_code, not a named function) in kept_fn_copies, and takes it out again where the read is a callee or an assignment target; emit_expr writes .clone() after such a read, and an argument decides for itself (walk.uncopied): cloned where handed over by value, not where lent or lent as its closure, once where hands_on_kept already clones. The source shorthand P { f } is covered by kept_fn_fields. A tuple part read back by let (p, _) = t needs nothing beyond the clone at (f, 1). Also probed: g = f, m.f = g, return h.f from an owned parameter, [f, g, f], a for over a Vec of fns, if c { f } else { g }.
Fixes: #1031
Pushed: 5a201f0b (pushed by drive.nika)
Tests: new a_kept_fn_copied_from_a_name_or_field and a_kept_fn_read_by_index pass; nikaia test --both-settings --project tests/compiler 414 passed at both; on the unfixed compiler the program lowered to let g = f; then f(1) (the move rustc refuses, the architect's run at d7e56e49: Borrow of moved value: f), not re-run after the fix since stash is barred; lower-std fixpoint clean
Gate: green
Deviation: none
Found: a kept fn captured by a lambda is moved into it: let w: fn(i64) -> i64 = fn(x) { f(x) + g(x) } then f(1) gives rustc Use of moved value (move closure, no clone); and return h.f from h: ref H is refused NK1131 (You can't give away h.f, h is only borrowed) although the read is a copy
Needs: none
Changelog: A function value copied from a name, a field or a tuple part shares its closure, as Part I says, instead of a lowering rustc refused.
Owner: none
Disk freed: 0 GB
Rs2nika: none
