Result: done
Files: crates/nikaia-std/src/tools/check_references.nika crates/nikaia-std/src/tools/emit_core_args.nika crates/nikaia-std/src/tools/lowered/check_references.rs crates/nikaia-std/src/tools/lowered/emit_core_args.rs tests/compiler/tests/lent_option_param_bare_arg.nika tests/compiler/tests/lent_option_param_bare_arg.stdout
Commit: check: a bare value handed to a lent option parameter goes inside its option (Fixes #1080)
Body: The fix went in at the end of the_compiler_writes_the_reference (check_references.nika), the path that writes the `&` and returns before check_arguments' later branches run. It already put a plain value lent to a `ref T?` in an option; it now does the same for a `T?` of this program's own that the callee reads as `&Option<T>`. Only own callees, because a std entry's lent option lowers otherwise. The other functions with this rule (the views_into_text branch, #598, and the element-read branch, #655, in check_arguments.nika) cover their own cases and were left unchanged. In the emitter (emit_core_args.nika), a Some wrap on a lent argument whose parameter is not an option of views (emit_arg_opening is empty) is now written `&Some(x.clone())` for a name or field and `&Some(expr)` for a value made in the call, never `Some(&x)`. No other lowered tools file changed.
Fixes: #1080
Tests: unfixed binary (suite worktree 8153b8c0): rows a and b plus a later-read row and a call-result row, 3 rustc Mismatched types. Fixed: tests/compiler --both-settings 496 passed, 0 failed (the new lent_option_param_bare_arg is one of them). it filters nullable option argument lent: 233 passed. lower-std fixpoint clean. fmt, clippy 1.97 and 1.99.1, pre-push-check all ok.
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A plain value handed to a `T?` parameter that the function only reads is now wrapped in its option (`&Some(..)`) instead of reaching rustc as `&value` (#1080).
Owner: none
Rs2nika: none
Disk freed: 0 GB (cache 4.4G, / 6.0G free; new cache entry over 100 MB: run-bundle/3f8b93b88f7f213d 475 MB)
Pushed: local 3b35d950e5015598f2c7f7ffd7b4a1508aa5e2f7
