Result: done
Files: crates/nikaia-std/src/tools/check_core_comptime.nika crates/nikaia-std/src/tools/comptime_tree.nika crates/nikaia-std/src/tools/dump.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/check_core_comptime.rs crates/nikaia-std/src/tools/lowered/comptime_tree.rs crates/nikaia-std/src/tools/lowered/dump.rs tests/compiler/tests/a_comptime_reads_the_reflected_field.nika tests/compiler/tests/a_comptime_reads_the_reflected_field.stdout tests/compiler/tests/a_comptime_calls_a_function_over_a_field.nika tests/compiler/tests/a_comptime_calls_a_function_over_a_field.stdout tests/compiler/tests/a_comptime_option_crosses.nika tests/compiler/tests/a_comptime_option_crosses.stdout
Commit: comptime: a turn's field handed to a function, its attributes read, and T? crossing from build time
Body: A comptime in a turn that reads the field itself (wire(f), f.attribute(X)?.value, f"<{f.name}>") is marked (TURN_MARK). The sub-program then gets a generic __nikaia_comptime_turn[T: Struct]() over T::fields that keeps the turn's field and hands back the value; it collects into a Vec and calls .remove(0) because a panic at build time is refused (Found). dump writes T? as (v Option None|Some ..). The checker spells it Option<..> and a String? arrives as ref String?. A literal null folds to None; a T value fits a declared T?. The sub-program carries a Some as its literal. No .stderr pinned String? as uncrossable, so none was updated.
Fixes: #1087
Part: #1019
Pushed: local 974b9ac4a09ba9d1cf462b6f02b225141fc8fe75
Tests: the 3 new/extended tests pass; 54 programs (comptime_*, grammar_*, a_comptime_*, a_function_takes_a_reflected_field, reflected_field_ty, a_sync_function_calls_a_generic_by_brackets) pass --both-settings. On the unfixed compiler the brief's probes and mine (wire.nika, attr.nika, opt.nika) failed with NK1127. lower-std fixpoint clean. clippy (stable and 1.99.1), fmt and pre-push-check green. nikaia-std --lib for the 3 modules: 0 tests.
Gate: green
Deviation: none. grammar_decode.nika and build_values.nika needed no change: the decoder already reads (v Option ..) as BuildValue::Variant.
Found: (1) a panic() in a sync function run at build time gives NK1152 "Nothing says what it touches outside the program", although Part II 10.2 allows the run to stop with panic(). Probe: a sync fn that calls panic("x") on one branch, called from a comptime. (2) A named-field variant pattern with a nested variant on a boxed field does not compile in rustc (Box<E> vs E): enum E { Num(i64), Call { func: E, n: i64 } }; match e { E::Call { func: E::Num(k), n } if n > 0 => k + n, else => 0 }. The tuple form works. Suspect emit_core_match/emit_misc boxed_bindings.
Needs: none
Changelog: A comptime in an unrolled turn may hand the turn's field to a function and read its attributes, and a T? now crosses from build time, null included (#1087).
Owner: none
Disk freed: 0 (project-tests cache 3.8G, / 9.0G free; no new cache entry over 100 MB)
Rs2nika: none
