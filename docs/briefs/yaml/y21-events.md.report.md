Result: done
Files: packages/yaml/src/main.nika packages/yaml/tests/events.nika packages/yaml/tests/events.stdout packages/yaml/nikaia.contracts packages/yaml/nikaia.derived
Commit: yaml: events, the parsed tree walked into the yaml-test-suite's event form (#928 step 4, ADR-yaml-1 D7)
Body: Adds pub Event (kind, anchor, tag, value as views of the input, an owned text for a Value::Str's text(), start/end byte offsets), pub fn events(docs: ref Vec[Document]) -> Vec[Event] and pub fn event_line(e: ref Event) -> String writing +STR/-STR, +DOC [---]/-DOC [...], +SEQ [[]]/+MAP [{}] with &anchor and <tag>, =VAL with its style mark and escaped value, =ALI *name; an empty node is =VAL :. The draft of runs 2-3 (wip/y21-events 1c1199d6) taken as is, walk keeps its `return match` (#1085 fixed by 91b72357). A tag is written as the tree keeps it, short forms unresolved (<!!int>, <!!map>, <!local>; a verbatim !<t> as <t>); resolving is a later step of #928. Spans are the node's: a document's start is its root's (after ---), an end event sits at the node's end offset. tests/events.stdout is blessed from what the program prints and checked by hand against the suite's form.
Part: #928
Pushed: local 0a55a3cec264fe56594404cfa55d9b8d993fed5d
Tests: package tests 17/17 green (events new, blessed then rerun); fmt::every_nika_file green; scripts/pre-push-check.sh green; no Rust changed, so no clippy; pre-flight y28 probe printed 1 hello
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0 GB (cache 3.7G, 9.4G free; new entry run-bundle/3f8b93b88f7f213d 364 MB)
Rs2nika: none
