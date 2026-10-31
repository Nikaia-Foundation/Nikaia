Result: done
Files: crates/nikaia-std/src/tools/emit_pattern.nika crates/nikaia-std/src/tools/emit_grammar.nika crates/nikaia-std/src/tools/lowered/emit_pattern.rs crates/nikaia-std/src/tools/lowered/emit_grammar.rs tests/compiler/tests/grammar_a_label_on_a_one_element_group_binds_its_value.nika tests/compiler/tests/grammar_a_label_on_a_one_element_group_binds_its_value.stdout
Commit: grammar: a label on a one-element group binds its value (#1027)
Body: The emitter writes a group around one element that is not a sequence or a choice as that element, nested groups included (Part II 10.8, `( p )` yields as `p`); winnow-grammar read `n:(P)` as a scope of its own. A label's pattern is looked at through such groups (`emit_ungrouped`) in the `span(..)`, binding `text(..)` and literal checks of the emitter and in `emit_span_bindings` / `emit_text_bindings`, so `sp:(span(p))` and `t:(text(a:x b:y))` work too (probed by hand).
Fixes: #1027
Tests: new test failed on the unfixed binary (three `Cannot find value` n, k, s), passes now; tests/compiler --both-settings 407 passed; packages/yaml 10 passed; clippy clean; guard set 24 passed (sequences within ceiling)
Gate: green
Deviation: none
Found: a captured literal returned over bytes reaches rustc: `entry rule top -> String = x:"ab" eof -> { x }` over `Bytes` passes `nikaia check`, rustc refuses (`Lifetime may not live long enough`); with or without a group, before this change
Needs: rebase and push of c46fe7cc (push rejected, develop moved)
Changelog: A label on a group around one element (`n:(P)`, `k:((P))`, `s:(text(p))`) binds what the element yields, as Part II 10.8 says, instead of reaching rustc as an unknown name.
Owner: none
Disk freed: 0.2 GB (scratch binary copy); cache 5.8G, disk 11G free, no new entry over 100 MB
Rs2nika: none
Pushed: c46fe7cc (pushed by drive.nika)
