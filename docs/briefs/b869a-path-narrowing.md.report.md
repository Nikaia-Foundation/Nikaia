Result: #869 steps 1-4 built in two commits: check+lowering 6f72dfad (null test narrows a path of fields, D5 ends it), tests in this one
Files: tests/compiler/tests/a_loop_that_writes_a_path_ends_its_proof.nika tests/compiler/tests/a_loop_that_writes_a_path_ends_its_proof.stderr tests/compiler/tests/a_path_of_fields_narrows_in_each_test_place.nika tests/compiler/tests/a_path_of_fields_narrows_in_each_test_place.stdout tests/compiler/tests/a_ref_mut_self_call_ends_a_path_proof.nika tests/compiler/tests/a_ref_mut_self_call_ends_a_path_proof.stderr tests/compiler/tests/a_sibling_write_keeps_a_path_proved.nika tests/compiler/tests/a_sibling_write_keeps_a_path_proved.stdout tests/compiler/tests/an_assignment_to_a_prefix_ends_a_path_proof.nika tests/compiler/tests/an_assignment_to_a_prefix_ends_a_path_proof.stderr 
Commit: tests: a path of fields narrowed after a null test, and what ends the proof (ADR-382)
Part: #869
Pushed: f8c4bb6f46f7f4aea8c2820f513d678bc9a580bf
Tests: nikaia test --both-settings tests/compiler 146 ok (5 new; the narrowing probe failed NK1125 on the unfixed compiler); it nullable narrow views_kept match_arms + guard set 104 ok; clippy clean
Gate: green
Deviation: steps 1-3 in one commit (the lowered files of both halves come from one compiler); 5th test added: a loop writing the path after the read is NK1102 naming the write
Found: order.nika:147 compared a proved String? path with a String? (refused like a name is); rewritten to also test later.binds; proofs do not cross a lambda/task/overlap/select (wall), so a read there gets the plain NK1102 without a note
Needs: none
Changelog: A path of fields (`p.name`, `self.cache.x`) narrows after a `null` test like a name does; a write to it or a part holding it, a `mut` hand-over, or a loop doing one ends the proof (ADR-382).
Owner: none
Disk freed: none (cache 4.4G, 17G free; new entry run-bundle/65d964130cc06fb2 293 MB)
