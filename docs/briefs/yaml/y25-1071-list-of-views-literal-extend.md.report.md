Result: done
Files: crates/nikaia-std/src/tools/views.nika crates/nikaia-std/src/tools/lowered/views.rs tests/compiler/tests/list_literal_of_views_handed_back.nika tests/compiler/tests/list_literal_of_views_handed_back.stdout tests/compiler/tests/recursive_walk_extends_a_list_of_views.nika tests/compiler/tests/recursive_walk_extends_a_list_of_views.stdout
Commit: views: a list of views handed back may be a literal or grown with extend
Body: The views pass treats the items of a list literal and an `extend` as pushes: `return [E { .. }]` of the result's element struct lists each item back (`handed_back`, `listed_back` gain a `ListLit` arm); `local_vec_handed_back` accepts a non-empty starting literal and counts `extend` beside `push` (`a_growing_method`), and the starting items are descended (a list that never leaves) or listed back (the list handed back), as a `push` of them is; `extend` into either list is a push of what it brings. What stands inside the items is still asked, so what a push refuses stays refused (two `ref` parameters: NK2106 for literal and push alike; a by-value `String` viewed in: accepted for both, as before for push).
Fixes: #1071
Pushed: local 5396ac1611e3c431866a5e9e720cb3282c225b23
Tests: tests/compiler --both-settings 481 passed (both new tests failed NK2302 on the unfixed binary as probes a and bc); packages/yaml 16 passed; it fmt::every_nika_file ok; probes b, c (non-recursive extend), a local list with literal start and extend, and four negatives literal-vs-push give the same result; pre-push-check ok; no Rust changed, so no clippy
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A list of views handed back may now be returned as a list literal, start from a non-empty literal, or grow with `extend`, as one built by `push` already could.
Owner: none
Disk freed: 0
Rs2nika: none
