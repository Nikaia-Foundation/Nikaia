Result: done
Files: crates/nikaia/src/contracts/keeps.rs tests/compiler/tests/grammar_an_entry_takes_a_parameter_of_its_caller.nika tests/compiler/tests/grammar_an_entry_takes_a_parameter_of_its_caller.stdout
Commit: keeps: a grammar entry keeps the values it takes after its input
Body: The decision lives in the keeps column (contracts/keeps.rs, the graph node of an entry rule), which lends_in_own reads for the checker and the emitter alike: an entry's node held only the input, so every caller's parameter handed to a value parameter was inferred lent (`&Limits`) while the rule's parser takes it by value. Each by-value rule argument is now kept, so a caller's parameter becomes owned as with an ordinary call that keeps it; a copied type still copies (lends asks moves first). No checker or emitter .nika changed, no lowered file changed.
Fixes: #1041
Pushed: 7a91e7d9 (pushed by drive.nika)
Tests: new grammar_an_entry_takes_a_parameter_of_its_caller fails on the unfixed binary (rustc Mismatched types at both calls, String and ref Bytes) and passes; tests/compiler --both-settings 424/424 ok both settings (all 24 grammar_ incl. grammar_an_entry_takes_values_after_its_input); packages/yaml 11/11; guard set + it `keeps` filter 156 passed (sequences guard green); clippy clean; pre-push-check ok
Gate: green
Deviation: none
Found: none
Needs: push of 7a91e7d9 (rejected, not rebased per brief)
Changelog: A grammar entry's value parameters take their arguments as an ordinary call's kept parameters do: a caller's parameter handed there is owned, no longer lent (#1041).
Owner: none
Disk freed: 0 (cache 6.4G, 7.5G free; new entry run-bundle/2df207096bba39fb 286 MB)
Rs2nika: none
