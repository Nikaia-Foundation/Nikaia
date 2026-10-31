Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs tests/compiler/tests/grammar_a_mixed_tuple_result_takes_views.nika tests/compiler/tests/grammar_a_mixed_tuple_result_takes_views.stdout
Commit: text_tiers: a tuple's part that holds no text adds no tier to it, so a mixed tuple result takes views (#999)
Body: The tier walk joined every part of a tuple into the tuple's tier, so an i64 or a Value::Num beside a view of the input made `(String, i64)` owned and the rule handing it on through a label refused the views with NK1104. Now kinds_of skips a tuple part that holds no text (tuple_part_holds_text), and flow_tuple_parts gives each text part the join of the text parts only and each other part its own kinds, recorded per tuple literal in TierWalk.tuple_items by node id. Taken over from an earlier run of this brief that died before committing; its fix was rebuilt on the current develop and verified.
Fixes: #999
Pushed: e064b36ee2c523a6338725a5a682f60958d6d70a
Tests: new grammar_a_mixed_tuple_result_takes_views fails on the unfixed compiler with NK1104 (the brief's error) and prints `ab 12` / `cd num 34` fixed; grammar_a_tuple_result_* and grammar_*tuple* tests pass; tests/compiler --both-settings: the 12 tests that failed in the shared runs (NK2302 on main.nika's `places` at one setting, varying from run to run, while other agents ran the same project) pass 3 of 3 in an isolated copy with the ledgers; grammar_a_guard_reads_a_name_bound_inside_text is 03d43063's, newer than my build; a_trait_column_change_warns printed cargo file-lock lines from a concurrent run; guard set 24 passed (sequences included); clippy clean; pre-push-check ok
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A grammar rule whose result is a tuple mixing `String` with another type (`(String, i64)`, `(String, Value)`) takes the views its action builds, also when another rule hands it on through a label; it was refused with NK1104 (#999).
Owner: none
Disk freed: 0
Rs2nika: none
