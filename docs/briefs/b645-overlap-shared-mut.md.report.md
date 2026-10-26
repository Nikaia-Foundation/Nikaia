Result: done
Files: crates/nikaia-std/src/tools/check_walks.nika crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/lowered/check_walks.rs crates/nikaia-std/src/tools/lowered/check_core_flow.rs tests/compiler/tests/overlap_one_changes_one_reads.nika tests/compiler/tests/overlap_one_changes_one_reads.stderr tests/compiler/tests/overlap_two_branches_change_one_binding.nika tests/compiler/tests/overlap_two_branches_change_one_binding.stderr tests/compiler/tests/overlap_two_branches_read_one_binding.nika tests/compiler/tests/overlap_two_branches_read_one_binding.stdout
Commit: Refuse two overlap branches where one changes a binding the other names (NK2104)
Part: #645
Tests: compiler project both settings 120 ok; it overlap:: + guard set 36 ok; clippy clean
Gate: green
Deviation: the new tests were not run against the unfixed compiler (no stash rebuild); the brief records the probe as accepted by check before
Found: none
Needs: none
Changelog: An overlap whose branches meet on a changed binding is refused with NK2104
Owner: none
Pushed: 3ec1f1ef8419e9379f471f1d0592fa573c2501ad
Pushed: yes
Disk freed: 0 (cache kept)
