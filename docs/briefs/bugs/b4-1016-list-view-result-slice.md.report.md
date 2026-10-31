Result: partial
Files: crates/nikaia-std/src/tools/emit_function.nika crates/nikaia-std/src/tools/emit_item.nika crates/nikaia-std/src/tools/emit_core_types.nika crates/nikaia-std/src/tools/lowered/emit_function.rs crates/nikaia-std/src/tools/lowered/emit_item.rs crates/nikaia-std/src/tools/lowered/emit_core_types.rs tests/compiler/tests/a_list_view_result_is_a_slice.nika tests/compiler/tests/a_list_view_result_is_a_slice.stdout
Commit: emit: a ref Vec[T] result lowers as &[T], like its parameter (#1016)
Body: A function's, a method's, a trait method's (its declaration too) and a function type's `ref Vec[T]` result now goes through a_list_read_as_a_slice, as the parameters do, so `return a` of a `&[T]` parameter compiles and a field's `&Vec<T>` coerces. The Call form (`for<'call> Fn(&'call Vec<T>, ..)`) and a function with named `'r` lifetimes keep `&'r Vec<T>` on both sides, as their parameters do today. The patch is b3-1016's, plus the code type's result in emit_core_types.
Part: #1016
Pushed: rejected e4a2dee3
Tests: a_list_view_result_is_a_slice failed on the unfixed compiler (rustc E0308 at lines 5 and 15), passes now; tests/compiler --both-settings 430 passed, 4 failed (grammar_a_tuple_result_takes_the_views_its_action_builds, grammar_text_binds_its_inner_parts, keep_moved_after_cut, regex_is_match: settings disagree, NK2302 on `places` in tests/compiler/src/main.nika:13 from 262d5fa6, a checker refusal my emit-only change cannot reach); clippy clean; binary ran with another session's DBG1043 eprints built in; guard set green (sequences passed)
Gate: green
Deviation: step 2 not built: a nullable `ref Vec[T]?` parameter fed from an `Option<Vec<T>>` lowers as `.as_ref()` (`emit_core_args.nika:45`, `emit_expr.nika:150`), which does not coerce to `Option<&[T]>` (callers `hands_on`, `const_field_type`, `copied_path` in tools), so parameter and result cannot both become `Option<&[T]>` within my files
Found: a typed `let w: ref Vec[i64] = a` of a `ref Vec` parameter lowers `let w: &Vec<i64> = a` with `a: &[i64]` (rustc E0308), before and after this change; probe: `fn f(a: ref Vec[i64]) -> i64 { let w: ref Vec[i64] = a; return w.len() }`; also `fn(ref Vec[i64], ref Vec[i64]) -> ref Vec[i64]` called with a `ref Vec` parameter (`&'call Vec<i64>` gets `&[i64]`)
Needs: supervisor push of e4a2dee3 (local develop 1 behind origin; committed by git commit --only because another agent has staged files and an unmerged check_facts.rs, which commit.nika refuses); a brief for step 2 that owns emit_core_args.nika and emit_expr.nika (`.as_deref()` for a list option lent to a slice option), and for the typed let and the Call-form parameters
Changelog: A `ref Vec[T]` result lowers as a slice `&[T]`, as its parameter does, so a function handing back its list parameter compiles (#1016).
Owner: none
Disk freed: 0.1 GB (my binary copy); no clone of my own; cache 3.3G, disk 7.5G free, no new entry over 100 MB
Rs2nika: none
