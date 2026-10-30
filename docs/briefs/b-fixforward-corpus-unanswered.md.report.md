Result: done
Files: crates/nikaia-std/src/tools/check_arms.nika crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/check_arms.rs crates/nikaia-std/src/tools/lowered/check_core_flow.rs crates/nikaia-std/src/tools/lowered/project_members.rs crates/nikaia/tests/it/sequences.rs tests/compiler/tests/match_tuple_of_views.nika tests/compiler/tests/match_tuple_of_views.stdout
Commit: check: arms meet where each is known, and a match over a tuple of views binds as over one (fix forward: corpus unanswered calls)
Body: 43312d30 raised the corpus count to 18: prover_bound_number's `let (called, generics, args) = match ..` hands `generics.clone()` beside `[]`, and the arm meet compared the types exactly, so `Vec[TypeRef]` beside a `Vec` of nothing typed left `generics` unknown (`generics.len()`, `generics[0].generics.is_empty()`). Arms now meet part by part where each is known (`arms_meet_with_holes`, check_arms.nika), after text and null; an arm of no known type as a whole is still the position's to settle, so no refusal moves (NK1245 stays where it was). That answers four calls: 14, ceiling lowered with its sentence. Writing the helper as `match (a, b)` over two views met a lowering defect: names bound inside a view part of a tuple were written `ref x` (refused under implicit borrowing) and a `bool` inside was not copied out (`&bool`); the checker now copies such parts out and leaves them out of `lent_bindings`, as over one view (check_core_flow.nika), test match_tuple_of_views. lowered/project_members.rs is the re-lower diff of the current compiler (HashMap to TrustedMap); none of its forms is one this change touches, so the committed file was stale.
Fixes: none
Pushed: ff9696e4 (pushed by drive.nika)
Tests: sequences:: (14) ok; corpus count 18 before, 14 after; match_tuple_of_views failed on the unfixed compiler (probe, rustc mismatched types) and passes; tests/compiler --both-settings 369 ok; examples:: not run
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: check: arms that differ only where one is not yet known meet at the known type; a match over a tuple of views binds the parts inside each view by reference and copies numbers and bools out
Owner: none
Disk freed: 0 GB (cache 4.0G, 3.4G free on /; no own clone or scratch target; no new cache entry over 100 MB)
Rs2nika: none
