Result: done
Files: crates/nikaia-std/src/tools/check_core_args.nika crates/nikaia-std/src/tools/check_core_leaves.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/emit_core_flow.nika crates/nikaia-std/src/tools/lowered/check_core_args.rs crates/nikaia-std/src/tools/lowered/check_core_leaves.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/emit_core_flow.rs tests/compiler/tests/a_let_lambda_handed_where_a_function_is_kept.nika tests/compiler/tests/a_let_lambda_handed_where_a_function_is_kept.stdout
Commit: check: a let-bound lambda handed where a function is kept is kept (Fixes #1043)
Body: Option (i). An untyped `let` that binds a lambda records it in `let_lambdas`, keyed by the binding. Where that name is kept (a parameter the callee keeps, a returned `fn`, a field), `check_core_a_kept_lambda` keeps the let's lambda once, as it does for a typed `let`, marks it in `let_kept_lambdas` and takes the name out of `run_code`. The #997 look-ahead now types the lambda from a keeping callee too. Without a declared type rustc cannot unsize `Kept<closure>` on its own, so the emitter writes the parameters with their types by use and casts the `Arc` to `dyn Fn(_..) -> _`, adding `+ Send + Sync` where the host crosses into user code.
Fixes: #1043
Tests: tests/compiler --both-settings 434 passed, 1 failed (a_trait_column_change_warns, settings disagree on cargo file-lock lines, not this change); new test and the 3 model tests ok; it guard set + selfhosting_defects:: 43 passed (sequences guard passed; it is on Red-on-develop at 18/16); clippy clean; unfixed: rows a and b failed (rustc mismatched types) on the pre-fix binary as probes, not by stash (forbidden in the shared checkout); where.nika lists no tests for check_core_a_kept_lambda
Gate: green
Deviation: none
Found: (c) a kept `fn` field read through a struct, `let h = H { f: g }; h.f(1)` with `g` from an untyped `let`, is refused at check with NK2605 "`H.f` can fail with `?`" whether or not this fix is in (a separate cause, so row c is not in the test). (d) #997 is incomplete: `let add = fn(x) { x + 3 }; apply(add)`, where `apply(f: fn(i64) -> i64)` only runs `f`, gets past check and then rustc refuses it with "`i64` is not a future": the closure is handed as `&add` where `&impl AsyncFn` is wanted. It fails the same way on the unfixed compiler, and this fix leaves its lowering unchanged.
Needs: none
Changelog: A lambda named by an untyped `let` and handed where a function value is kept is kept, instead of a lowering rustc refused.
Owner: none
Disk freed: 0 GB (cache 3.3G kept, 7.5G free on /)
Rs2nika: none
Pushed: 599cef06 (pushed by drive.nika)
