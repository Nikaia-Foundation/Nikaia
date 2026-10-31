Result: done
Files: crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/lowered/check_core_stmt.rs tests/compiler/tests/an_untyped_let_lambda_in_a_fn_field.nika tests/compiler/tests/an_untyped_let_lambda_in_a_fn_field.stdout
Commit: check: an untyped let lambda put into a fn field keeps its error set
Body: check_core_stmt_let keeps the stored-code frame of a `let` whose value is a lambda whatever type the checker gave it (an untyped lambda no call hands on is not Ty::Fn there), so check_core_places_stored records the lambda's free calls under the name and a field given the name resolves instead of being unresolved. The test holds row a (`x + 1`) and a second infallible body (`twice(x)`) in place of row b: row b (`fn(x) { 5 }`) now reaches NK1245, a type nothing tells, which it reaches on the unfixed compiler too once the NK2605 is out of the way (`let k = h.f; k(1)`), since #997's typing by use sees only call arguments, not a struct field.
Fixes: #1045
Tests: new test passes in tests/compiler both settings (437 passed); before the fix the old binary refused it NK2605 "`H.f` can fail" at 15:5 and at its second row alone; rows before: a NK2605, b NK2605, c 2, d 2, e 2; after: a 2, b NK1245, c 2, d 2, e 2; a lambda calling a throwing fn still NK2605 at the lambda; let_lambda_string_param, a_let_lambda_handed_where_a_function_is_kept ok; where.nika lists no tests for the three functions; guards 24 passed (sequences guard green); clippy clean; lower-std fixpoint clean
Gate: green
Deviation: step 2's test holds row a and a twice(x) row, not row b, which is refused NK1245 for a reason apart from #1045 (see Body)
Found: none
Needs: none
Changelog: An untyped lambda bound by `let` and put into a function field no longer makes a call through the field refused NK2605.
Owner: should #997's typing of an untyped lambda by its first use also take a function field it is put into (`H { f: g }`), so `let g = fn(x) { 5 }` there runs instead of NK1245?
Rs2nika: none
Disk freed: 0 GB (cache 3.3G, disk 7.5G free; no new entry over 100 MB; no clone)
Pushed: a79b8cd8 (pushed by drive.nika)
