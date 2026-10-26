Result: done
Files: crates/nikaia/src/parser/mod.rs crates/nikaia/tests/it/literals.rs crates/nikaia/tests/it/parse_error.rs crates/nikaia/tests/it/parser.rs docs/specification/10-nikaia-light.md tests/compiler/tests/a_bang_after_a_name_is_not_a_second_statement.nika tests/compiler/tests/a_bang_after_a_name_is_not_a_second_statement.stderr tests/compiler/tests/a_statement_may_follow_its_brace.nika tests/compiler/tests/a_statement_may_follow_its_brace.stdout tests/compiler/tests/two_statements_on_one_line_are_refused.nika tests/compiler/tests/two_statements_on_one_line_are_refused.stderr
Commit: A statement begins a line; two on one line are a parse error
Part: #655
Pushed: 9936c7046c8f55ee7f7c25a4050273941175f157
Tests: project tests 125 ok both settings; it parse/guards 76 ok; clippy clean; new tests not run against unfixed compiler (the programs parsed before by the issue's probe)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
