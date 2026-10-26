Result: done
Files: crates/nikaia-std/src/tools/buffers.nika crates/nikaia-std/src/tools/lowered/buffers.rs tests/compiler/tests/a_view_of_a_formatted_local_is_returned_owned.nika tests/compiler/tests/a_view_of_a_formatted_local_is_returned_owned.stdout
Commit: A view of a formatted number local is returned owned (#865)
Fixes: #865
Tests: tests/compiler both settings 110 ok; guard set 22 ok; new test failed before fix (len, n+1, true, g() refused by rustc)
Gate: green
Deviation: tuple field `p.0.to_string()` not fixed (type of a field is unknown here; a field may be text, #851 area), left out of the test
Found: `p.0.to_string()` of a tuple of numbers local still fails rustc; needs types of inferred locals
Needs: none
Changelog: A view of the `.to_string()` of a number expression (`len()`, arithmetic, `bool`, a call returning a number) may leave the function.
Owner: none
Disk freed: 0
Pushed: 635b59f65ed1cdfcb73d6957d759258bdd727073
