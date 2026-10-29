Result: done
Files: crates/nikaia-std/src/tools/check_compare.nika crates/nikaia-std/src/tools/check_core_expr.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/check_crossings.nika crates/nikaia-std/src/tools/check_function.nika crates/nikaia-std/src/tools/check_state.nika crates/nikaia-std/src/tools/check_takes.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/check_compare.rs crates/nikaia-std/src/tools/lowered/check_core_expr.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/check_crossings.rs crates/nikaia-std/src/tools/lowered/check_function.rs crates/nikaia-std/src/tools/lowered/check_state.rs crates/nikaia-std/src/tools/lowered/check_takes.rs crates/nikaia/src/check/core.rs tests/compiler/tests/a_failing_cleanup_closed_by_hand_is_handled.nika tests/compiler/tests/a_failing_cleanup_closed_by_hand_is_handled.stdout tests/compiler/tests/a_failing_cleanup_left_to_the_block_end_is_nk2605.nika tests/compiler/tests/a_failing_cleanup_left_to_the_block_end_is_nk2605.stderr
Commit: check: NK2605 only for a failing cleanup still held where its block ends
Body: The let's NK2605 is no longer said at once: a_failing_cleanup_refusal builds it, and it waits in CheckState.cleanups_pending until its block ends (failing_cleanups_settled in check_core_block), where it is dropped if the value was closed by hand on every path: a close() hand-over of the same binding, in the let's own branch (same choices), with no return, break or continue of its loop between the let and the close. A close in one arm of an if stays refused, and so does a close in both arms (fail-closed, not proved); a close in a loop or lambda is NK2105 already. Leftovers no block settled are said at the function's end. A kept parameter (check_function) keeps the eager refusal. The .stdout holds what the program prints, close: cleaning up a `Flaky` failed: closed would not flush, as cleanup_points expects; the brief's text lacked the wrapper.
Fixes: #934
Pushed: b1759a101eb3bafa97034854efb91e3f04e20372
Tests: both new programs pass at both settings; the first failed with NK2605 on the unfixed compiler; probes (close in one branch, return before close, break before close, shadowed let, close in a nested block) give NK2605 except the nested block; it cleanup errors_lowering file_paths plus guard set 79 passed; check:: unit ok
Gate: green
Deviation: the .stdout of the closed-by-hand test is the real output with the cleanup wrapper, not the brief's shorter text
Found: none
Needs: none
Changelog: A value whose cleanup can fail, closed by hand with `close()` on every path of its block, is no longer refused with NK2605.
Owner: none
Disk freed: 0
Rs2nika: none
