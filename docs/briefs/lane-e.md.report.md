Result: done
Pushed:
Tests: for_mut_element_without_parens (new, #970 probe; unfixed it printed rustc's unused-parens warning), format_hole_before_changing_method (new, #937 probe; failed unfixed per the issue, borrow of self while mutably borrowed); tests/compiler --both-settings 348 passed; it guard set 23 passed; clippy -D warnings clean without the unused_parens allow
Deviation: #970 and #937 in one commit (both touch emit_core_leaves.nika); the no-warning check of #970 is clippy over the lowered tools, as tests/compiler compares no rustc warnings
Found: none
Needs: none
Changelog: A `mut` lambda or `for mut` element lowers as `*x`, in parentheses only as a postfix base (#970; the `unused_parens` allow on the tools module is gone); an `f"…"` hole read before a hole that calls a `ref mut self` method of the same value is copied first (#937). Decided: holes are evaluated left to right as written; the earlier ones are made owned (`ToOwned::to_owned`) only when a later hole changes the receiver.
Gate: green
Files: crates/nikaia-std/src/tools/emit_core_leaves.nika crates/nikaia-std/src/tools/lowered/emit_core_leaves.rs crates/nikaia-std/src/tools/emit_expr.nika crates/nikaia-std/src/tools/lowered/emit_expr.rs crates/nikaia-std/src/tools/lowered/bounds_walk.rs crates/nikaia-std/src/lib.rs tests/compiler/tests/for_mut_element_without_parens.nika tests/compiler/tests/for_mut_element_without_parens.stdout tests/compiler/tests/format_hole_before_changing_method.nika tests/compiler/tests/format_hole_before_changing_method.stdout docs/briefs/lane-e.md.report.md
Commit: Lower a changed binding as *x without parens; copy f-string holes read before a mutating hole
Fixes: #970, #937
