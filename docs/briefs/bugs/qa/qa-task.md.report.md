Result: done
Cause: 39375a17 (#1140): CI arm64 bootstrap_compiler, E0609 no field `in_place_bindings` on CheckFacts (check_core_flow.rs:1004); the commit used the #1139 field but check_facts.nika/.rs, which declare it, were left out of the commit.
Fixed: added the `in_place_bindings` field and its empty() init to check_facts.nika and its lowered .rs (already in the working tree, nothing new written).
Tests: cargo build -p nikaia-std green (was E0609); the full suite is the driver's.
Found: none (emit_core_match/emit_core_leaves working-tree edits for #1139/#1099 belong to other agents, left untouched)
Files: crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/lowered/check_facts.rs
Commit: The CheckFacts field in_place_bindings that the committed flow check uses (the fix for #1140's build)
Changelog: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 92fec4f3084aa57302d7afbe5c88fe2ce93fffd2
