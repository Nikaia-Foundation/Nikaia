Result: done
Cause: daf44ac2 (already on origin/develop, ADR-388 D3 moved winnow_grammar::Symbol into std's ledger): it sysroot::a_tool_reads_the_names_of_the_tree_as_described
Fixed: the test's std half now probes a name only described.contracts holds (node_id::NodeId::fresh().get()), since Symbol is std's on purpose
Tests: sysroot::a_tool_reads_the_names_of_the_tree_as_described ok; cargo fmt; clippy --all-targets clean; pre-push shows badges/version out of date (not from my file, supervisor's)
Found: none
Files: crates/nikaia/tests/it/sysroot.rs
Commit: A module of std is refused a name only the tools' description holds: the sysroot test probes node_id::NodeId, not Symbol (ADR-388 D3)
Body: ADR-388 D3 (daf44ac2) moved winnow_grammar::Symbol and InternerContext into std's ledger, so lower_std_module rightly accepts them and the test's refusal half no longer held. The refusal it guards is unchanged and is now shown with node_id::NodeId, which only described.contracts holds.
Changelog: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 65c3296bd2b489b92098c78be0ce06e2e9da6883
