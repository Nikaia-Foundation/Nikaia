Result: blocked
Files: none
Commit: none
Fixes: none
Pushed: none (work kept on wip/y2c-properties b223f972: draft ea068fa4 on develop fbba5562 plus tests/anchors_tags_aliases.nika/.stdout)
Tests: pre-flight green after one fast build (the binary predated 262d5fa6): grammar_a_call_in_a_guard_is_a_call_site and grammar_a_guard_reads_a_binding_text_label pass, so #1037 is covered by the package tests; draft applied cleanly, nikaia check passes; nikaia test --project packages/yaml 11/11 with the new anchors_tags_aliases (spec examples 2.10, 2.24, 6.23, 6.24, 6.28, 7.1, 7.2, core tags fixing and checking the kind, empty nodes, properties above block collections, error cases), outputs checked by hand against YAML 1.2.2; fmt::every_nika_file red on packages/yaml/src/main.nika
Gate: red fmt::every_nika_file_formats_to_the_same_tree_and_a_fixed_point
Deviation: none
Found: #1015 is still open and the draft hits it: nikaia fmt rewrites every rule line over 100 columns that holds `if(call(..), "label")` into a file that does not parse ("Expected `)` here, but found `,`"); 9 such lines in the draft (Key, Propped x3, FlowAny, Double, Single, BlockScalar, Plain). Probe: `grammar G { rule WS = "" -> { } entry rule top -> i64 = a:here <a 48-letter label>:here if(f(1, 2), "a node its tag fits") eof -> { 1 } }` with `fn f(a: i64, b: i64) -> bool sync`; the same line under 100 columns formats fine. A hand break inside the rule does not help: fmt moves the continuation line to one tab, below the rule's own indentation (a second layout defect, same pass)
Needs: fix #1015 (fmt breaking a grammar guard with a label; also a continuation line of a rule alternative at its proper indentation), then a run that takes wip/y2c-properties as-is, runs fmt on main.nika and the package gate, and commits (Part of #928). Shortening the rules to fit 100 columns was not done: that is a shape picked to get around the defect
Changelog: none
Owner: none
Disk freed: 0 GB (target/nikaia-project-tests 6.1G, / 5.4G free; no cache entry over 100 MB new in this run; no clone made)
Rs2nika: none
