Result: done
Files: crates/nikaia-std/src/tools/check_core_expr.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/check_flow.nika crates/nikaia-std/src/tools/check_reads.nika crates/nikaia-std/src/tools/lowered/check_core_expr.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/check_flow.rs crates/nikaia-std/src/tools/lowered/check_reads.rs crates/nikaia-std/src/tools/lowered/fold_bound.rs tests/compiler/tests/a_returned_map_read_of_literal_text_is_copied.nika tests/compiler/tests/a_returned_map_read_of_literal_text_is_copied.stdout
Commit: check: a returned map read of literal text is copied (#996)
Body: The tier pass marks a map of literal text `BTreeMap[ref String, ref String]` (`&'static str`), so its read is a view of a view (`Option<&&str>`), and `a_map_read_kept` returned early on any view. Now the checker records a map read whose element is a view (`flow.view_reads`, set where `map_read_sites` is) and copies it out (`copied_reads`, ADR-293 D15) where a `T?` of the view is kept (return, typed let, argument) and at an untyped `let`. This covers `m.get(k)`, `let r = m.get(k); return r` and the bracket form `m[k]`, which failed the same way, so the test covers it too. An owned `String` value is still refused with NK1102. The untyped-let copy changes one lowered tools file: `fold_bound.rs` now writes `self.rules.get(&name).copied()`, which compiles.
Fixes: #996
Pushed: e7caa69c (pushed by drive.nika)
Tests: new test a_returned_map_read_of_literal_text_is_copied prints two/two/two; on the unfixed compiler the method forms lowered as Option<&&str> (the brief's E0308), and `return m[k]` failed rustc with Mismatched types on an intermediate build; p6 (owned value) still refuses NK1102; lower-std fixpoint clean; gate: clippy clean, guard set 23 passed, tests/compiler --both-settings 367 passed
Gate: green
Deviation: none
Found: none
Needs: rebase and push e7caa69c (push rejected, fast-forward only)
Changelog: A map of literal text handed back as `String?` (`return m.get(k)`, `return m[k]`, or through a `let`) copies the read out instead of reaching rustc (#996).
Owner: none
Disk freed: 0.02 GB (scratch binary copy); cache 4.0G, disk 3.5G free, no new cache entry over 100 MB
Rs2nika: none
