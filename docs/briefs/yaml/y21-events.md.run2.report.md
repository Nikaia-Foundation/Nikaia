Result: blocked
Files: none
Commit: none
Pushed: none
Tests: pre-flight green (Node.raw/style, scalar_written.stdout present); `nikaia check packages/yaml/src/main.nika` refuses the draft; probes run with target/fast/nikaia at 6269eca3; package tests not run (nothing committed)
Gate: green
Deviation: steps 2 and 3 not built: the view checker refuses the walk (Found)
Found: three view-checker defects (#1063 family), probes in branch wip/y21-events, docs/briefs/yaml/y21-probes/: (1) enum_param_blamed: `fn opening(kind: K, n: ref N) -> E { E { kind: kind, name: n.name } }` with K a field-less enum and N.name a real view gives NK2302 "hands back a view of `kind`"; even `fn bare(kind: K) -> E { E { kind: kind, name: "" } }` is blamed. (2) vec_literal_refused: `fn walk(n: ref N) -> Vec[E] { return [E { name: n.name, value: "" }] }` gives NK2302 "n: ref N doesn't say whose data", while the same with `let mut out = []; out.push(..); return out` (vec_push_accepted) prints `2 x`. (3) payload_for_element_field: `match n.value { V::Seq(items) => for x in items { out.push(E { name: x.name, value: x.raw }) } }` gives NK1106 on both fields (y22 fixed a `for` over a `ref Vec` parameter, not over a match payload of a lent struct). recursive_extend (`out.extend(walk(x))` in a recursive walk) is also NK2302. Pushing views of a `ref` node into a `mut out: Vec[Event]` parameter is NK2302 by the rule (whose data is not declared), so the walk returns its events.
Needs: architect decision: file (1)-(3) as compiler defects for the bug lane; then y21 run 3 starts from wip/y21-events (events, event_line, tests/events.nika drafted, fmt-clean; .stdout not yet blessed). Tags: the tree keeps them as written, the draft writes `<!!str>`, a verbatim `!<t>` as `<t>`.
Changelog: none
Owner: none
Disk freed: 0 GB (target/nikaia-project-tests 3.4G, / 12G free, no new entry over 100 MB)
Rs2nika: none
