Result: done
Files: crates/nikaia-std/src/tools/check_core_args.nika crates/nikaia-std/src/tools/check_core_ops.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/check_flow.nika crates/nikaia-std/src/tools/emit_core_flow.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/check_core_args.rs crates/nikaia-std/src/tools/lowered/check_core_ops.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/check_flow.rs crates/nikaia-std/src/tools/lowered/emit_core_flow.rs tests/compiler/tests/let_lambda_string_param.nika tests/compiler/tests/let_lambda_string_param.stdout
Commit: check, emit: a let-bound lambda takes its parameter types from its first use
Body: A `let` lambda without a type whose parameters are untyped is now typed by the first call in the rest of its block that hands it to a parameter of `fn` type that runs it (as ADR-135 D2 types an empty list by its first use): the checker sets that type as the lambda's expected one, so the body is checked as the same lambda written into the call (`as_text`, `.copied()`), records the parameter types (`lambda_params_by_use`) which the emitter writes on the closure (`|n: String|`), and treats the name as code run in place (`run_code`), so it is handed on as `&w`, not as a kept value. A callee that keeps the value is left as before (a kept lambda needs its type on the `let`). Separately, a lambda checked by `check_core_ops_closure` (a let-bound one, also with a declared type) now copies a map read that is its last value, as one handed to a call does (`let v: fn(String) -> i64? sync = fn(n) { m.get(n) }` missed `.copied()`).
Fixes: #997
Pushed: bad24d06 (pushed by drive.nika)
Tests: tests/compiler --both-settings 355/355 ok both (new let_lambda_string_param); rows 1-3 failed in rustc on the unfixed compiler (probes before the change: E0277 str Sized, Borrow<String>, Option<&i64>); it guard set 23 ok; clippy clean; pre-push-check ok; lower-std fixpoint
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A lambda bound by `let` without types takes its parameter types from the first call it is handed to, and a map read as its last value is copied out.
Owner: none
Disk freed: 0
Rs2nika: none
