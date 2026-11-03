Result: partial
Files: tests/compiler/tests/a_view_into_a_field_changed_in_place_is_refused.nika tests/compiler/tests/a_view_into_a_field_changed_in_place_is_refused.stderr tests/compiler/tests/a_copy_into_a_field_changed_in_place_is_accepted.nika tests/compiler/tests/a_copy_into_a_field_changed_in_place_is_accepted.stdout tests/compiler/tests/a_view_into_an_element_changed_in_place_is_refused.nika tests/compiler/tests/a_view_into_an_element_changed_in_place_is_refused.stderr tests/compiler/tests/map_keys_from_a_buffer_stay_views.nika tests/compiler/tests/map_keys_from_a_buffer_stay_views.stdout docs/briefs/views/v2-inferred-views.md.report.md
Commit: tests: a field or element changed in place holds its own text, pieces of a buffer never changed stay views (ADR-views-3 D5)
Body: Step 1 of v2 (#1213, D5) already holds on develop before this brief, so this commit adds only its tests and no checker change. A piece of a buffer flowing into a field changed in place is NK1106 at the literal, with help `.clone()`. Flowing into a list whose element is changed in place, it is NK1102 at the `push`, also with help `.clone()`. The same field given `.clone()` is accepted. Map keys, list elements and a struct's map field that hold pieces of a buffer and are never changed stay views. The ledger part was checked by hand with `nikaia lower`, because a test program's ledger is not recorded: `count` has `HashMap[ref(text) String, i64]`, `words` has `Vec[ref(text) String]`, and `Index` has `tethered = ["counts"]`. Step 2 is not built (see the report).
Part: #1213
Tests: 4 new tests/compiler programs pass in a scratch project (nikaia test --bless, then compared), fmt --check clean; they pass on the unchanged compiler too (step 1 was already built, so no fails-before run); no Rust or tools .nika changed, so no build, lower-std, clippy or it run
Gate: green
Deviation: step 2 not built: its order (refuse a struct holding a view of a local handed back) conflicts with Part I 6.6 and NK2303, which anchor it in the caller and refuse nothing; ADR-views-3 D5 and D8 name no refusal, and no escape error code exists to use
Found: none
Needs: supervisor decision on step 2. (a) b37's probe `struct L { n: i64 } struct W { own: ref L, k: i64 } fn make() -> W { let xs = [L { n: 3 }]; let first: ref L = xs[0]; return W { own: first, k: 1 } }` still reaches rustc ("Cannot return value referencing local variable"). D4 removes `ref` fields anyway. Should this be refused (NK2304 "the tether cannot lower" is the closest code, but Part III lists only two cases for it) or anchored as 6.6 says? (b) The views-3 spelling also fails: `struct L { n: i64, tag: String } struct W { own: L, k: i64 }` with `let first = xs[0]; return W { own: first, k: 1 }` reaches rustc ("Mismatched types"). `fn go(l: L) -> i64 { let w = W { own: l, k: 1 }; return show(w) }` gives NK2105 at the caller ("go keeps l"), but D5's last paragraph says that is lending. Building that needs a non-text field to be a view or own its value per value, which is emitter work (`emit_*`, not this lane). Is a lent non-text parameter "a piece of a buffer" for D5?
Changelog: none
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 95bfd5db943ded64ea6cea58c7cc10667f36e227
