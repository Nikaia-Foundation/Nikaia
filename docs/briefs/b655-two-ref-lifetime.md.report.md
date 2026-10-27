Result: done
Files: crates/nikaia-std/src/tools/emit_function.nika crates/nikaia-std/src/tools/lowered/emit_function.rs tests/compiler/tests/a_view_result_of_two_ref_parameters.nika tests/compiler/tests/a_view_result_of_two_ref_parameters.stdout
Commit: A view result of two ref parameters is lowered with their lifetime
Fixes: #655
Tests: tests/compiler both settings 135 ok; it guard set + lifetime/views_kept/element_views/view_result 49 ok; unfixed compiler (probe a, before the edit) died with rustc Missing lifetime specifier, not re-run with the fix stashed
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A function with two `ref Vec[T]` parameters returning `ref T?` or `ref T` is lowered with one lifetime over both parameters instead of dying in rustc.
Owner: none
Disk freed: 0
Pushed: db99e129363fcc3349d8d48a9816028a5b9be7e5
