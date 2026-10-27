Result: done
Files: tests/compiler/tests/an_if_let_binding_in_a_condition_is_refused.nika tests/compiler/tests/an_if_let_binding_in_a_condition_is_refused.stderr tests/compiler/tests/an_is_binding_in_a_condition_is_refused.nika tests/compiler/tests/an_is_binding_in_a_condition_is_refused.stderr docs/briefs/b869b-path-rest.md.step2.report.md
Commit: tests: `if let v = x` and `if e is Leaf(v)` are refused (ADR-382 D6); probed, the parser refuses all three forms already, so no compiler change (its words do not yet name `match`)
Part: #869
Tests: nikaia test --both-settings tests/compiler 149 ok; tests only, no compiler change, so nothing to fail against
Gate: green
Changelog: none
Model: Opus 5.5
Pushed: a9978f9836b4a3df6253675fa4af0d5f9fb274be
