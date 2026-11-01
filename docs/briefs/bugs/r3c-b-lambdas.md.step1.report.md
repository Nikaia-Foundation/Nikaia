Result: done
Pushed: local a7d64f715811b0caf843a08502b5e019ad2c5f59
Tests: tests/compiler --both-settings: new a_variant_part_of_code_keeps_a_lambda_or_a_named_function ok (failed unfixed: Mismatched types, Kept<dyn Fn> vs closure/fn item); only failure the other builder's untracked a_copy_bound_under_a_match_on_a_part_of_a_mut_parameter
Deviation: none
Found: none
Needs: none
Changelog: A variant part of code keeps a lambda or a named function: check_core_call_variant_parts now calls check_core_a_kept_lambda per part, as a struct field and an assignment do
Files: crates/nikaia-std/src/tools/check_core_call.nika crates/nikaia-std/src/tools/lowered/check_core_call.rs tests/compiler/tests/a_variant_part_of_code_keeps_a_lambda_or_a_named_function.nika tests/compiler/tests/a_variant_part_of_code_keeps_a_lambda_or_a_named_function.stdout
Commit: A variant part of code keeps the lambda or named function it is built from
Part: #125
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
