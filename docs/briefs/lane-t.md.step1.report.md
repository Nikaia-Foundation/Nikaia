Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs tests/compiler/tests/text_field_of_element_changed.nika tests/compiler/tests/text_field_of_element_changed.stdout tests/compiler/tests/map_value_changed_through_view.nika tests/compiler/tests/map_value_changed_through_view.stdout docs/briefs/lane-t.md.step1.report.md
Commit: text tiers: an element reached by index, get or a for binding owns the text changed through it
Body: #967: the struct of a list or map element is known for v[i], let mut e = v[i] and a for binding over iter/iter_mut/drain/into_iter (element_struct, walked_struct, Local.of), so a change in place of its text field makes that field text of its own. #981: a let of xs[i], m[k] or m.get(k), alone or before a ?? whose fallback jumps (return, break, continue), reads the element's position (element_text_word, tier_a_jump), so a change in place through the local makes the element text of its own. One commit for both: the two changes share text_tiers.nika.
Fixes: #967, #981
Tests: 2 new tests/compiler programs (text_field_of_element_changed, map_value_changed_through_view), both fail on the unfixed compiler (rustc += on &'static str); tests/compiler --both-settings 348 of 348 ok; it touched + guard set 175 ok; clippy clean; pre-push-check ok; lower-std at fixpoint
Gate: green
Deviation: #967 and #981 in one commit (one file)
Found: let mut sg = s.get("b") ?? return; sg.push_str("?") gives a false NK1171 (String has no method push_str), lane C's checker; the test uses += instead
Needs: none
Changelog: A text field of a list or map element changed in place through v[i], a let of the element or a for binding, and a text element of a list or map changed in place through a let of xs[i], m[k] or m.get(k) (also before ?? return), now makes that element text of its own instead of lowering a borrowed text the change cannot write to.
Rs2nika: none
