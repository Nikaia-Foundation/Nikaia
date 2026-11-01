Result: done
Files: crates/nikaia-std/src/tools/emit_reach.nika crates/nikaia-std/src/tools/emit_state.nika crates/nikaia-std/src/tools/emit_function.nika crates/nikaia-std/src/tools/lowered/emit_reach.rs crates/nikaia-std/src/tools/lowered/emit_state.rs crates/nikaia-std/src/tools/lowered/emit_function.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/emit/mod.rs tests/compiler/src/main.nika tests/compiler/src/cycle_beside.nika tests/compiler/tests/a_lambda_round_a_cycle_across_files.nika tests/compiler/tests/a_lambda_round_a_cycle_across_files.stdout
Commit: emit: a lambda handed round a cycle through two files is &dyn Fn (Fixes #1129)
Body: code_on_a_cycle split into code_cycle_edges (one file's edges between code-running functions) and code_on_a_cycle(graph, contracts, key); the Rust adapter code_cycle_graph (emit/mod.rs) merges the edges over the file and its beside units, as pausing_reach does, into the new EmitState field code_cycle_graph.
Fixes: #1129
Tests: new a_lambda_round_a_cycle_across_files failed on the unfixed binary (rustc recursion limit at cycle_ping), passes now with --both-settings; a_lambda_handed_round_a_cycle_runs and pausing_cycle_across_files pass (scratch copy of the package); nikaia --lib emit:: ok; clippy, fmt, pre-push-check clean; lower-std fixpoint clean.
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: Fixed: a lambda handed round a call cycle through two files of a package is now written &dyn Fn instead of reaching rustc's recursion limit.
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 11ab3c2d9c0b7ba47e56f52d16259baead87b819
