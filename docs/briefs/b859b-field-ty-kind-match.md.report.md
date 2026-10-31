Result: done
Files: crates/nikaia-std/src/tools/check_buildtime.nika crates/nikaia-std/src/tools/lowered/check_buildtime.rs crates/nikaia-std/src/tools/check_core_expr.nika crates/nikaia-std/src/tools/lowered/check_core_expr.rs crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/lowered/check_core_flow.rs crates/nikaia-std/src/tools/check_core_pilot.nika crates/nikaia-std/src/tools/lowered/check_core_pilot.rs crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/emit_core_match.nika crates/nikaia-std/src/tools/lowered/emit_core_match.rs crates/nikaia-std/src/tools/emit_misc.nika crates/nikaia-std/src/tools/lowered/emit_misc.rs crates/nikaia-std/src/tools/lowered.rs tests/compiler/tests/match_on_a_kind.nika tests/compiler/tests/match_on_a_kind.stdout tests/compiler/tests/match_on_a_kind_arm_checked.nika tests/compiler/tests/match_on_a_kind_arm_checked.stderr
Commit: check, emit: a match on a kind checks each arm under its kind and emits the arm the type takes (ADR-330 D4, #859 step 4)
Part: #859
Tests: reflected_field_ty, match_on_a_kind, match_on_a_kind_arm_checked fail on the unfixed compiler (parse error; NK1231, NK1231); tests/compiler --both-settings 425/425; full it after step 1: 2886 passed, only the_compiler_test_package failed (run concurrently with tests/compiler, different programs each run), which passed 423/423 alone
Gate: green
Deviation: check_core_flow_meet made pub (check_core_flow.nika); `path.last() ?? return null` of Vec[Symbol] lowers to &Symbol (E0308), so written `path[path.len() - 1]` as check_boxes.nika does
Found: lowering of `v.last() ?? return null` over a described non-Copy type (winnow_grammar::Symbol) passed by value emits &Symbol and rustc refuses it (user programs with i64 work)
Needs: supervisor rebase of 4ff292be and this commit onto origin/develop (push rejected)
Changelog: Reflected field types: in a walk over `T::fields`, `field.ty::name`, `field.ty::kind` and `field.ty::fields` name the field's type, and `expr.name::member` parses (ADR-330 D5).
Changelog: A match on a kind: `match T::kind { Text => ..., List(it) => ..., Struct(s) => ... }` checks every arm and builds the arm each type takes (ADR-330 D4).
Owner: none
Disk freed: none (cache 3.3G, / 19G free; new entries over 100 MB: build-time-parsers 849M, build-time-bundle 484M, run-bundle 286M)
Rs2nika: none
Pushed: rejected
