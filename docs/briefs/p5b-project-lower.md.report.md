Result: done
Files: crates/nikaia-std/src/tools/project_lower.nika crates/nikaia-std/src/tools/lowered/project_lower.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/project.rs crates/rs2nika/convert/rs2nika.names scripts/self_hosting.py docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: project: a lowering's steps, its cache and its ledger into Nikaia (P5b)
Body: `lower_reading` keeps the layout, the reads and the choices, then forwards to `project_lower_reading` (tools/project_lower.nika) over `ProjectLowerHost`, implemented by `LowerHost` in project.rs (program, cache, choices, export, foreign, book, facts, published, proofs in `RefCell`s, an anyhow refusal kept in `failed` and handed back unchanged, as M3b). Nikaia holds the order of the steps, the cache hit's replay of the warnings, the published-contract merge over `Ledger` (`project_lower_published`), the allowlist warning, the cache record and the notes; Rust builds `Lowered` from `ProjectLowerOut`, the proofs and the export. Host choices: `key_source` and `dependencies` are not trait methods (only the host's cache and `changed_contracts` use them, so it keeps them), and `sources` is `Vec[fs::Path]` (PathBuf, lossless) rather than `Vec[String]`. rs2nika.names gains `[forms project_lower_]` (could_promise, lost_sync, in_the_wrapper, eprint, unused_reads, changed_contracts, parse_ledger) and `[hosts project_lower_]`. Self-hosting share: 82.1 % (project_lower.nika 116 code lines).
Part: #125
Pushed: 6a65c91b72058a26adc5fdc7f1e720905cdc9f9b
Tests: --lib project:: (10 passed); it filters project cache lower ledger prove proofs export library described foreign allowlist reads warnings + guard set (580 passed; 2 failed under load at the build-success assert, both pass rerun alone: project::a_program_another_compiler_lowered_is_lowered_again, build_time_dependencies::the_build_time_program_links_no_c_library); nikaia test --project tests/compiler (246 passed, 1 failed: view_of_a_field_leaves_another_field_free, another agent's untracked test); clippy clean; pre-push-check ok; lower-std fixpoint (only lowered/project_lower.rs and the index of mine)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0 (no clone; cache 2.1G, 4.4G free; new entries over 100 MB: rlib df8bcfe6 614M, af187e66 264M, run-bundle 194302eb 285M, build-time-parsers 768d2772 468M, build-time-bundle b6221fc9 482M)
Rs2nika: kept 11 of 94 lines; costliest: the cache, check and emit blocks (886-1046) replaced by five host calls, the Rust cache/export matches replaced by `open_cache`/`export`/`cached`/`record`, the in-place `get_mut` merge rewritten as get-clone-insert with an index loop (no `zip`, no `std::mem::take`)
