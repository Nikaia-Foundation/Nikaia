Result: done
Files: crates/nikaia-std/src/tools/check_core_call_on.nika crates/nikaia-std/src/tools/lowered/check_core_call_on.rs tests/compiler/tests/a_bound_method_hands_back_the_type_parameter.nika tests/compiler/tests/a_bound_method_hands_back_the_type_parameter.stdout tests/compiler/tests/a_bound_method_takes_the_type_parameter_as_self.nika tests/compiler/tests/a_bound_method_takes_the_type_parameter_as_self.stdout
Commit: Self of a bound's method is the type parameter
Body: A method call on a type parameter answered by a bound (check_core_call_on_undescribed, bound_that_answers) was typed with the trait method's ledger signature as written, so `Self` stayed `Self`: `x.twin()` returned `Self` (NK1104) and `x.same(y)` wanted `other: Self` (NK1102). The bound path now resolves `Trait::method`, clones its contract with every `Self` in the parameters and the result named by the type parameter (check_core_call_on_self_signature), and types the call as before. No Rust changed by hand.
Fixes: #1244
Tests: the two new tests failed on the old binary (NK1104, NK1102) and print `3` / `true false` now; tests/compiler --both-settings filters a_bound_method, bound, trait, clone, generic 44 ok; it generics:: traits:: std_bounds:: trait_mut_parameter:: shape_bounds:: 60 ok; lower-std fixpoint clean; fmt, clippy green
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: `Self` of a bound's method is the type parameter: in `fn id[T: Twin](x: T) -> T`, `x.twin()` declared `-> Self` hands back a `T`, and a parameter `other: Self` takes a `T` (Part I 4.7, #1244).
Owner: none
Disk freed: 0 (cache 2.8G, 3.5G free on /, no new cache entry over 100 MB)
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local d1d97c496622767a200601d357ed6c036fd43c9c
