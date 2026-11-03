Result: done
Files: crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/lowered/check_core_flow.rs tests/compiler/tests/inner_match_takes_arm_bound_part.nika tests/compiler/tests/inner_match_takes_arm_bound_part.stdout
Commit: A match arm that keeps a non-copy part by value takes its subject, so an outer arm binds that name by value
Body: check_core_flow_arm now calls check_core_flow_subject_taken: if the subject is a plain name, not a view, not bound by reference and not a mut parameter, and the arm binds a part that does not copy and that the arm does not lend, the subject is handed over (hands_over_name, "taken apart by `match`"). The outer arm's parts_lent then sees the name taken and binds it by value: `Ran::Hit(wall) => match wall { .. }` lowers without `ref wall` / `match *wall` (E0507 before). As a side effect, matching the same owned name twice where the first match moves a part out is now refused with NK2105 instead of reaching rustc's E0382.
Fixes: #1158
Pushed: local bd9c647a480df7d5f0b198c684c3bf612f0f1cf2
Tests: new inner_match_takes_arm_bound_part (both settings) - pass, failed on the unfixed binary; it handed_over:: match_on_a_lent_name:: match_patterns:: match_views:: match_on_element:: match_else:: defects_found_running:: nullable:: text_tiers:: typecheck:: selfhosting_defects:: pausing_lambdas:: articles:: - pass (339); tests/compiler --both-settings got through 407 programs before the run was killed, no NK2105 among them; the 6 it failed (a_foreign_impl_keeping_beyond_the_column_is_refused a_library_trait_with_no_impl_runs a_trait_column_change_warns a_trait_column_does_not_depend_on_item_order bounds_expectation_guards_a_proof commit_dry_run) start target/fast/nikaia by path and fail the same way, "no such file", in a scratch project (the first two also with the unfixed binary), so the driver's run decides them; lower-std fixpoint clean; clippy clean; pre-push green
Gate: green
Deviation: the test is in tests/compiler/tests/, where program tests live, not in tests/compiler/ as the brief wrote; the #1158 probe text was not in the brief, so the test is my own probe of the same shape
Found: matching an owned name a second time after a match that moved a part out of it reached rustc (E0382) and was not refused; this change now refuses it with NK2105 (scratch probe q.nika)
Needs: none
Changelog: An inner `match` on a name an outer arm bound, whose arm keeps a part that does not copy, now moves that name, so the program compiles (it was refused by rustc with E0507); a second `match` on a name a `match` already took apart is refused with NK2105.
Cache: run-bundle/3f8b93b88f7f213d 288 MB
Disk freed: 0
Measure: builds=1 clippy_with_tests=0
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
