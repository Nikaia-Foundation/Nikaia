Result: done
Files: crates/nikaia-std/src/tools/types.nika crates/nikaia-std/src/tools/check_compare.nika crates/nikaia-std/src/tools/check_core_call_on.nika crates/nikaia-std/src/tools/check_decls.nika crates/nikaia-std/src/tools/lowered/types.rs crates/nikaia-std/src/tools/lowered/check_compare.rs crates/nikaia-std/src/tools/lowered/check_core_call_on.rs crates/nikaia-std/src/tools/lowered/check_decls.rs docs/specification/10-nikaia-light.md tests/compiler/tests/a_clone_bound_copies_a_number_and_a_struct.nika tests/compiler/tests/a_clone_bound_copies_a_number_and_a_struct.stdout tests/compiler/tests/a_task_handle_is_not_clone.nika tests/compiler/tests/a_task_handle_is_not_clone.stderr tests/compiler/tests/a_cloned_generic_parameter_is_not_kept.stderr
Commit: Clone is a trait of the language, so [T: Clone] can be written
Body: `Clone` joins `Eq` and `Ord` as a bound the language answers (types.nika STANDARD_BOUNDS, so no NK1135). A `.clone()` on a type parameter bound by `Clone` is typed as that parameter (check_core_call_on_cloned_parameter, before NK1126). At a call, CheckComparable::clones answers the bound: a type parameter where its own bounds hold `Clone`; TaskHandle, task::ChildHandle, Locked and SharedMut never (they have no `.clone()` today, NK1171, and TaskHandle/ChildHandle are not Clone in the runtime); a tuple, option, container, declared struct or enum where its parts do; otherwise yes, as `.clone()` on a concrete type does; a miss is NK1164 (compare_standard_bounds). A program that declares its own `Clone` trait keeps it. The NK1126 help now names the bound (`[T: Clone]` for `.clone()`, a trait declaring the method otherwise, the concrete type for a field). Part I 4.7 gets the sentence on `Clone`. The lowering needed no change: bounds pass through, `fn id<T: Clone>(x: T) -> T { x.clone() }`; EXPECTED.txt is unchanged (prose only). Both steps are in this one commit. No Rust written by hand.
Fixes: #1245
Tests: the two new tests fail on the old binary (NK1135, NK1126) and pass now (prints `3`/`x`; NK1164 for a handle and a struct holding one); a_cloned_generic_parameter_is_not_kept.stderr re-blessed (new NK1126 help); tests/compiler --both-settings filters clone, bound, generic, trait, NK1126, type_parameter: 47 ok (4 that spawn target/fast/nikaia by path failed with the scratch copy and passed run with target/fast/nikaia, the same build); it generics:: traits:: std_bounds:: shape_bounds:: trait_mut_parameter:: 60 ok; lower-std fixpoint clean; specification example output equals EXPECTED.txt; fmt, clippy, pre-push green
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: `Clone` is a trait of the language: `fn id[T: Clone](x: T) -> T { return x.clone() }` can be written; the language answers the bound for the numbers, text, views, containers and declared types whose parts copy, and a task's handle or a lock passed for it is NK1164 (Part I 4.7, #1245).
Owner: none
Disk freed: 0 (cache 3.6G, 4.3G free on /, cache entry run-bundle/c1ccf85af4afe566 309 MB touched in this run)
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 478949acebd4433472ccbab85516d47f7956b280
