Result: done
Cause: 33ed1119 (on origin/develop, not in the tested HEAD) - test red tests/compiler: a_trait_column_change_warns printed Cargo's "Blocking waiting for file lock" line under parallel builds; the suite ran on a base without 33ed1119's filter line.
Fixed: the same one-line filter of "Blocking waiting for file lock" in a_trait_column_change_warns.nika (identical hunk to 33ed1119, rebases cleanly).
Tests: nikaia test --both-settings --project tests/compiler: 473 passed, 0 failed (was 472/1).
Found: none (the test is timing-sensitive to lock contention; the filter covers it).
Files: tests/compiler/tests/a_trait_column_change_warns.nika
Commit: tests: a_trait_column_change_warns filters Cargo's file-lock line (as 33ed1119 on origin/develop)
Changelog: none
Pushed: local 8debcc6e896320376bb18436695be63dd024d7ce
