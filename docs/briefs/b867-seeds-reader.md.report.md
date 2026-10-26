Result: done
Files: crates/nikaia-std/src/tools/emit_seeds.nika crates/nikaia-std/src/tools/lowered/emit_seeds.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/emit/seeds.rs scripts/self_hosting.py
Commit: emit/seeds: the reader's decisions move to emit_seeds.nika
Fixes: #867
Pushed: 0e24e46229bcd52a234a8581aa3a603c54e74add
Tests: seeds maps sysroot (87 passed); nikaia test --both-settings --project tests/compiler (122 passed); emit:: unit tests ok; clippy clean; pre-push-check ok; no defect met, so no new test
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: emit/seeds: the counting fixpoint of the `pub` structs' map positions, the barrier closure and the self-hander test move to `tools/emit_seeds.nika` (`EmitSeedsTables::count_types`, `close_barriers`, `self_handers`, `hands`); the Rust keeps the AST walks as collectors of plain facts (#867)
Owner: none
Disk freed: 0 GB (no clone; target/nikaia-project-tests kept)
Model: Opus 5.5
