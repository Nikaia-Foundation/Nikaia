Result: done
Files: crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/lowered/check_core_stmt.rs tests/compiler/tests/a_proved_arm_bound_at_an_optional_let.nika tests/compiler/tests/a_proved_arm_bound_at_an_optional_let.stdout
Commit: check: a proved arm bound at a `T?` let or assignment is given up (#893)
Body: `check_core_stmt_owns_whole` now holds for `T?` when it holds for `T`, so a `let`/assignment at a `T?` place takes the `return` path (`narrowed_reads_given_back`, ADR-382): the choice is wrapped whole in `Some(..)`. lower-std rewrote only lowered/check_core_stmt.rs.
Fixes: #893
Pushed: 34fe82f3ccc11a147704a0f0ad3a7a4411ccfebd
Tests: tests/compiler both settings 185 passed; it -- narrow nullable handed_over 121 passed (with the guard set); test failed on the unfixed compiler in the earlier report
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A proved name at an arm tail of a value bound by a `let` or an assignment at a `T?` place is given up and the choice wrapped in `Some(..)`, as `return` already did, instead of a borrowed `&T` that rustc refused (#893).
Owner: none
Disk freed: 0
Rs2nika: none
