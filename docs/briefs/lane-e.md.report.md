Result: done
Pushed:
Tests: match_arm_if_statement (new, #930 probe), match_subject_field_copied (new, #940 probe), an_else_after_every_variant_is_silent (new, #918 probe; rustc's unreachable-pattern warning gone, its dead-variant warning stays as the issue says); each failed unfixed per its issue; tests/compiler --both-settings 348 passed; it guard set 23 passed
Deviation: three fixes in one commit, all in emit_core_match.nika (the commit script adds whole files); lowered/check_compare.rs is in it because #930 drops a stray `;` there
Found: none
Needs: none
Changelog: A `match` arm whose body is an `if` without `else` lowers as a block statement (#930); a field read in an arm of a `match` that is itself another `match`'s subject is copied out (#940); an `else` after arms that name every variant of an enum is not written (#918, ADR-357 D5). Decided: #918 holds for an enum this file declares, arms matched whole (unit, tuple or named with only bindings), no guard; EmitFlow gets `subject_copies`.
Gate: green
Files: crates/nikaia-std/src/tools/emit_core_match.nika crates/nikaia-std/src/tools/lowered/emit_core_match.rs crates/nikaia-std/src/tools/emit_kinds.nika crates/nikaia-std/src/tools/lowered/emit_kinds.rs crates/nikaia-std/src/tools/lowered/check_compare.rs tests/compiler/tests/match_arm_if_statement.nika tests/compiler/tests/match_arm_if_statement.stdout tests/compiler/tests/match_subject_field_copied.nika tests/compiler/tests/match_subject_field_copied.stdout tests/compiler/tests/an_else_after_every_variant_is_silent.nika tests/compiler/tests/an_else_after_every_variant_is_silent.stdout docs/briefs/lane-e.md.report.md
Commit: Lower an if arm as a block, copy a field out of a nested match subject, drop an unreachable else
Fixes: #930, #940, #918
