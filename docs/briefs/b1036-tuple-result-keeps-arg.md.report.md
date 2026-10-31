Result: done
Files: crates/nikaia-std/src/tools/buffers.nika crates/nikaia-std/src/tools/lowered/buffers.rs tests/compiler/tests/a_tuple_result_keeps_its_temporary_argument.nika tests/compiler/tests/a_tuple_result_keeps_its_temporary_argument.stdout
Commit: buffers: a tuple result with a view part keeps the argument made in the call (Fixes #1036)
Body: The contract of `pair` is `(s: String) -> (ref(s) String, i64)`, views `<result>: borrowed`: a tuple with a view part, which `made_arguments` did not count as a result that views an argument. The new `a_result_of_views` takes a view, a struct of views, or a tuple any part of which is one, so the argument made in the call gets a source and a keep. Optional results are not added: `fn first(s: String) -> (String, i64)?` is refused earlier by the checker (NK1104, returning `(ref String, ?)` against `(String, i64)?`), a different cause.
Fixes: #1036
Tests: optional probe `fn first(s: String) -> (String, i64)?` not added: refused before the lowering by NK1104 (other cause); new test a_tuple_result_keeps_its_temporary_argument fails on the unfixed binary (rustc "temporary value dropped while borrowed" at gen:113, twice) and passes fixed; where.nika lists no tests for made_arguments/a_struct_of_views; tests/compiler --both-settings 436 passed 0 failed; guard set 24 passed; clippy clean; pre-push-check ok
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A function returning a tuple with a view of an argument made in the call keeps that argument, instead of rustc's "temporary value dropped while borrowed".
Owner: none
Disk freed: 0 GB (no clone; cache 3.3G, 7.5G free, no new entry over 100 MB)
Rs2nika: none
Pushed: b5b0fb83a656e9ec40f4cadbda317ef66e3ed970
