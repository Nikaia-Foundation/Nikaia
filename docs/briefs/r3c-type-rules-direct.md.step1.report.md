Result: done
Pushed: local 3a3089b0f5b1630ffb68f738c415425ae8ba2ff2
Tests: tests/compiler --both-settings 538 passed 0 failed; the two new tests failed on the unfixed compiler (Cannot borrow `lhs` as mutable; Mismatched types)
Found: kind 3, fixed: parts an arm takes out of a mut parameter were copied instead of changed in place; a narrowed mut option handed to a mut parameter lost its write-through; a `for mut` over an arm's list did not open the list mut
Changelog: Fixed: parts a taking arm takes out of a `mut` parameter, and a narrowed `mut` option, can be handed on `mut` and changed in place.
Files: crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/check_references.nika crates/nikaia-std/src/tools/lowered/check_core_flow.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/check_references.rs tests/compiler/tests/a_narrowed_mut_option_is_handed_on_mut.nika tests/compiler/tests/a_narrowed_mut_option_is_handed_on_mut.stdout tests/compiler/tests/the_parts_an_arm_takes_out_are_changed_in_place.nika tests/compiler/tests/the_parts_an_arm_takes_out_are_changed_in_place.stdout
Commit: Parts taken out of a mut parameter, and a narrowed mut option, change in place
Part: #125
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
