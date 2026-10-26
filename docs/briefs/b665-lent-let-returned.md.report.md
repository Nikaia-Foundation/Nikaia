Result: done
Files: crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/check_fields.nika crates/nikaia-std/src/tools/keeps.nika crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/check_fields.rs crates/nikaia-std/src/tools/lowered/keeps.rs tests/compiler/tests/let_moved_returned.nika tests/compiler/tests/let_moved_returned.stdout tests/compiler/tests/let_moved_pushed.nika tests/compiler/tests/let_moved_pushed.stdout tests/compiler/tests/let_moved_pushed.stdout tests/compiler/tests/let_moved_read_again.nika tests/compiler/tests/let_moved_read_again.stderr tests/compiler/tests/let_moved_ref_parameter.nika tests/compiler/tests/let_moved_ref_parameter.stderr assets/badges/self-hosted.svg
Commit: A `let` over a part of an owned parameter, handed back or pushed, is a move (Fixes #665)
Fixes: #665
Tests: compiler project --both-settings 117 passed; it: guard set + handed_over + keeps/lent/views/let_ 280 passed; probe and push tests failed before the fix (rustc E0308 / E0507); the two refusals were already refused (regression pins)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A `let` over a field of a by-value parameter, then handed back or pushed, is a move; the contract keeps the parameter when the alias is handed on
Owner: none
Disk freed: 0
Pushed: bcdbeca3d1823180d79adb9387a2f6349c218b69
