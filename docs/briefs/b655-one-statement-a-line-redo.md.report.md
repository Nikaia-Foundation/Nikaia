Result: done
Files: assets/badges/self-hosted.svg crates/nikaia/src/parser/mod.rs crates/nikaia/tests/it/anonymous_constructors.rs crates/nikaia/tests/it/block_comments.rs crates/nikaia/tests/it/build_time.rs crates/nikaia/tests/it/catch_handler.rs crates/nikaia/tests/it/comptime.rs crates/nikaia/tests/it/contracts.rs crates/nikaia/tests/it/else_if.rs crates/nikaia/tests/it/errors_lowering.rs crates/nikaia/tests/it/expressions.rs crates/nikaia/tests/it/four_words_freed.rs crates/nikaia/tests/it/grammar_actions.rs crates/nikaia/tests/it/ignore_pattern.rs crates/nikaia/tests/it/jump_expressions.rs crates/nikaia/tests/it/jumps.rs crates/nikaia/tests/it/lent_arguments.rs crates/nikaia/tests/it/literals.rs crates/nikaia/tests/it/manifest.rs crates/nikaia/tests/it/mut_bindings.rs crates/nikaia/tests/it/number_literal.rs crates/nikaia/tests/it/options_only.rs crates/nikaia/tests/it/overlap.rs crates/nikaia/tests/it/parse_error.rs crates/nikaia/tests/it/parser.rs crates/nikaia/tests/it/stored_views.rs crates/nikaia/tests/it/tasks.rs crates/nikaia/tests/it/touch_sets.rs crates/nikaia/tests/it/typecheck.rs docs/specification/10-nikaia-light.md tests/accepted/738_lent_element_handed_back.nika tests/accepted/738_lent_field_mixed_result.nika tests/compiler/tests/a_bang_after_a_name_is_not_a_second_statement.nika tests/compiler/tests/a_bang_after_a_name_is_not_a_second_statement.stderr tests/compiler/tests/a_statement_may_follow_its_brace.nika tests/compiler/tests/a_statement_may_follow_its_brace.stdout tests/compiler/tests/two_statements_on_one_line_are_refused.nika tests/compiler/tests/two_statements_on_one_line_are_refused.stderr tests/errors/EXPECTED.txt tests/language/src/error_types.nika tests/language/src/self_hosting.nika tests/specification/EXPECTED.txt 
Commit: A statement begins a line; two on one line are a parse error (Part of #655)
Part: #655
Pushed: f4dfb8ce272941a74f0bfebdf40e28ecf15dce57
Tests: whole it binary 2886 ok after fixes (75 failed first; 1 more fixed after); tests/compiler --both-settings 129 ok; guard set ok; clippy clean
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: a statement begins a line; two on one line are a parse error (Part I 3.1).
Owner: none
Disk freed: 0 GB (shared checkout, target kept)
