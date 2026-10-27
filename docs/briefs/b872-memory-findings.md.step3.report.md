Result: done
Files: crates/nikaia-std/src/tools/check_references.nika crates/nikaia-std/src/tools/check_arguments.nika crates/nikaia-std/src/tools/lowered/check_references.rs crates/nikaia-std/src/tools/lowered/check_arguments.rs docs/specification/10-nikaia-light.md tests/compiler/tests/a_for_element_is_lent_to_a_parameter_that_only_reads.nika tests/compiler/tests/a_for_element_is_lent_to_a_parameter_that_only_reads.stdout tests/compiler/tests/a_for_element_is_refused_to_a_parameter_that_keeps.nika tests/compiler/tests/a_for_element_is_refused_to_a_parameter_that_keeps.stderr tests/compiler/tests/a_list_element_past_a_jump_is_refused_where_an_own_value_is_wanted.stderr
Commit: A view handed to a parameter the callee only reads is lent as it is
Fixes: #872
Model: Opus 5.5
Tests: nikaia test --both-settings --project tests/compiler 141 passed; it subsets 299 passed; guard set 22 passed; clippy clean; pre-push-check clean; both new tests failed on the unfixed compiler (NK1102)
Gate: green
Deviation: none (NK1102 kept for the keeping case, with new notes and a `.clone()` help)
Found: none
Needs: none
Changelog: A view (a `for` element of a borrowed list, `v[i]`, a `ref` binding) handed to an owned parameter the body only reads is lent as it is (ADR-094 D1) instead of refused NK1102; where the body keeps the parameter, NK1102 now says it is a view and the callee keeps it, and helps with `.clone()`. Part I 6.6 says so.
Pushed: 4456f1fd19274a63b03b74e4684d4f2ac419a144
