Result: done
Files: crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/emit_core_match.nika crates/nikaia-std/src/tools/lowered/check_core_flow.rs crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/emit_core_match.rs tests/compiler/tests/a_narrowed_field_in_an_or_pattern.nika tests/compiler/tests/a_narrowed_field_in_an_or_pattern.stdout assets/badges/self-hosted.svg
Commit: emit: a narrowed field in an or-pattern alternative is Some(..) (Fixes #1127)
Body: The checker kept #886/#902's set parts only for a whole arm (`set_positions` by body), never for an `Or` arm. Now `check_core_flow_fields_set` / `check_core_flow_variant_set` compute them for any pattern, and `check_core_flow_set_alternatives` stores one list per alternative in the new `CheckFacts::set_alternatives`. The emitter's set writer is pulled out as `emit_core_set_pattern_written`, and `emit_core_arm_pattern_written` writes each alternative with its own set. Names in an or-pattern stay untyped in the checker (as before), so only the lowering changes. Tuple or-patterns (#879) are left out because their narrowing changes the arm's type.
Fixes: #1127
Tests: new a_narrowed_field_in_an_or_pattern (struct and variant shapes) passes --both-settings, fails on the unfixed binary (E0308); the 12 tests/compiler programs on #879/#886/#902 narrowing pass --both-settings; lower-std fixpoint clean; clippy --all-targets clean; pre-push-check green after badges.py
Gate: green
Deviation: none
Found: separate older defect (the unfixed binary has it too): an or-pattern binding of a positional variant's `String` field that is filled only from literals lowers the field as `&'static str`, so `s.clone()` pushed into a Vec[String] is E0308. Probe: `enum Pair { B(String, i64), D(String, i64), C }`, `match pair { Pair::B(s, _) | Pair::D(s, _) => s.clone(), else => continue }` pushed to Vec[String], main builds `[Pair::B("y", 3), Pair::D("z", 4), Pair::C]`. A single `Pair::B(s, _)` arm works. Suspected cause: the field's tier pass does not see the untyped or-pattern binding flow into owned text.
Needs: an issue for the Found defect; uncommitted lowered/emit_function_body.rs and lowered/emit_core_leaves.rs come from another agent's .nika and were not staged
Changelog: Fixed: a `T?` field or variant value narrowed by an earlier `null` arm now lowers as `Some(..)` inside its own alternative of an or-pattern, not only in a single pattern (#1127).
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 1b7fa366c94d611d4ab533a5b0f0e9634ec4aef1
