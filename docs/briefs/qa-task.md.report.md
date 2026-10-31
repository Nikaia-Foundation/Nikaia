Result: done (nothing to commit: the red was already fixed upstream)
Cause: f2292c44 (pushed, #1049 made text(..) of a lone sequence emit as its group); steps examples::every_example_package_passes_its_own_tests and grammar_lowering::a_type_argument_is_written_with_brackets_and_emitted_with_angles were red before it. qa-input.txt lists no unpushed commits and cuts the failing list at "a_p...".
Fixed: nothing by me; f2292c44 changed the test's expected `t:text((alpha1 digit*))`, which is what #1049 meant.
Tests: with --profile fast, `grammar_lowering::` and `examples::` (39 tests) all pass, including both named failures. I could not see the third name cut at "a_p..." but the whole grammar_lowering module is green.
Found: `cargo test -p nikaia` in the dev profile (opt-level 1) fails to compile the lib: "reached the recursion limit while instantiating <closure as Parser>::parse_next" (winnow-grammar rt.rs:902, winnow impls.rs:578). The fast profile builds. Probe: `cargo test -p nikaia --test it -- grammar_lowering::a_type_argument`. Suspect the grammar in crates/nikaia/src (recursion_limit = 256 in lib.rs); not touched here.
Files: none
Commit: none
