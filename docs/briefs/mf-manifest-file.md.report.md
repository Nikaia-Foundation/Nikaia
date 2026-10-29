Result: done
Files: crates/nikaia-std/src/tools/manifest.nika crates/nikaia-std/src/tools/lowered/manifest.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/manifest.rs docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: manifest: Manifest and its readers into Nikaia (MF)
Body: `manifest.nika` gains `ManifestItem` (the document in plain words: section, name, key, is-table, `to_string` text, `as_str`, `as_integer`, and `machine`, `Target::parse`'s answer passed in from Rust), `ManifestDependency` (shape, text, path), `ManifestFile`, and `manifest_parse` building it with every refusal (`[build]` keys, `libraries`' four, `codegen`, dependency shapes), byte for byte, plus `manifest_empty` (version `0.0.0`), `manifest_has_build_section`, `manifest_foreign_crates`. Rust `Manifest` holds a `ManifestFile` and the `Dependency` map derived from it once; `parse` walks `toml::Value` into items (`items_of`), the readers keep their signatures and forward. Still Rust: removing `type` from a Rust crate's table when `Dependency::Rust` is derived (rebuilding an inline-table text in Nikaia would need TOML key quoting), `codegen_for`'s `toml::Value` (rebuilt from the text with `v = <text>`), and `setting` (returns a borrow of flag, map or default). `[package]` is held as text; its `name` and `version` are words in their own fields, since `package_name` returns `as_str`. No caller changed.
Part: #125
Pushed: eb90dc36311f21b7d8ccac690f8258693b0f6a27
Tests: --lib manifest:: 15 passed; it filters manifest project describe package foreign reentrancy library codegen described overflow map_ledger map_seeds modules result_source system_libraries + guard set: 408 passed; clippy clean; pre-push-check ok; lower-std fixpoint (only lowered/manifest.rs and the index changed); tests/compiler --both-settings: 242 passed, 1 failed: agent_script (environment: agent.nika ESCALATEs at under 3 GB free disk, 2896 MB; reproduced by hand)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: manifest: `Manifest`'s parse and readers are Nikaia (`ManifestFile`, `manifest_parse`); the `toml` crate stays behind the adapter (MF, #125)
Owner: none
Disk freed: 2.9 GB (target/nikaia-project-tests removed at 2.8 GB free; now 5.8 GB free; new entry over 100 MB this run: run-bundle/194302eb 285 MB, removed with it)
Rs2nika: kept 12 of 180 lines; costliest: the draft's `toml::Value` walks (`get`/`as_table`/`and_then`) all rewritten as scans over `ManifestItem` (other 46, control flow 42); `impl Manifest` methods redone as free functions over `ManifestFile` (signature/ref 22); `let x = opt ?? return` / `match value.as_table()` shapes replaced by `continue` filters (return 21)
