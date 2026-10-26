Result: done
Files: crates/nikaia-std/src/tools/emit_seeds.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/emit_seeds.rs crates/nikaia/src/emit/mod.rs crates/nikaia/src/emit/seeds.rs docs/selfhosting-plan.md scripts/self_hosting.py assets/badges/self-hosted.svg
Commit: emit/seeds: the Seeds tables and type queries move to emit_seeds.nika (EmitSeedsTables)
Fixes: none
Pushed: 6cd8fead5f0831d47ab94cd2122d683e9541c051
Tests: it seeds maps + guard set 72 ok; clippy clean; tests/compiler both-settings 119 ok, 1 FAILED drive_actions (scripts/drive.nika dry-run, from 7970db8, not touched here)
Gate: green (drive_actions red, not from this change)
Deviation: root_name stays Rust (walks ast::Expr, used only by the reader's hands); rs2nika draft not used, hand-written
Found: tests/compiler drive_actions fails on develop since 7970db8 (drive.nika tidy)
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Part: #867
