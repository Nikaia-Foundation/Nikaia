Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs crates/nikaia-std/src/tools/emit_function.nika crates/nikaia-std/src/tools/lowered/emit_function.rs tests/compiler/src/main.nika tests/compiler/nikaia.contracts tests/compiler/nikaia.derived tests/compiler/tests/view_field_of_a_lent_struct_handed_on.nika tests/compiler/tests/view_field_of_a_lent_struct_handed_on.stdout tests/compiler/tests/view_fields_of_for_elements_handed_on.nika tests/compiler/tests/view_fields_of_for_elements_handed_on.stdout tests/compiler/tests/view_fields_of_a_lent_package_struct_handed_on.nika tests/compiler/tests/view_fields_of_a_lent_package_struct_handed_on.stdout
Commit: tiers, emit: a view field of a lent struct is handed on as a view
Body: The refusal came from text_tiers, not the checker. A read of an undecided field counted as owned (tier_at's default), so `Word { text: w.text }` fed Word.text text of its own, which made the field mixed (EitherText) and the checker then refused NK2106. Now a field read put back into the same field adds nothing to it, extending #659's rule (tier_a_name) from names to field reads. A plain text field that only `null` and its own reads go into, in a struct that already holds views, is a view too (decide_tiers), so `tag: w.tag` copies. emit_function treats a struct result of the parameter's own type as source_nested (`fn copy<'r>(n: &Node<'r>) -> Node<'r>`, as `clone()` is), so copies outlive the list they came from (Probe C, yaml's load). An owned or mixed field is still refused NK2106. lowered/project_members.rs HashMap/TrustedMap drift is not mine and was left out.
Part: #928
Fixes: #1063
Tests: probes A, B and C failed on the unfixed binary (A, B: NK2106; C: NK1106 cascade, NK2106, NK1127); after the fix A and B print 1 hello and C's load test passes. tests/compiler --both-settings: 470 passed; the one failure was the new package test's .stdout, left empty and then filled with its real output. let_lent_field_kept and let_moved_ref_parameter unchanged. yaml package 14/14, fmt::every_nika_file ok, clippy (1.97) clean, pre-push-check ok
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A `String` field that holds a view can be copied from a borrowed struct (a `ref` parameter or a `for` element) into a new struct, which views the same buffer and can outlive the list it was copied from.
Owner: none
Disk freed: 0
Rs2nika: none
Pushed: local ac156203daa7bbadbf02f729e50d3eb33a4d51e4
