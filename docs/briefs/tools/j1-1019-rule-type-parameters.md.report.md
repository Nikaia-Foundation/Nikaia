Result: done
Files: crates/nikaia-std/src/tools/ast.nika crates/nikaia-std/src/tools/node_numbers.nika crates/nikaia-std/src/tools/grammar_types.nika crates/nikaia/src/parser/mod.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/ast.rs crates/nikaia-std/src/tools/lowered/node_numbers.rs crates/nikaia-std/src/tools/lowered/grammar_types.rs scripts/self_hosting.py tests/compiler/tests/grammar_a_rule_takes_a_type.nika tests/compiler/tests/grammar_a_rule_takes_a_type.stdout tests/compiler/tests/grammar_a_value_where_a_rule_takes_a_type_is_refused.nika tests/compiler/tests/grammar_a_value_where_a_rule_takes_a_type_is_refused.stderr
Commit: grammar: a rule and an entry take type parameters (ADR-grammar-2 D1)
Body: The parser reads `rule Name[T: Bound]` (GrammarRule.generics; node_numbers.nika carries the new field). Right after parsing, parser/mod.rs `expand_grammar_types` rewrites each `G::entry[T](input)` to the copy's call `G::entry__T(input)` (a value in the brackets is NK1230), and grammar_types.nika copies the rules once per list of type arguments, following `Rule[T]` refs in patterns, shared by copy name (check_specialised spelling); the generic rule is not kept, so checker, analyses and emitter see only single-type rules. Not yet in this step: a bound is parsed but not checked; type parameters inside actions and guards are not substituted; a rule call with both types and values in a pattern.
Part: #1019
Tests: new grammar_a_rule_takes_a_type and grammar_a_value_where_a_rule_takes_a_type_is_refused both fail on the unchanged compiler (parse error "Expected `=` here, but found `[`"), pass now; nikaia test --both-settings --project tests/compiler: 470 passed, 0 failed; it binary: 2887 passed, 0 failed; nikaia --lib parser:: has no unit tests (0 run); clippy, cargo fmt, pre-push-check green; lower-std fixpoint holds for my files
Gate: green
Deviation: none
Found: none
Needs: none
Supervisor: node_numbers.nika touched for the new GrammarRule field (accepted); Found handled - the lowering defect is #1073, project_members.rs is another session's, GR predates the brief; suite green at fe76ebf8, pushed 5cefba2d.
Changelog: Grammars: a rule and an entry take type parameters, `rule Value[T] -> T`, called `G::read[i64](input)` and `Value[T]` in a pattern; one copy per list of types, a value in the brackets is NK1230 (ADR-grammar-2 D1).
Owner: none
Disk freed: none (8.3G free, not under 3G); cache 2.8G; new entries over 100 MB: rlib df8bcfe6 607M, rlib af187e66 265M, run-bundle 3f8b93b8 287M, build-time-parsers 768d2772 850M, build-time-bundle efd5abe9 484M
Rs2nika: none
Pushed: local c9488f7c5fd7fe47a5450f126c82eacd2f0bb3f9
