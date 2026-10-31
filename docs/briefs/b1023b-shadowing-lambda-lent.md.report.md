Result: done
Files: crates/nikaia-std/src/tools/check_fields.nika crates/nikaia-std/src/tools/lowered/check_fields.rs tests/compiler/tests/a_shadowing_lambda_is_lent.nika tests/compiler/tests/a_shadowing_lambda_is_lent.stdout
Commit: checker: a let that shadows a fn parameter is not the lent parameter (Fixes #1023)
Body: a_lent_parameter now answers false unless the name's innermost binding lies in the outermost frame that binds it; a function's walk begins on an empty scope (hollow() in check/core.rs), so that frame is the parameter frame check_function pushes, and a lambda's capture of the parameter still finds it there. No CheckState field was needed, which kept core.rs (outside the file list) untouched. The test's lambda argument is `_name` to keep the run free of an unused-variable warning.
Fixes: #1023
Pushed: 0fbaf20ecc8f035ed91307b6841735633872af0b
Tests: a_shadowing_lambda_is_lent failed on the unfixed compiler (rustc Mismatched types), passes now; nikaia test --both-settings --project tests/compiler 398 passed 0 failed (incl. a_lambda_handed_round_a_cycle_runs, a_recursive_function_hands_a_new_lambda_to_itself, a_trait_method_*, a_method_hands_its_callback_to_a_function); guard set 24 passed; clippy clean; pre-push-check clean
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A let lambda that shadows a fn parameter is lent with & when handed on (#1023).
Owner: none
Disk freed: 0.3 GB (scratch binary copy); test cache 530M, disk 6.3G free; new cache entries over 100 MB: rlib/df8bcfe6 265M, rlib/af187e66 265M
Rs2nika: none
