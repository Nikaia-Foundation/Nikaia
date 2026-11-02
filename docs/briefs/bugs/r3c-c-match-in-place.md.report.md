Result: done
Pushed: local 39375a179ebcf7c711b07fec711d526effa09c73
Tests: tests/compiler --both-settings PENDING; it mut_parameter_parts:: + taking 15 passed; both new programs failed on the unfixed compiler (one-branch give-back accepted; `s = if c {..} else {..}` refused NK2108)
Deviation: none
Found: none
Body: #1140 fixed: a taking arm's parts count as out from where the arm opens (CheckPartOut.from/choices, set by parts_taken_out_of_a_mut_parameter), not from the hand-over after it, so a give-back on one branch before the hand-over no longer covers the other; a give-back counts where the gives before the hand-over cover every branch of a choice (gives_cover over CheckFlow.choice_arms: arm count of each `match`, 2 for an `if` with `else`), so both branches of an if/else and an `if` expression give back
Needs: handed to R3c-d: wip type_rules chunk 2 (6465e74c): in a scratch sysroot tr_rewrite_item now lowers to `match item.node { Item::Impl { ref target, ref mut methods, .. } => .. methods.iter_mut() ..`, no E0507 left in the lowered text; not compiled with cargo (would need a second target/), so the supervisor or the wip's builder builds it
Changelog: Fixed: a taking arm's parts are out from where the arm opens: a give-back on one branch only is refused (NK2108), and an `if` expression or both branches of an if/else give them back (#1140)
Files: crates/nikaia-std/src/tools/check_state.nika crates/nikaia-std/src/tools/check_reads.nika crates/nikaia-std/src/tools/check_takes.nika crates/nikaia-std/src/tools/check_flow.nika crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/lowered/check_state.rs crates/nikaia-std/src/tools/lowered/check_reads.rs crates/nikaia-std/src/tools/lowered/check_takes.rs crates/nikaia-std/src/tools/lowered/check_flow.rs crates/nikaia-std/src/tools/lowered/check_core_flow.rs tests/compiler/tests/a_taking_arm_giving_back_on_one_branch_only_is_refused.nika tests/compiler/tests/a_taking_arm_giving_back_on_one_branch_only_is_refused.stderr tests/compiler/tests/a_taking_arm_gives_back_through_an_if_expression.nika tests/compiler/tests/a_taking_arm_gives_back_through_an_if_expression.stdout
Commit: A taking arm's parts are out from where the arm opens (#1140)
Fixes: #1140
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
