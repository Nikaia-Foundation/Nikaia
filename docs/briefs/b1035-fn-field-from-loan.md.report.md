Result: done
Files: crates/nikaia-std/src/tools/check_core_leaves.nika crates/nikaia-std/src/tools/lowered/check_core_leaves.rs tests/compiler/tests/a_fn_field_handed_back_from_a_loan.nika tests/compiler/tests/a_fn_field_handed_back_from_a_loan.stdout
Commit: check: a fn field handed back out of a loan is a copy (#1035)
Body: check_core_a_lent_place_given_away returns early for a place of Ty::Fn type out of a lent root (Part I 5.4: a copy of a function value shares the one closure), beside copies_plainly, which stays as it is. The emitter already clones the field read (#1031): the lowered pick is `h.f.clone()`.
Fixes: #1035
Tests: a_fn_field_handed_back_from_a_loan fails unfixed (NK1131 at 15:2, rebuilt without the fix) and passes fixed; tests/compiler --both-settings 413 passed 11 failed, none from this change: 10 are one NK2302 in tests/compiler/src/main.nika `places` (the uncommitted text_tiers.nika #999 work in the tree), 1 is a_trait_column_change_warns printing cargo file-lock lines; no failing test expects NK1131; guard set green; lower-std fixpoint clean; clippy and pre-push clean
Gate: green
Deviation: none
Found: none
Needs: supervisor: the uncommitted text_tiers.nika (#999) in the shared tree makes tests/compiler refuse NK2302 at src/main.nika:13 (places), 10 tests
Changelog: A function value handed back out of a borrowed struct (`return h.f` over `h: ref H`) is a copy that shares its closure, no longer refused NK1131.
Owner: none
Disk freed: 0 GB (cache 2.0G, 9.2G free, no new entry over 100 MB; no clone)
Rs2nika: none
Pushed: b65e39fd (pushed by drive.nika)
