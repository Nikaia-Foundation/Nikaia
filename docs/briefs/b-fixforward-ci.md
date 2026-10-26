# Fix forward: red develop CI (runs 1936-1946)

CI on `develop` fails in job `bootstrap_compiler` (and arm64), step Test, with 5 tests of `-p nikaia --test it`:
- language::the_language_package_passes_its_own_tests
- sequences::the_corpus_has_no_more_unanswered_method_calls_than_it_had
- specification::the_specifications_programs_are_the_ones_in_expected_txt
- nikaia_test::the_compiler_test_package_passes_at_both_settings
- tuple_let::a_tuple_of_names_binds_the_parts

First red commit is around 0.0.1223/0.0.1224 (another session); the #853 literal migration (0c17885c) and #856 (86848a0d) came after. Find which commit broke each test (run them with `cargo test --profile fast -p nikaia --test it -- NAME`), fix forward at the root (compiler, not a workaround), commit only your files, push `git push origin develop`. Report which commit caused what. Do not touch version lines.
