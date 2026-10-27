Result: blocked
Files: crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/lowered/check_core_stmt.rs tests/compiler/tests/a_proved_arm_bound_at_an_optional_let.nika tests/compiler/tests/a_proved_arm_bound_at_an_optional_let.stdout
Commit: check: a proved arm bound at a `T?` let or assignment is given up (#893)
Body: `check_core_stmt_owns_whole` now holds for `T?` when it holds for `T`, so a `let`/assignment at a `T?` place takes the `return` path (`narrowed_reads_given_back`): the arm is `b.unwrap()` and the choice is wrapped whole, `Some(if b.is_some() { b.unwrap() } else { c })`, as `return` from a `-> T?` function already wrote it (ADR-382). A `T?` of a view keeps its view. lower-std reached its fixpoint after one build; only lowered/check_core_stmt.rs changed.
Fixes: #893
Pushed: no
Tests: new test a_proved_arm_bound_at_an_optional_let (let, assignment, return; String and a struct with a String field) failed on the unfixed compiler (rustc Mismatched types at 6:37 and 13:28), passes now; nikaia test --both-settings --project tests/compiler 185 passed; it -- narrow nullable handed_over 120 passed; pre-push-check ok; fmt ok; clippy clean
Gate: red range_spelling::no_nika_source_in_the_tree_was_left_on_the_old_meaning
Deviation: not committed: the gate is red (range_spelling, from another lane's file, see Found)
Found: committed develop (HEAD d61fbed) fails range_spelling: crates/rs2nika/convert/src/main.nika:5377 `for d in [repo.clone(), ".", "..", "../..", ...] {` is a `for` head with ".." inside string literals, which the guard's line test takes for an old-spelling range (rs2nika lane, d61fbed)
Needs: fix
Changelog: A proved name at an arm tail of a value bound by a `let` or an assignment at a `T?` place is given up and the choice wrapped in `Some(..)`, as `return` already did, instead of a borrowed `&T` that rustc refused (#893).
Owner: none
Disk freed: 0 (nikaia-project-tests 2.5G, disk 19G free; new entry over 100 MB: run-bundle/65d964130cc06fb2 293 MB)
Rs2nika: none
