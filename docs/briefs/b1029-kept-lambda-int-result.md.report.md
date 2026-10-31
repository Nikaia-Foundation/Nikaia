Result: done
Files: see step reports (b1029-kept-lambda-int-result.md.step1.report.md, .step2.report.md)
Commit: 1b4be7fc check: a block arm's lent optional part opens its option (#1029); 76f68f63 emit: a kept lambda's integer result types its literals (#1029)
Body: The cast's _ alone could not fix it: rustc carries no expected type through an as cast into the async block (checked on plain rustc), so the emitter writes the future as a typed let, { let __nikaia_future: BoxedIn<'_, i64> = Box::pin(async move {..}); __nikaia_future }, where the checker recorded an integer or optional-integer result (kept_results); the cast with _ stays everywhere else. Recording the result in check_core_leaves.nika met a lowering defect (a ref T? argument from a block arm over an owned value lost its .as_ref()), fixed first as its own commit with its own test.
Fixes: #1029
Pushed: rejected (1b4be7fc, 76f68f63 local on develop; origin/develop moved, needs rebase)
Tests: a_kept_lambda_keeps_its_integer_result failed on the unfixed compiler (i32 error on all 6 rows), passes now; a_block_arm_lends_an_optional_part_inside_its_option: the same shape (probes q2/q9/q10) failed with Mismatched types on the unfixed compiler, passes now; tests/compiler --both-settings 411 passed 0 failed; guard set 24 passed (sequences guard passes); clippy clean; pre-push-check clean; lower-std fixpoint clean
Gate: green
Deviation: step 1's type is written in a typed let around the boxed future, not in the cast's place of _ (rustc does not carry it through the cast)
Found: none
Needs: supervisor rebase and push of 1b4be7fc, 76f68f63
Changelog: A pausing lambda kept under a declared fn type whose result is an integer type, or an optional of one, hands back that type's literals; a part a match binds, handed from a block arm to a ref T? parameter, is lent inside its option.
Owner: none
Disk freed: 0 GB (project-tests cache 1.8G, / 4.6G free, no new entry over 100 MB)
Rs2nika: none
