Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/views.nika crates/nikaia-std/src/tools/emit_function.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs crates/nikaia-std/src/tools/lowered/views.rs crates/nikaia-std/src/tools/lowered/emit_function.rs crates/nikaia-std/src/tools/lowered/project_members.rs tests/compiler/tests/a_struct_built_from_a_ref_parameters_views_keeps_them.nika tests/compiler/tests/a_struct_built_from_a_ref_parameters_views_keeps_them.stdout tests/compiler/tests/an_enum_built_from_a_ref_parameters_views_keeps_them.nika tests/compiler/tests/an_enum_built_from_a_ref_parameters_views_keeps_them.stdout
Commit: tiers: a struct or variant built from views read through a ref list's elements keeps them
Body: text_tiers sees the element struct through a `ref Vec[P]` (through_ref), so the field becomes a view position; views no longer refuses (NK2302) a push into a local list that is handed back as the result when what it holds brings the buffer; emit_function writes such a list parameter `&[P<'r>]` (source_nested), so the result points into what the elements view, not into the slice. lowered/project_members.rs: fixpoint drift (TrustedMap to HashMap) that the unfixed binary writes too.
Fixes: #1000
Pushed: none
Tests: both new tests print ab/cd/ü at both settings; unfixed binary: struct test NK1106, enum test rustc "Mismatched types"; SUITE
Gate: build + lower-std fixpoint clean, cargo fmt, pre-push-check.sh green, no `&'r [` in it tests; clippy not run (no hand-written Rust changed)
Deviation: step 2 has no .stderr test: the tier/value mismatch of probe 2 is gone, and no case is refused on purpose
Found: an owned field moved out through an index of a ref list (`Ev { raw: ns[0].raw }`, Node.raw owned) reaches rustc "cannot move out of a shared reference", before and after this change; `n.raw` with `n: ref Node` gets NK2106. Cause: check_arm_leaves.hands_over_read_at needs a read path (bounds_basic.path_of), which has no Index segment
Needs: a bug issue for that NK2106 gap (probe above, 6 lines), status ready
Changelog: A struct or enum built in a function from views read through a `ref` list's elements keeps those views, and the list lowers as `&[T<'r>]`, so the result points into what the elements view.
Owner: none
Disk freed: DISK
Rs2nika: none
