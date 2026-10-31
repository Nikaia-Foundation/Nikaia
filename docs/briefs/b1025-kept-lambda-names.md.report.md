Result: done
Files: crates/nikaia-std/src/tools/keeps.nika crates/nikaia-std/src/tools/lowered/keeps.rs tests/compiler/tests/a_lambda_kept_by_a_callee_keeps_the_parameter_it_names.nika tests/compiler/tests/a_lambda_kept_by_a_callee_keeps_the_parameter_it_names.stdout tests/compiler/tests/a_lambda_kept_by_a_trait_method_keeps_the_parameter_it_names.nika tests/compiler/tests/a_lambda_kept_by_a_trait_method_keeps_the_parameter_it_names.stdout tests/compiler/tests/a_lambda_run_during_the_call_leaves_the_parameter_lent.nika tests/compiler/tests/a_lambda_run_during_the_call_leaves_the_parameter_lent.stdout
Commit: keeps: a parameter a kept lambda names is kept (#1025)
Body: A lambda argument of a free or method call now hands every parameter its body names (and every parameter a local it names points into) to the callee's column at that position, as a bare argument is handed (Passed for the fixpoint; an unknown callee keeps, D2). A callee that does not keep the position leaves them lent, its body running during the call. A body with a spawn inside names all parameters, as a task does (ADR-312 D1). New helper keeps_named_by_a_lambda; keep_argument and keep_method_arguments take holes_of for it.
Fixes: #1025
Tests: 3 new tests/compiler programs; against the unfixed compiler the two keep rows failed with rustc's "Lifetime may not live long enough", the lent control printed abc/9; tests/compiler --both-settings 403 passed 0 failed (incl. a_lent_parameter_kept_by_a_trait_method_is_taken, a_trait_method_callback_is_handed_on); it keeps:: + guard set 57 passed; clippy clean; lower-std fixpoint clean
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A lent parameter a lambda names, handed to a position the callee keeps, is now kept (owned) instead of reaching rustc as a lifetime error (#1025).
Owner: none
Disk freed: 0
Rs2nika: none
Pushed: 030fadb9453eac1a611502136dd88dba09113245
