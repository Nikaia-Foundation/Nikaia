Result: done
Files: crates/nikaia-std/src/tools/check_core_leaves.nika crates/nikaia-std/src/tools/lowered/check_core_leaves.rs docs/specification/30-nikaia-tooling.md tests/compiler/tests/a_field_of_an_element_read_handed_back.nika tests/compiler/tests/a_field_of_an_element_read_handed_back.stderr tests/compiler/tests/a_field_of_an_element_read_copied.nika tests/compiler/tests/a_field_of_an_element_read_copied.stdout
Commit: check: a field of an element read, handed back by value, is NK1131
Body: The Field arm of check_core_a_lent_place_given_away now asks, when the place is kept and its root is a local whose view_of is an element read (check_core_leaves_list_of_a_read non-null), whether the field's type copies; a field that does not is NK1131, "part of an element of `path`", help `s.inner.clone()`. check_core_leaves_an_element_read_given_away takes the text the help clones and whether it is a part, so the rule stays in one place. Part III's NK1131 entry names the field case. Step 2 of #1057 (`(path.last() ?? return null).inner`, `let x = s.inner; return x`) stays open.
Part: #1057
Tests: before the fix (fast binary at be9c7a6c): row a and map rustc E0507 "Cannot move out of `s.inner`", d prints b, e NK1131; after: a and map NK1131 naming path / m with help `s.inner.clone()`, d prints b, e NK1131 unchanged; a_field_of_an_element_read_handed_back fails on the unfixed compiler (row a/map = its two functions, rustc error); nikaia test --both-settings --project tests/compiler: 463 passed, 0 failed (incl. the two new tests, an_element_read_handed_back_by_value, an_element_read_copied_with_clone); lower-std fixpoint clean (only lowered/check_core_leaves.rs changed, no tools/*.nika caught); build, cargo fmt, clippy -D warnings, pre-push-check.sh green
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A field that does not copy, read off a local bound to an element read (`let s = path.last() ?? return null`) and handed back by value, is now `NK1131` with help `s.inner.clone()`, instead of passing the check and failing in rustc (#1057).
Owner: none
Disk freed: 0 (cache 3.2G, disk 8.7G free; new entries over 100 MB: run-bundle/3f8b93b8 347M, build-time-parsers/768d2772 1027M)
Rs2nika: none
Pushed: local 44e164429e3b01a91b7ee70794aad19b20997bf1
