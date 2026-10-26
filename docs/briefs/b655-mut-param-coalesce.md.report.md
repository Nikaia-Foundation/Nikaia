Result: done
Files: crates/nikaia-std/src/tools/check_core_ops.nika crates/nikaia-std/src/tools/lowered/check_core_ops.rs tests/compiler/tests/a_mut_parameters_field_past_a_jump_is_lent.nika tests/compiler/tests/a_mut_parameters_field_past_a_jump_is_lent.stdout tests/compiler/tests/a_mut_parameters_view_survives_a_write_to_another_field.nika tests/compiler/tests/a_mut_parameters_view_survives_a_write_to_another_field.stdout
Commit: check: a field of a mut parameter read through ?? return is lent (#655)
Fixes: none
Part: #655
Pushed: 2003b4caaec6f4350b6b57f9ef4732e5a4d302c0
Tests: project tests 122 ok both settings; it coalesce/lent/view + guard set 272 ok; second test (write to count after the view) compiles and prints 2, no refusal needed; not run against the unfixed compiler (stash not rebuilt; architect reproduced the E0507 on the probe)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A field of a `mut` parameter read through `?? return` is lent, not moved (rustc E0507).
Owner: none
Disk freed: 0
