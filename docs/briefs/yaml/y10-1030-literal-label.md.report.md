Result: done
Files: crates/nikaia-std/src/tools/check_grammar.nika crates/nikaia-std/src/tools/lowered/check_grammar.rs crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs docs/specification/20-nikaia-advance.md tests/compiler/tests/grammar_a_label_on_a_literal_binds_its_text.nika tests/compiler/tests/grammar_a_label_on_a_literal_binds_its_text.stdout
Commit: grammar: a label on a literal binds its text, also when returned or stored
Body: `CheckState::pattern_type` gets a `Pattern::Literal` arm (the matched text, a view), and `grammar_pattern_kinds` in `text_tiers.nika` the same arm (`view_kinds()`): with the checker's arm alone the checker refused the probe with NK1104/NK1106, since the result-tier pass still lowered the rule `-> String` owned; with both, a rule returning a literal capture lowers as `-> &str`, as one returning a `text(..)` capture does, and a choice of literals stored in a `String` field makes the field a view. Part II 10.8, the `"text"` row's yields cell now reads `nothing; a label on it binds the text matched`.
Fixes: #1030
Pushed: 24751a95 (pushed by drive.nika)
Tests: new test grammar_a_label_on_a_literal_binds_its_text failed on the unfixed binary (rustc `Mismatched types` at 3:45 and 12:47), passes now (all five rows, choice row included); it guards + `grammar` filter 119 passed; tests/compiler --both-settings 409 passed, 0 failed at both settings
Gate: green
Deviation: also changed `tools/text_tiers.nika` (and its lowered twin), outside the brief's file list: the checker's arm alone gives NK1104/NK1106, the result-tier pass needs the same arm
Found: none
Needs: none
Changelog: A label on a literal in a grammar binds the matched text, now also when the action returns it or stores it in a field (#1030).
Owner: none
Disk freed: 0.02 GB (scratch binary copy); cache 5.9G, disk 10G free, new entry run-bundle/b2db8b4e959f1cfc 341 MB
Rs2nika: none
