Result: done
Files: crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/check_patterns.nika crates/nikaia-std/src/tools/emit_core_match.nika crates/nikaia-std/src/tools/lowered/check_core_flow.rs crates/nikaia-std/src/tools/lowered/check_patterns.rs crates/nikaia-std/src/tools/lowered/emit_core_match.rs tests/compiler/tests/a_struct_field_is_set_after_its_null_arm.nika tests/compiler/tests/a_struct_field_is_set_after_its_null_arm.stdout tests/compiler/tests/a_struct_field_without_its_null_arm_stays_nullable.nika tests/compiler/tests/a_struct_field_without_its_null_arm_stays_nullable.stderr
Commit: check, emit: a struct field at a `T?` position narrows after its `null` arm
Body: Steps 1-3 in one commit. The checker's new check_core_flow_set_fields sits next to the tuple case and records the narrowed fields in set_positions, keyed by the arm body. A field is recorded by its index in the pattern's field list as written, not in declaration order, because the emit walks that list. Bindings are narrowed in check_core_flow_arm_pattern_parts, and the emit writes `field: Some(pat)`. Root cause: a plain struct pattern `F { .. }` over an `F` looked up the key `F::F` and found nothing, so its fields bound untyped and `name.len()` on a `String?` passed the checker. The new named_pattern_key resolves to the struct's own name when the struct has no type arguments.
Fixes: #886
Tests: tests/compiler both settings 175/175 green (2 new); new tests failed before the fix (old binary: rustc "Method `len` is private" for the stdout probe; checker passed the stays-nullable case); it nullable:: map_reads:: members_written_down:: match_*:: struct_patterns:: green; nikaia-std --lib green; clippy, pre-push-check green
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A struct field narrows after its `null` arm: in a `match`, `F { name, .. }` after `F { name: null, .. }` binds `name` as the `T`, as a tuple position does (ADR-352 D1, #886); fields of a plain struct pattern are now typed (they were unknown)
Owner: none
Disk freed: 0 GB
Pushed: 81310c8b823419ae9fa988903e90145b02a4f3e2
