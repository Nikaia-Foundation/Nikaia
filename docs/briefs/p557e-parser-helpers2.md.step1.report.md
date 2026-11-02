Result: done
Files: crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/lowered/check_core_flow.rs tests/compiler/tests/a_part_a_taking_arm_takes_out_is_lent_to_a_ref_parameter.nika tests/compiler/tests/a_part_a_taking_arm_takes_out_is_lent_to_a_ref_parameter.stdout
Commit: checker: a part a taking arm takes out is lent to a ref parameter with its &
Body: An arm over a `mut` parameter that takes the parts out (`mem::replace`) binds them by value, yet the checker had marked a part handed to a `ref` parameter as a view handed on as it is (`views_handed_on`, from `a_lent_part_is_handed_on_as_it_is`) and recorded no lend, so the emitter wrote `longer(ty)` for `longer(&ty)` (E0308). `check_core_flow_parts_handed_on_in_place` now, for a taking arm, moves each call argument naming a taken part from `views_handed_on` to `lent_args`. Found moving `put_rule_types` into tools/grammar_types.nika (#557 step 4b).
Part: #557
Tests: new test fails on the unfixed compiler (rustc E0308 at the call), prints ab! after
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 5853ffbf49b864a160e3f55e2398f03e0aeacd7e
