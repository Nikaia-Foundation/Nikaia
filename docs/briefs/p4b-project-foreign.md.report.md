Result: done
Files: crates/nikaia-std/src/tools/project_members.nika crates/nikaia-std/src/tools/lowered/project_members.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/project.rs crates/nikaia/src/modules.rs crates/nikaia/tests/it/described_entries.rs crates/nikaia/tests/it/describing.rs crates/nikaia/tests/it/errors_lowering.rs crates/nikaia/tests/it/send.rs docs/selfhosting-plan.md scripts/self_hosting.py assets/badges/self-hosted.svg
Commit: project: a build's foreign boundary into Nikaia (P4b)
Body: `Foreign` becomes the Nikaia struct `ProjectMembersForeign` (`project.rs` re-exports it as `Foreign`, so `Around` holds the lowered type); `crates_beneath`, `Foreign::of` (`project_members_foreign_of`, async, called through `block_on` by the Rust `foreign_of`), `packages` (a method in an `impl`), `sources_that_moved`, `proofs_digest`, `description_at` and the old `Default` (`project_members_no_foreign`, `Ledger::blank`) move to `project_members.nika`. Manifests are read through P4a's `ProjectMembersHost::manifest_at`; a new `ProjectMembersForeignHost` (Rust `ForeignHost`) gives `sha256_hex`, `read_beside`, `absorb` (by value, returns the ledger), `empty_ledger` and `crate_root`. `Foreign::library` stays Rust as the free function `foreign_library` (an inherent method on a type of `nikaia_std` cannot be written in `nikaia`). The draft's map/set chains became loops: `BTreeMap` has no `iter` and `BTreeSet` no `extend` in `std`, and `fs::read` gives `Bytes`, copied into the `Vec[u8]` that `sha256_hex` takes (as `sysroot_cache.nika` does). The Rust `it` tests that called `Foreign::of`, `.library()` and `Foreign::default()` were changed only so they still pass. No #909 or #916 use.
Part: #125
Pushed: 932e4da (pushed by drive.nika)
Tests: it `foreign describe contracts_across proofs NK2504 rust_dep` + guard set 129 ok; `send:: errors_lowering:: describing:: described_entries::` 73 ok; `--lib project::` 10 ok; `nikaia test --project tests/compiler` 223 ok; clippy clean; lower-std fixpoint (second run wrote nothing new); self-hosting 80.8 %
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: kept 21 of 79 lines (under 60 %); the costliest edits: (1) no `--prefix project_members_` and no host trait in the draft, so every signature and call changed (prefixed names, `manifests`/`host` parameters, `Manifest::read` turned into `manifest_at`, `orchestrator::cache::sha256_hex`/`Ledger::*`/`crate_root` turned into host calls); (2) `BTreeSet.extend`/`.push` and `BTreeMap.iter()` chains, which are not in `std`: rewritten as loops with `insert`; (3) `fs::read` gives `Bytes`, copied byte by byte into a `Vec[u8]`, and `read_dir` gives names, not `DirEntry` paths
