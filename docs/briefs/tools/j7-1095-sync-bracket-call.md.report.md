Result: done
Files: crates/nikaia-std/src/tools/foreign.nika crates/nikaia-std/src/tools/lowered/foreign.rs tests/compiler/tests/a_sync_function_calls_a_generic_by_brackets.nika tests/compiler/tests/a_sync_function_calls_a_generic_by_brackets.stdout tests/compiler/tests/a_sync_function_calls_a_pausing_generic_by_brackets.nika tests/compiler/tests/a_sync_function_calls_a_pausing_generic_by_brackets.stderr
Commit: sync and comptime: a call with type arguments in brackets is named by its base in the effect walk
Body: `a_named_call` (foreign.nika) named a callee only for `Expr::Variable` and `Expr::Path`, so `id[i64](3)`, an `Expr::Index`, was recorded as `Seen::Opaque` and every `sync` caller or `comptime` of it was refused (NK2202, NK1152). It now names an `Expr::Index` call by its base (variable or path), so the call takes the generic function's promise as `id(3)` does (Part I 4.7, ADR-288). A bracket call of a function that can pause stays NK2202 and now names it. The brief's second probe declared `comptime W: String`, which NK1167 refuses by design. The test uses `ref String`, as that refusal's help says. A non-function base such as `fs[0](x)` is now named `fs` and still refused, as an opaque name.
Fixes: #1095
Part: #1019
Tests: both new tests pass with --both-settings in a scratch project; both FAILED on the compiler without the change; comptime_* tests and the tests/compiler programs carrying NK2202/NK1152 or sync functions (24 programs): all pass except a_foreign_impl_keeping_beyond_the_column_is_refused and a_library_trait_with_no_impl_runs, which fail the same way on the old binary in a scratch project (missing support files); it: sync_of_a_lambda:: comptime*:: build_time:: 103 passed; lower-std fixpoint clean
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A `sync` function or a `comptime` may call a generic function with type arguments in brackets, `id[i64](3)`; it takes that function's promise (#1095).
Owner: none
Disk freed: 0.4 GB (clippy 1.99.1 target dir deleted)
Rs2nika: none
Pushed: local db6e78766576cd70beaf5cc4127045b3a45c1845
