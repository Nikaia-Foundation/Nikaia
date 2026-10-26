Result: partial
Files: assets/badges/self-hosted.svg crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/emit_seeds.nika crates/nikaia-std/src/tools/lowered/emit_seeds.rs crates/nikaia/src/emit/host.rs crates/nikaia/src/emit/mod.rs crates/nikaia/src/emit/seeds.rs docs/selfhosting-plan.md scripts/self_hosting.py
Commit: The seeds writer (`Place`, `Seeds::write`) moves to tools/emit_seeds.nika
Part: #867
Pushed: 38697bfa63ac1778847f2c2916a307f12b2206e8
Tests: it -- seeds maps + guard set 72 ok; lib emit:: ok; nikaia test --both-settings tests/compiler 117 ok; clippy clean. Left: reader half (read_with, barriers, count, key, ...) stays Rust; Seeds::declared/applied duplicates dropped for emit_item's.
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: Seeds writer in Nikaia: the hidden-seed-parameter writer of emit is tools/emit_seeds.nika (#867 part)
Owner: none
Disk freed: 0
rs2nika: kept about 40 of 330 lines (draft used Option-closure forms and `impl EmitState`; rewritten by hand on scalars); fixed by hand: byte/at state into a run struct, shape as groups/within vectors, `match` on a temporary needs a `let` (lowering borrows the temporary: E0716)
