Result: done
Files: crates/nikaia-std/src/tools/comptime_tree.nika crates/nikaia-std/src/tools/lowered/comptime_tree.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/comptime_run.rs crates/rs2nika/convert/rs2nika.names
Commit: comptime: a build-time program of several files assembled in Nikaia (K3a part 1)
Body: `lowered_units`, `without_foreign`, `dependency_modules` and `by_package` of comptime_run.rs are `comptime_*` in comptime_tree.nika, over the new `ComptimeLowerHost` (units by index; the emitter, `Needs`, `Ledger::infer_package`, the trust and the source map stay Rust in comptime_run.rs's `LowerHost`); the Rust functions are adapters with their signatures, so no caller changed. Texts byte for byte; the offset handed to `emit_unit` is `rust.len()`, bytes in the lowering, as modules.nika's `emit_unit` does (Part I has no `String.len()`). `by_package`'s sort and dedup is a `BTreeSet` (the same order); a `BTreeSet` has no `iter`, so its map is filled by a `for`. `without_foreign` walks the items as `comptime_drop_readers` does, `retain` for the methods; `Parsed::keeping` stays as the copy. rs2nika.names gets `[forms comptime_]` and `[hosts comptime_]`. Self-hosting: 82.2 % of the toolchain (scripts/self_hosting.py, working tree).
Part: #125
Tests: it comptime grammar_at_build_time + guard set 82 passed; clippy clean; lower-std fixpoint unchanged (comptime_tree.rs md5 same after rebuild)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Rs2nika: none
Disk freed: 0
Pushed: 5bf28f577078b770deaf332a44465f7e1683d9df
