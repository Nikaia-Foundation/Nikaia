Result: done
Files: crates/nikaia-std/std.contracts crates/nikaia-std/src/tools/check_core_call_on.nika crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/emit_core_calls.nika crates/nikaia-std/src/tools/ledger_items.nika crates/nikaia-std/src/tools/lowered/check_core_call_on.rs crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/emit_core_calls.rs crates/nikaia-std/src/tools/lowered/ledger_items.rs crates/nikaia/src/contracts/mod.rs tests/compiler/tests/drain_every_map_and_set.nika tests/compiler/tests/drain_every_map_and_set.stdout
Commit: Add drain() on BTreeMap, HashSet and BTreeSet (ADR-386), repay the clone in ledger_items_stored
Body: Contracts for the three; a kept B-tree drain lowers to `mem::take(&mut x).into_iter()` (new fact take_drains); a HashMap/set/B-tree field path of a local is now emptied in place, not moved (Vec excluded: node_numbers relies on partial moves). ledger_items_stored takes `mut codes` and drains; contracts/mod.rs passes `&mut`.
Fixes: #1130
Pushed: local c9b5e563e546f6ba7ba1f0f18d96aad85b4808ff
Tests: new test drain_every_map_and_set passes --both-settings (NK1171 on old binary); lib contracts:: 36 ok; it throws/ledger/contract 144 ok; clippy, pre-push green
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: drain() on BTreeMap, HashSet and BTreeSet (ADR-386)
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
