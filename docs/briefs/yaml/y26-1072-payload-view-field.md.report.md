Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs tests/compiler/tests/view_field_of_a_payload_handed_on.nika tests/compiler/tests/view_field_of_a_payload_handed_on.stdout tests/compiler/tests/view_fields_of_payload_elements_handed_on.nika tests/compiler/tests/view_fields_of_payload_elements_handed_on.stdout
Commit: tiers: a view field read off a match payload of a lent struct is a view
Body: A match payload binding of struct type got no tier local, so `m.name` had no source and counted as owned (tier_at's default), making E.name mixed and the checker refuse NK1106. bind_a_payload now binds a struct payload with `of` set to its struct (as a for element is, #967) and a list payload with its declared position, so `for x in items` binds the element's struct. Named variant fields were not declared to the tiers at all; TextDeclared gains `variant_fields` (types only, not positions), used for a named-variant field that holds no text itself (a struct or a list of one), so `V::One { m }` works too; text fields of named variants are unchanged. An owned field read into an f-string stays owned (probed: `O { name: f"<{m.name}>" }` prints `<hello>`).
Fixes: #1072
Pushed: local 9e706dc030d6972bb1af48e57d757521c12a0660
Tests: both new tests failed NK1106 on the unfixed binary (nikaia-suite) and print `1 hello` now; named-variant probe and owned-into-f-string probe pass; tests/compiler --both-settings 483 passed 0 failed; packages/yaml 16 passed; fmt::every_nika_file ok; clippy 1.97 clean (1.99.1 not installed here, change is only a lowered file); pre-push-check ok
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A `String` field that holds a view can be copied from a struct bound by a `match` pattern (a tuple or named variant payload, or an element of a payload list) of a borrowed value into a new struct.
Owner: none
Disk freed: 0 GB (scratch binary only); cache 3.7G, 6.9G free, 1 new entries over 100 MB
Rs2nika: none
