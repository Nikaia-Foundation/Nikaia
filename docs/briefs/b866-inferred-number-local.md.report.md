Result: done
Files: crates/nikaia-std/src/tools/buffers.nika crates/nikaia-std/src/tools/lowered/buffers.rs tests/compiler/tests/a_view_of_an_inferred_number_local_is_returned_owned.nika tests/compiler/tests/a_view_of_an_inferred_number_local_is_returned_owned.stdout
Commit: A formatted inferred number local or tuple field returned as a view is owned (#866)
Fixes: #866
Pushed: 662fb73096467a16020a026cf77407ea0d8eb417
Tests: new test failed on the unfixed compiler (Cannot return value referencing local `s`), passes fixed; tests/compiler both settings 111 passed; guard set 22 passed
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: to_string of a local with an inferred number type, or of a tuple field bound to a number, is a buffer of its own (#866)
Owner: none
Disk freed: 0
