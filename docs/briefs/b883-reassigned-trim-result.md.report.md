Result: done
Files: crates/nikaia-std/src/tools/buffers.nika crates/nikaia-std/src/tools/lowered/buffers.rs tests/compiler/tests/string_reassigned_trim_result.nika tests/compiler/tests/string_reassigned_trim_result.stdout
Commit: keeps: a String local handed a kept local's view whole is not put in the keep again (Fixes #883)
Fixes: #883
Pushed: rejected
Tests: string_reassigned_trim_result (issue probe, untyped let, return st.trim()) failed on the unfixed compiler (rustc Mismatched types, all three), passes with the fix; tests/compiler --both-settings 157 passed, 1 failed (bytes_iterate_and_compare_their_ends, another agent's uncommitted test); it keep/view/move + guard set 366 passed; clippy, fmt, pre-push-check clean; `mut head` parameter assigned a literal local is refused by NK1105 (checked, not a rustc refusal)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: a String local assigned whole from another kept String local, whose trimmed view is the result, compiles (#883)
Owner: none
Disk freed: 0
