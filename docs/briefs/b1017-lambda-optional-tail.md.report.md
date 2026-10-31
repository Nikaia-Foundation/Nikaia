Result: done
Files: crates/nikaia-std/src/tools/check_function.nika crates/nikaia-std/src/tools/check_core_ops.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/check_function.rs crates/nikaia-std/src/tools/lowered/check_core_ops.rs tests/compiler/tests/a_lambda_typed_optional_widens_its_arms.nika tests/compiler/tests/a_lambda_typed_optional_widens_its_arms.stdout
Commit: check: a lambda typed -> T? wraps its non-null arms in Some (#1017)
Body: The wrap a named function's tail takes (some_at_the_tails for a branching tail under a nullable result, else the slot's FactWrap) moves into check_function_wrap_tail; check_core_ops_closure calls it for a lambda whose place declares a fn type with a nullable result. The site is check_core_ops_closure, not check_core_lambda_leave: its `outer` is the enclosing function's expected type, not the lambda's declared result. The emitter needed no change (emit_expr reads some_tails and wrapped on any node).
Fixes: #1017
Pushed: ee9aa6a6bc9cfd0562447a3ad3c5e3bc90631b8f
Tests: new a_lambda_typed_optional_widens_its_arms (tests/compiler --both-settings 407 passed); its lines 1 and 3 failed on the unfixed binary (rustc Option<i64> vs i64), line 2 printed 9; guard set 24 passed; clippy clean; lower-std fixpoint clean
Gate: green
Deviation: none
Found: a lambda whose tail is a match of bare literals takes i32 for them, not its declared result: `let k: fn(i64) -> i64 = fn(x) { match x { 1 => 5, else => 7 } }` / `println(k(1))` fails in rustc (resolves to i32, expected i64), with or without `?`; the named fn of the same body prints 5; state.number_asked on the lambda tail did not fix it
Needs: none
Changelog: A lambda whose declared result is optional widens a non-optional tail into `Some`, as a function does (#1017).
Owner: none
Disk freed: 0 GB (no clone, no own target; scratch binary copy removed; cache 1.8G, disk 4.6G free, no new entry over 100 MB)
Rs2nika: none
