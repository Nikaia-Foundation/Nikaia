Result: done
Files: crates/nikaia-std/src/tools/emit_function.nika crates/nikaia-std/src/tools/emit_ops.nika crates/nikaia-std/src/tools/emit_stmt.nika crates/nikaia-std/src/tools/lowered/check_core_places.rs crates/nikaia-std/src/tools/lowered/emit_function.rs crates/nikaia-std/src/tools/lowered/emit_ops.rs crates/nikaia-std/src/tools/lowered/emit_stmt.rs tests/compiler/tests/a_list_view_named_lifetime_is_a_slice.nika tests/compiler/tests/a_list_view_named_lifetime_is_a_slice.stdout docs/briefs/bugs/b4-1016.patch docs/briefs/bugs/b4-1016.test.nika docs/briefs/bugs/b4-1016.test.stdout
Commit: emit: a ref Vec[T] with a named lifetime, and a typed let of one, lower as a slice
Body: a_list_read_as_a_slice (emit_ops) now also rewrites `&'r Vec<T>` to `&'r [T]` (never `&mut`), so a result sharing a lifetime is a slice; the parameter branch of emit_function slices `&Vec<T>` and `&'r Vec<T>` (a stored `&'a` and a kept `&'k` keep the field's or keep's spelling); emit_stmt_annotation slices the declared type of a typed `let x: ref Vec[T]`. lower-std re-lowers check_core_places.rs (two typed lets of a `ref Vec[FieldContract]` now `&[FieldContract]`); fixpoint holds. Removes b4-1016's attachments.
Part: #1016
Pushed: 073dcd0486a77524c5b2face67ff2ac5a7d3360b
Tests: new a_list_view_named_lifetime_is_a_slice failed on the unfixed compiler (rustc "Arguments to this function are incorrect" 8:5, "Mismatched types" 11:5), passes now; gate ran tests/compiler whole at both settings (emit_*.nika changed) plus guards: green; no it test holds the old `&'r Vec<` / `: &Vec<` text
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A `ref Vec[T]` parameter or result with a named lifetime, and a typed `let` of a `ref Vec[T]`, lower as a slice (`&'r [T]`, `&[T]`), so a slice argument reaches them (#1016).
Owner: none
Disk freed: 0
Rs2nika: none
