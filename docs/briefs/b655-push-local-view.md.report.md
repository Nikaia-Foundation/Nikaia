Result: done
Files: crates/nikaia-std/src/tools/views.nika crates/nikaia-std/src/tools/check_arguments.nika crates/nikaia-std/src/tools/lowered/views.rs crates/nikaia-std/src/tools/lowered/check_arguments.rs assets/badges/self-hosted.svg tests/compiler/tests/a_view_pushed_into_a_local_vec_is_not_kept.nika tests/compiler/tests/a_view_pushed_into_a_local_vec_is_not_kept.stdout tests/compiler/tests/a_view_pushed_into_a_local_vec_that_leaves_is_kept.nika tests/compiler/tests/a_view_pushed_into_a_local_vec_that_leaves_is_kept.stderr
Commit: A view pushed into a local Vec that does not leave is not kept (Part of #655)
Part: #655
Fixes: none
Pushed: 2e1ba5db4b5c15a0b8ce59e6e23ad8e1ea73d846
Tests: tests/compiler both settings 131 ok; it keeps/result_source/stored_views/tethers/text_views/views_kept 174 ok; nikaia-std lib 239 ok; guard set ok; clippy clean. Cases: (a) prints 4 with for-loop, (b) stays NK2302, (c) mut Vec param to a callee that pushes stays refused (the callee's own keep), (d) Vec[ref E?] push fails in rustc "Mismatched types" = #655 item 2 (step 2). Not run against the unfixed compiler (stash) for (a): probe reproduced NK2302 on f4dfb8c per brief.
Gate: green
Deviation: check_arguments: arm index replaced by a ref Expr? field `given` in CheckArgument plus `arm: bool` (needed for #596 choice arms); all_arms removed; check_arguments_at kept, since `ref Expr?` cannot yet be built from an `if` or assigned from a `ref Expr` (NK1103/NK1105)
Found: none
Needs: none
Changelog: The checker no longer refuses a view pushed into a local Vec that never leaves the function (NK2302).
Owner: none
