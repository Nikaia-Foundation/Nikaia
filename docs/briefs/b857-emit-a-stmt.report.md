Result: done, `a_stmt` moved whole to `emit_stmt_a_stmt` (tools/emit_stmt.nika); host `a_stmt` and Rust `a_stmt` and 40 dead helpers gone
Files: crates/nikaia-std/src/tools/emit_stmt.nika crates/nikaia-std/src/tools/emit_host.nika crates/nikaia-std/src/tools/lowered/emit_stmt.rs crates/nikaia-std/src/tools/lowered/emit_host.rs crates/nikaia/src/emit/host.rs crates/nikaia/src/emit/mod.rs scripts/self_hosting.py docs/selfhosting-plan.md
Commit: E11-4b part 2: Emitter::a_stmt moves to Nikaia (emit_stmt_a_stmt)
Fixes: #854, #857
Part: #583
Tests: emit:: guard set ok; tests/compiler both settings 94 ok; sysroot 15 ok; clippy clean
Gate: green
Deviation: none; host gained given_up_unmuted, given_up_rebound
Found: none; rs2nika not used (hand-written)
Needs: none
Changelog: The statement writer of the lowering is written in Nikaia.
Rust lines removed: about 1540 (net)
Pushed: rejected
