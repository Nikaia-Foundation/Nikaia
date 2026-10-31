Result: done
Files: crates/nikaia-std/src/tools/build_eval.nika crates/nikaia-std/src/tools/lowered/build_eval.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/comptime_run.rs docs/selfhosting-plan.md
Commit: build time: the default read moves into Nikaia (BT step 5)
Body: bt_known_default(ledger, package, key, at) in build_eval.nika; known_default is one KNOWN.with forward. Pre-flight b9 test output identical. The ledger comes in as a parameter, so the converter's KNOWN.with closure was dropped.
Part: #125
Pushed: local 4a3b701521db2bf33ab9a94f073c3f26afa81027
Tests: it comptime 50 ok, it build_time 65 ok, clippy -D warnings ok, pre-push-check ok; no new test (faithful move)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: kept 2 of 6 lines (33%, under 60%: the draft wrapped the body in KNOWN.with, the move takes the ledger as a parameter); costliest edits: ledger parameter, ref on key, let k for the key
