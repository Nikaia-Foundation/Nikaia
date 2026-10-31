Result: done
Files: crates/nikaia-std/src/tools/check_grammar.nika crates/nikaia-std/src/tools/emit_grammar.nika crates/nikaia-std/src/tools/emit_pattern.nika crates/nikaia-std/src/tools/lowered/check_grammar.rs crates/nikaia-std/src/tools/lowered/emit_grammar.rs crates/nikaia-std/src/tools/lowered/emit_pattern.rs docs/specification/20-nikaia-advance.md tests/compiler/tests/grammar_an_unbound_text_binds_its_inner_names.nika tests/compiler/tests/grammar_an_unbound_text_binds_its_inner_names.stdout tests/compiler/tests/grammar_a_name_below_a_repeat_is_not_the_actions.nika tests/compiler/tests/grammar_a_name_below_a_repeat_is_not_the_actions.stderr
Commit: grammar: an unbound text(..) binds its inner names, and the action's scope is the emitter's (#1039)
Body: The emitter writes an unbound text(p) whose p binds names as the bound form under a label of its own, __nikaia_text_<span start> (emit_hidden_text): emit_pattern catches it before grammar_nested is set, and emit_text_bindings (now over a Spanned pattern, helper emit_text_binding), emit_text_nested and emit_text_edge_of know it, so the action's prelude binds the covered text (covered_bytes over Bytes keeps the UTF-8 check) and a guard reads the inner names. The checker's grammar_values hands an unbound text's inner names on to a later guard; bindings_of now hands the action only what the emitter binds: through a sequence, a group, a label, text(..) bound or not, and a bound span(p) (helpers grammar_a_span_call, grammar_ungrouped); below a repeat, a counted repeat, a choice or another built-in a name is not the action's (NK1117 at the use). The two new tests are probe rows a-d, which failed on the unfixed binary (rustc "Cannot find value d" for a, b, d; NK1117 at the guard for c); no existing test read a name the new rule refuses.
Fixes: #1039
Tests: rows a-e probed before (a, b, d rustc error; c NK1117; e unchanged) and after (2, "12- 2", ok, NK1117 at the action's d, 12/error), plus an unbound text over Bytes with a guard (prints 3); the two new tests are those programs, so they fail on the unfixed binary; gate test step = tests/compiler --both-settings whole package green; packages/yaml 14 passed; fmt::every_nika_file green in the guards
Gate: green
Deviation: none
Found: none
Needs: push rejected (085bb858 is committed locally on develop, needs the supervisor's rebase and push); packages/yaml/nikaia.lock left dirty by the package test run, not committed
Changelog: An unbound `text(..)` in a grammar binds the names inside it as a bound one does, for the action and a later guard (`text(d:P+) if(d.len() > 1)`), and a name below a repeat, a counted repeat, a choice or another built-in's argument is refused in the action (NK1117) instead of reaching rustc (Part II 10, #1039).
Owner: none
Rs2nika: none
Pushed: 085bb858 (pushed by drive.nika)
Disk freed: 0 GB (no clone; target/nikaia-project-tests 6.4G, / 7.0G free; new entry over 100 MB: run-bundle/2df207096bba39fb 286 MB)
