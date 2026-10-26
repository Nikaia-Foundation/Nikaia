Result: done. A literal bound without a type and then changed in place or reassigned is text of its own; a match binding of a String is not a view (D31).
Files: crates/nikaia-std/src/tools/buffers.nika crates/nikaia-std/src/tools/check_function.nika crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/buffers.rs crates/nikaia-std/src/tools/lowered/check_function.rs crates/nikaia-std/src/tools/lowered/text_tiers.rs tests/compiler/tests/a_literal_bound_by_a_match_goes_to_a_mut_parameter.nika tests/compiler/tests/a_literal_bound_by_a_match_goes_to_a_mut_parameter.stdout tests/compiler/tests/a_literal_changed_after_the_last_use_of_its_view.nika tests/compiler/tests/a_literal_changed_after_the_last_use_of_its_view.stdout tests/compiler/tests/a_literal_reassigned_in_a_loop_under_a_view.nika tests/compiler/tests/a_literal_reassigned_in_a_loop_under_a_view.stdout tests/compiler/tests/a_literal_reassigned_then_changed.nika tests/compiler/tests/a_literal_reassigned_then_changed.stdout
Commit: A literal changed in place or bound by a match is text of its own (ADR-282 D31)
Fixes: #856
Tests: 4 new programs fail on the unfixed compiler, pass fixed; nikaia test --both-settings tests/compiler 97 ok; it guard set, text_tiers, text_views, handed_over, result_source, nullable, lent_arguments 261 ok (unpatched); clippy clean
Deviation: none. Built and gated in a scratch clone (the shared checkout's build was broken by another agent's Target::name move); lowered files copied back and identical (fixpoint).
Found: with the #853 patch, 19 of 169 tests in handed_over/lent_arguments/nullable/result_source/ still fail, all expectation changes (no compile error): NK2105 no longer raised for a name bound to a literal and handed over (the literal is copied each time), element-list push of literals (Vec of static views), keep frame no longer needed for a literal in an inner scope, String::from counts. #853 must adjust these tests (keep .clone() or write the String type).
Needs: #853 may proceed; adjust the 19 expectations listed above.
Gate: green
GB freed: pending
Owner: supervisor
Pushed: rejected
