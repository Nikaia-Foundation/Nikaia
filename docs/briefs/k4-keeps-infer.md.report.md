Result: done
Files: crates/nikaia-std/src/tools/keeps.nika crates/nikaia-std/src/tools/lowered/keeps.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/contracts/keeps.rs crates/nikaia/src/contracts/mod.rs crates/nikaia/src/contracts/sync.rs crates/nikaia/src/emit/mod.rs crates/nikaia/src/views.rs crates/nikaia/src/modules.rs assets/badges/self-hosted.svg
Commit: contracts: keeps' inference moves into keeps.nika (K4)
Part: #125
Body: `infer`, `the_impls_own`, `uses_of`, `by_value`, `no_uses`, `holds_either`, `Column`/`Keeper`/`ImplRow` are Nikaia now (`keeps_infer`, `keeps_uses_of`, `KeepsColumn`, `KeepsKeeper`, `KeepsImplRow`, `KeepsFound`, trait `KeepsHost`); Rust keeps `keeping_statement`, a `Host` impl, and `infer` as a forward. Dead `emit::holds_view` and `views::fields_of` removed; `Noted` got a hand-written `Default` (lowered column has none). Loops with counters replace the `enumerate().flat_map()` chains except for the trait positions.
Pushed: local 08de5ea7
Tests: lib contracts:: 36 ok; it keeps:: lending:: handed_over:: tether_keep:: contracts:: modules:: 167 ok; both NK1264/NK1265 programs ok in a scratch project; lower-std fixpoint clean; clippy clean for my files (run with `-A clippy::needless_as_bytes`: another agent's untracked lowered/sysroot_package.rs fails it); contracts files unchanged. pre-push: index/fmt/adr-refs/badges green on the staged copy; `version` red only because the working tree holds other agents' .nika (share differs there).
Gate: green
Deviation: `keeps_infer` takes one more parameter, `indexes: KeepsIndexes` (the two method indexes), built by the Rust forward: a struct holding `ref` fields cannot be built from locals in Nikaia (NK1106) and storing a `ref` parameter in KeepsBody is NK2302 unless the struct comes in by value. Committed with plain `git commit` of the staged index, not `commit.nika`: it would `git add` all of lowered.rs, which also holds another agent's uncommitted sysroot_package hunks (I staged only my hunk).
Found: untracked `tools/sysroot_package.nika` (another agent) lowers to Rust that fails clippy (`needless_as_bytes`, 7 sites) and briefly failed `lower-std`/cargo; not touched.
Needs: none
Changelog: keeps inference (infer, the impls' own column, the per-function walk's driver) is written in Nikaia, `tools/keeps.nika`, behind a `KeepsHost`.
Disk freed: none (cache 1.3G, disk 6.6G free)
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
