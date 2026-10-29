Result: done
Files: crates/nikaia-std/src/tools/fmt.nika crates/nikaia-std/src/tools/lowered/fmt.rs tests/compiler/tests/fmt_keeps_a_nested_tuple_closure_whole.nika tests/compiler/tests/fmt_keeps_a_nested_tuple_closure_whole.stdout
Commit: fmt: a closure's tuple pattern is not cut (fix-forward, CI red since 0f9a525)
Body: fmt_sealed now seals a `fn(..)` parameter list and any tuple pattern nested in one (fmt_in_closure_params), so no cut and no trailing comma.
Fixes: none
Pushed: 7c1643dd9eb436eba78a4bb5fa9ad1ecd018efde
Tests: fmt:: (40 incl. the scan) + guard set pass; new test passes; not run against the unfixed formatter (architect measured the failure on project_check.nika); project_check.nika formats, second pass unchanged
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: The formatter no longer cuts a closure's tuple-pattern parameters (`fn((a, _))`) when it breaks a long line.
Owner: none
Disk freed: 0
