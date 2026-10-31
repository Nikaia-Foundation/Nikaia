Result: done
Files: crates/nikaia-std/src/tools/check_core_ops.nika crates/nikaia-std/src/tools/lowered/check_core_ops.rs tests/compiler/tests/a_field_of_a_coalesce_is_read_through_a_view.nika tests/compiler/tests/a_field_of_a_coalesce_is_read_through_a_view.stdout
Commit: check: an optional field read off a ?? result is a view of the element (Fixes #1055)
Body: check_core_ops_behind_a_view now counts a field whose base is a `get` handed back past a jump (`check_core_stmt_got`, the same test that makes row d's `let c = m.get(k) ?? return null` a loan) as behind a view, so the outer `?? return null` is recorded in lent_coalesces and lowers to `.as_ref()`. That fact was chosen over map_read_views because the list `get` (row b) does not set read_a_map, so map_read_views holds for maps only.
Fixes: #1055
Tests: before the fix, rows a, b and c were refused by rustc with E0507 (the new test program run on the unfixed binary), and d and e ran. After it, all rows print as expected (one none one none dflt none one lbl). New test a_field_of_a_coalesce_is_read_through_a_view; tests/compiler --both-settings: 461 passed, 0 failed. where.nika names no tests. lower-std fixpoint: only lowered/check_core_ops.rs changes from this edit (lowered.rs, project_members.rs and grammar_run.rs are rewritten because of another agent's untracked grammar_run.nika, not staged). clippy clean, pre-push-check ok.
Gate: green
Deviation: none
Found: none
Needs: cargo fmt --check shows a diff in crates/nikaia/src/grammar_run.rs (another agent's uncommitted file, not touched)
Changelog: An optional field read off the result of a `??` that jumps past a missing list or map element is now a view of the element, and no longer moved out of it.
Owner: none
Disk freed: 0 (cache 3.1G, 11G free)
Rs2nika: none
Pushed: local 4de39626f8f6a1f96d4784c1bb4b089bdbb418a1
