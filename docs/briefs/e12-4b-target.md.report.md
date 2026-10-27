Result: done
Files: crates/nikaia-std/src/tools/emit_build.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/emit_build.rs crates/nikaia/src/bounds.rs crates/nikaia/src/emit/mod.rs crates/nikaia/src/manifest.rs crates/nikaia/src/project.rs crates/nikaia/tests/it/project.rs docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: emit: the build target and the bounds and overflow switches into Nikaia (E12-4b)
Body: `Target` (parse, is_bare_metal, name, triple, has_threads, has_runtime, unbuildable), `BoundsChecks` and `OverflowChecks` (parse, name) move into `tools/emit_build.nika`; `emit` and `bounds` re-export them. A `ref String` payload lowers to `Rust(&'a str)` with a lifetime parameter on the enum and without `Copy` (as `ledger.nika`'s `LedgerLine`), so `Target::Rust(i64)` carries the index into `emit_build_rust_targets()` and `name`/`triple` read the list; `ALL` became `emit_build_targets()`. `Target::host`/`Default` stay Rust as `emit::host_target()` and `emit::default_target()`; `Build` stays Rust. `parse` throws the old refusal text byte for byte, mapped back through `switch`; `unbuildable` now gives `Option<String>`. The switches' `Default` and `PartialOrd`/`Ord` were unused beyond `::default()`, which callers now spell `::Kept`. Both brief steps in one commit: they share `emit_build.nika` and its lowered file.
Part: #125
Tests: it `target parallelism bounds overflow optimization` 124 passed; --lib `emit:: bounds:: project:: manifest::` ok; `nikaia test --both-settings --project tests/compiler` 191 passed; lower-std fixpoint (only emit_build.rs and the index line); self_hosting.py 79.6 %
Gate: green
Deviation: none
Found: rs2nika garbles a `refused!` whose string continues with `\` over lines: the draft of `Target::parse` swallowed `is_bare_metal`, `name`, `triple` and half of `has_threads` into one string line (the main cost of the score below)
Needs: none
Changelog: none
Owner: none
Disk freed: 0 GB (no clone); cache target/nikaia-project-tests 2.5G, disk 18G free; new entry over 100 MB: run-bundle/65d964130cc06fb2 294 MB
Rs2nika: kept 45 of 81 lines; costliest edits: the five functions lost in the garbled `refused!` string rewritten by hand; `Rust(String)` to `Rust(i64)` with `triple()` reading the list; `-> &'static str` / `Some(..)` spelled `-> ref String` / `String?`
Pushed: d1ea0728aaece3ef9c71c671429192e00b48b900
