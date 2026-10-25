Result: part 1 of E11-4b built: blocks, stmt, stmt_written, makes_a_map in tools/emit_stmt.nika; a_stmt (about 950 lines) still Rust behind host.a_stmt
Files: crates/nikaia-std/src/tools/emit_stmt.nika crates/nikaia-std/src/tools/emit_host.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/emit_host.rs crates/nikaia-std/src/tools/lowered/emit_stmt.rs crates/nikaia/src/emit/host.rs crates/nikaia/src/emit/mod.rs scripts/self_hosting.py docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: emit: blocks and statements in Nikaia, a_stmt through the host (E11-4b part 1)
Part: #583
Tests: it emit:: sysroot (15) range_spelling shipped_ledger contracts::every_std_module dependency_ledger, lib emit::, tests/compiler --both-settings (93): all ok; clippy and pre-push clean
Gate: green
Deviation: a_stmt not moved (950 lines, about 25 callees: keep_plan, ty_counted, tethered_type, map_key, index_expr, around_value, handed_back...), left for a fresh agent; no Fixes line. target/fast/nikaia was stale (refused NK1301 on committed files), rebuilt at HEAD first.
Found: none new; no test added (behaviour unchanged)
Needs: part 2, a_stmt, as its own brief
rs2nika: kept 0 of 116 lines (draft stopped at the first let-chain); fixed by hand: everything
Rust lines removed: about 290
Changelog: none (internal move, version raised by supervisor)
Pushed: rejected
