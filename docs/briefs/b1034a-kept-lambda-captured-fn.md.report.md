Result: done
Files: crates/nikaia-std/src/tools/check_lambdas.nika crates/nikaia-std/src/tools/lowered/check_lambdas.rs tests/compiler/tests/a_kept_lambda_calls_a_captured_fn.nika tests/compiler/tests/a_kept_lambda_calls_a_captured_fn.stdout
Commit: emit: a kept lambda that calls a captured kept fn clones its handle (#1034)
Body: a local of `fn` type holds a `Kept`, an `Arc`, so `a_name_of_a_hull` now counts it as a hull when it is not in `run_code`. `kept_hulls` records it, and `emit_flow_kept` clones it once around the closure and once per call, the way it already clones a `Shared`. The `let` row and the field row had the same cause, and both pass.
Part: #1034
Pushed: ec386c71 (pushed by drive.nika)
Tests: the new test failed on the unfixed compiler (rustc: Cannot move out of `f`/`g`, Use/Borrow of moved value); after the fix tests/compiler --both-settings 419 passed 0 failed; it function_types:: shared:: lock_floor:: lock_doors:: selfhosting_defects:: plus the guard set 100 passed; clippy clean; pre-push-check ok; lower-std fixpoint clean
Gate: green
Deviation: none
Found: none
Needs: push of ec386c71 was rejected (develop moved); the supervisor rebases
Changelog: A lambda kept in a `let` or a field that calls a function value it captures now shares that value's closure. Before, rustc refused the lowering.
Owner: none
Rs2nika: none
Disk freed: 0 GB (cache 2.0G, 9.0G free; no new cache entry over 100 MB; scratch binary copy removed)
