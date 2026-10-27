Result: done
Files: crates/nikaia-std/src/tools/check_arm_leaves.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/lowered/check_arm_leaves.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs tests/compiler/tests/a_proved_value_bound_by_let_is_given_up.nika tests/compiler/tests/a_proved_value_bound_by_let_is_given_up.stdout
Commit: check: a proved name bound whole by a `let` or assignment that owns it is given up (#888)
Body: The value of a `let` whose bound type is owned whole (no view, no option, not unknown; no element view, deref view or bytes view) and the value assigned to such a place go through `narrowed_reads_given_back`, as `return` and the tail do: a proved `b` in an arm (or bound directly, `let s: String = b`) is `b.unwrap()`, text and structs alike. A local the checker makes a view because the source is read later keeps today's behaviour (NK1103 / NK1105, as before). `a_narrowed_read_given_back` is now idempotent (bit 8 is not added twice, which would have read as the `as_mut` bit 16). A later read of a directly bound proved name is NK2105, as for a plain `String` local.
Fixes: #888
Pushed: 3b77d7c2123ed153fe0ed90e68a135e422780495
Tests: new test a_proved_value_bound_by_let_is_given_up (its forms p1/p2/p3/direct let/struct failed in rustc on the unfixed binary as probes, "Mismatched types"; controls match/clone ran); nikaia test --both-settings --project tests/compiler; guard set; lower-std changed only the lowered files of the two edited tools files (no tools file used the form)
Gate: green
Deviation: none
Found: a `String?`-typed let from a proved arm, `let s: String? = if b != null { b } else { c }`, fails in rustc (Mismatched types), before and after; a plain struct moved in an `if` arm and read later (`let s: P = if c { t } else { … }; println(t.name)`) passes the checker and fails in rustc (Borrow of moved value), and the proved form does the same
Needs: none
Changelog: A name proved not `null` and bound whole by a `let`, or assigned to a place that owns it, is the value itself, so `let s: String = if b != null { b } else { "none" }` compiles (#888).
Owner: none
Disk freed: 0 (no clone; cache 2.5G, disk 19G free, no new entry over 100 MB)
