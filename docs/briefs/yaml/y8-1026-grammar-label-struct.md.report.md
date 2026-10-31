Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs tests/compiler/tests/grammar_a_view_field_read_from_a_bound_struct_stays_a_view.nika tests/compiler/tests/grammar_a_view_field_read_from_a_bound_struct_stays_a_view.stdout
Commit: text tiers: a field read from a struct a grammar label binds holds what its position holds
Body: A label bound to a rule of the grammar (directly or through a group) whose declared result is one plain name now binds a local whose `of` is that struct (`grammar_bound_struct`, the test of `called_struct`), so `p.a` and `let an = p.a` read the field's position. A second cause showed up while building the brief's `match p { Props { a: x } => .. }` case, and it was not about the label: `bind_a_payload` bound no local for a plain `String` field whose tier is not static, so `x` read as owned text whatever the scrutinee. It now binds the field's kinds, with `from` set to the position, as a map walk's key is. The test labels the rule directly, not through a group: a label on a parenthesised group (`p:(P)`) is out of scope in the action, a separate defect (see Found).
Fixes: #1026
Tests: new grammar_a_view_field_read_from_a_bound_struct_stays_a_view fails on the unfixed compiler (NK1106 at 9:76 and 9:130); tests/compiler --both-settings 404 passed 0 failed (all grammar_ tests included); packages/yaml 10 passed; it guard set + text_tier 82 passed; nikaia-std --lib text_tiers:: 0 tests; clippy clean; pre-push-check ok
Gate: green
Deviation: none
Found: a label on a parenthesised group is not in scope in the action, without any text either: `struct N { v: i64 }` grammar G { rule WS = "" -> { } entry rule top -> i64 = n:(P) eof -> { n.v } rule P -> N = "&" -> { N { v: 1 } } } gives "Cannot find value `n` in this scope" (old and new binary)
Needs: issue for the group-label defect (Found)
Changelog: A text field read from a struct that a grammar label binds, or bound by a struct pattern, now keeps the tier of its position, so a view stays a view instead of being refused as text of its own (NK1106).
Owner: none
Disk freed: 0
Rs2nika: none
Pushed: 555fe6e97e38c1cb6f517407f278219621c154b7
