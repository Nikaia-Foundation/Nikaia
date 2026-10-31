Result: blocked
Files: packages/yaml/src/main.nika packages/yaml/nikaia.contracts packages/yaml/nikaia.derived packages/yaml/tests/document_markers.nika packages/yaml/tests/document_markers.stdout
Commit: yaml: documents with their markers, and a flow flag on each node (#928 step 4, ADR-yaml-1 D7)
Body: Node gets `pub flow: bool`: true for a flow sequence or mapping, including the single pair `[a: b]`. A node that takes its properties copies `flow` from its content. `documents(input; limits)` gives `Vec[Document]` (`root`, `start` for `---`, `end` for `...`) and throws the same size and depth errors. The grammar's `Docs` rule moves each document into the list with `drain()`, so no node is copied: each further document records whether a `...` came before it, and the stream's last `...` ends the last document. `parse` is now `documents` with each root taken out by `drain()`, with no second parse. Test `document_markers` covers markers, spans and `flow`; its expected output was checked by hand.
Part: #928
Tests: packages/yaml 15 passed (all 14 existing tests have the same output byte for byte, plus the new document_markers); fmt::every_nika_file ok; pre-push-check ok; Probe A of y22 prints `1 hello`
Gate: green
Deviation: steps 2 and 3 (events, event_line, tests/events.nika) not built; blocked on the gap under Found
Found: for a scalar that resolves to Null, Bool, Int or Float, the tree drops what the suite's `=VAL` line needs. Null and Bool drop their raw text (`~`, `NULL` and `True` give the same nodes as `null` and `true`). A tagged quoted scalar drops its quote style (`!!int "1"` becomes Int(1), and its span starts at the tag). Probe: parse `[~, null, NULL, true, True, !!int "1"]` and print each item's value: `Null Null Null Bool(true) Bool(true) Int(1)`. The suite wants `=VAL :~`, `=VAL :NULL`, `=VAL :True`, `=VAL <tag:yaml.org,2002:int> "1`. Following D7 the fix is to keep this in the tree as written, but where (a `raw` and `style` on Node, or payloads on Value::Null and Value::Bool, which changes every `match`) is a public API choice the brief does not make
Needs: architect decision
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: none
Pushed: local 2a303980d5c1aa2671daa917383858318dbfe3cc
