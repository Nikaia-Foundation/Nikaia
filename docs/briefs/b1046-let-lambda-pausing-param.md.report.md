Result: done
Files: crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/emit_core_flow.nika crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/emit_core_flow.rs tests/compiler/tests/an_untyped_let_lambda_run_by_a_pausing_parameter.nika tests/compiler/tests/an_untyped_let_lambda_run_by_a_pausing_parameter.stdout
Commit: emit: an untyped let lambda run by a parameter that may pause is an async closure
Body: check_core_stmt_let records a lambda typed by a use whose fn type has no `sync` in run_lambdas (the fact an inline lambda run in place gets; the let's closure is not an argument, so emit_arg never reads it there), and emit_core_closure writes `async ` for it as for pausing_lambdas. Row a had a second cause: the search for the first use (check_core_stmt_handed_to) did not look into the holes of an f-string, so `println(f"{apply(add)}")` typed nothing; it now takes the host and walks host.literal_holes (new check_core_stmt_handed_in), as a_claim_looks_at does.
Fixes: #1046
Tests: on the unfixed binary rows a, b refused by rustc ("`i64` is not a future"), c/d/e printed 4; fixed: a-e print 4; new test passes; let_lambda_string_param, a_let_lambda_handed_where_a_function_is_kept, an_untyped_let_lambda_in_a_fn_field pass; tests/compiler --both-settings 438 passed, 3 failed (a_lent_argument_from_arms_view_and_new, a_returned_map_read_of_literal_text_is_copied, an_array_is_walked_with_iter_and_zip: all the same NK2302 on `places` in tests/compiler/src/main.nika at user-parallelism=yes, a file with no let lambda, so my change never runs there); where.nika lists no tests; guard set 24 passed (sequences green); clippy, fmt, pre-push clean; lower-std fixpoint clean
Gate: green
Deviation: none
Found: tests/compiler --both-settings: 3 tests "settings disagree", NK2302 `places` hands back a view of `s` (tests/compiler/src/main.nika:13, last changed 262d5fa6) at user-parallelism=yes only
Needs: none
Changelog: An untyped lambda bound by `let` and handed to a function parameter that may pause now lowers as an `async` closure instead of being refused by rustc.
Owner: none
Rs2nika: none
Pushed: 8b964882651a440f5413f03e8c39e955913c39f1
Disk freed: 0.2 GB (scratch binary copy); target/nikaia-project-tests 3.3G, / 7.9G free, no new cache entry over 100 MB
