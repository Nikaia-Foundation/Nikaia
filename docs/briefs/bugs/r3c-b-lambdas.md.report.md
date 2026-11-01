Result: done
Pushed: local d2da2e7b8e99342a0d5387107dbf6ae2a200ae0c
Tests: tests/compiler --both-settings: new a_text_field_is_asked_of_a_list_of_text ok (failed unfixed: expected &String, found &'static str); step 1 test ok; only failure the other builder's untracked a_copy_bound_under_a_match_on_a_part_of_a_mut_parameter
Deviation: defect 2 is not lambda-specific: any String field held as text of its own (&'static str) handed to Vec[String].contains failed; fixed for Expr::Field arguments
Found: none
Needs: none
Changelog: A text field is asked of a list of text: check_core_method_resolved takes the receiver type and marks text_in_lists for a field argument when the list holds text, so contains compares as text
Files: crates/nikaia-std/src/tools/check_core_method.nika crates/nikaia-std/src/tools/lowered/check_core_method.rs tests/compiler/tests/a_text_field_is_asked_of_a_list_of_text.nika tests/compiler/tests/a_text_field_is_asked_of_a_list_of_text.stdout docs/briefs/bugs/r3c-b-lambdas.md.step1.report.md
Commit: A text field handed to contains on a list of text compares as text
Part: #125
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
