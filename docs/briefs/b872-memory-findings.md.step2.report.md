Result: done
Files: crates/nikaia-std/src/tools/check_exprs.nika crates/nikaia-std/src/tools/check_arguments.nika crates/nikaia-std/src/tools/lowered/check_exprs.rs crates/nikaia-std/src/tools/lowered/check_arguments.rs docs/specification/10-nikaia-light.md docs/specification/30-nikaia-tooling.md tests/compiler/tests/a_struct_handed_to_println_is_refused.nika tests/compiler/tests/a_struct_handed_to_println_is_refused.stderr tests/compiler/tests/a_struct_field_and_an_error_are_printed.nika tests/compiler/tests/a_struct_field_and_an_error_are_printed.stdout
Commit: A value with no text form handed to a print call is NK1201, as in an f-string hole
Fixes: #873
Model: Opus 5.5
Tests: nikaia test --both-settings --project tests/compiler 139 passed; it accepted_compiles accepted_sweep examples file_paths literals error_types errors fmt duration error_sums diagnostics 174 passed; guard set 22 passed; clippy clean; refusal test failed on the unfixed compiler (only the f-string hole was refused, println(ada) reached rustc)
Gate: green
Deviation: NK1201 reused (the existing no-text-form code of f-string holes), not NK1302/NK1303 or NK1102
Found: none
Needs: none
Changelog: `print`/`println`/`eprint`/`eprintln` of a value with no text form (a struct or enum of the program that is not an error, a list, map, set, range, tuple, SharedMut/Locked) is refused NK1201 before lowering, as `f"{x}"` already was; the help names the fields to print. Part I 2.5 says so; Part III NK1201 row extended.
Pushed: aeb02ace929631174bdcfb482e11c465ff73cadc
