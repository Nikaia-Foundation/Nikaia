Result: done
Cause: bbca2268 (v18a) arm64 bootstrap_compiler: grammar_lowering::a_par_fold_entry_is_entered_from_another_package (merge named in a grammar's par_fold stayed lent, rustc mismatched types)
Cause: bbca2268 (v18a) arm64: lent_arguments::a_method_argument_is_not_lent and result_source::a_view_of_a_parameter_taken_by_value_in_a_method (tests expected the old "methods never lend"; the commit changed that on purpose)
Fixed: keeps_named_as_values now walks the fold expressions of grammar rules (fc2b95da, lower-std fixpoint holds); the two tests now state the D1 behaviour (b0cd9ce6)
Tests: it filters lent_arguments result_source grammar_lowering keeps parallel_folds stored_views accepted_compiles contracts:: 332 ok; clippy, fmt, pre-push green; lower-std twice, second changes nothing
Found: a method's `b: String` returned as `ref b` is no longer NK2302 ("takes by value"); the free function never was, and nothing else in the tree covers that message. Probe: `impl S { fn get(ref self, b: String) -> ref String { return ref b } }` checks clean.
Files: crates/nikaia-std/src/tools/keeps.nika crates/nikaia-std/src/tools/lowered/keeps.rs crates/nikaia/tests/it/lent_arguments.rs crates/nikaia/tests/it/result_source.rs
Commit: fc2b95da Keep by value a method a grammar's par_fold names as its merge; b0cd9ce6 Two tests say what an own method's lent parameter means (ADR-views-3 D1)
