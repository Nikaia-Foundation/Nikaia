Result: done
Files: crates/nikaia-std/src/tools/migrate_views3.nika crates/nikaia-std/src/tools/lowered/migrate_views3.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/migrate.rs crates/nikaia/src/fmt.rs crates/nikaia/src/lib.rs crates/nikaia/src/main.rs tests/compiler/tests/migrate_views3_rewrites_each_form.nika tests/compiler/tests/migrate_views3_rewrites_each_form.stdout tests/compiler/tests/migrate_views3_writes_keep_from_the_ledger.nika tests/compiler/tests/migrate_views3_writes_keep_from_the_ledger.stderr tests/compiler/tests/migrate_views3_writes_keep_from_the_ledger.stdout assets/badges/self-hosted.svg
Commit: migrate: `nikaia migrate views-3` drops `ref` and writes `keep` from the ledger (ADR-views-3 D4)
Body: The rewriting is Nikaia (tools/migrate_views3.nika: tokenizer, the D4 rewrites, `extern` blocks and `extern fn` signatures, comments and strings left alone, only the ```nika blocks of Markdown); `keep` comes from the checker's own inference (keeps.nika), run after `ref` is dropped, across a package's files. Rust added for #557 / #125 (the thin door, #1214): migrate.rs `run`, `Package`, `keeps_in_package`, `keeps_alone`, `keeps_of`, `characters`, `files_under`; fmt.rs `sources_under` made pub(crate); main.rs `Command::Migrate` and its dispatch in `run` and `project_command`. Fields of `mut self` go through `ref Vec` parameters in the tool (NK1131, std's convention until std is migrated). A package with dependencies is refused (their ledgers would be needed). Built by v9, checked and committed by v9b.
Part: #1213
Tests: migrate_views3_rewrites_each_form and migrate_views3_writes_keep_from_the_ledger pass at --both-settings (scratch project holding just those two; `nikaia test` has no --filter); both FAILED on target/release/nikaia without the tool; blessed files reviewed, none empty where output is expected; cargo fmt, clippy -D warnings, --lib fmt::, lower-std fixpoint clean, pre-push green; full suite left to the driver
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: `nikaia migrate views-3 PATH...` rewrites `.nika` files and the Nikaia blocks of Markdown to views-3: `ref` is dropped outside `extern`, `ref mut` parameters become `mut x: T`, and `keep` is written where the body keeps a parameter.
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 763bf9e4e6aa77f8ff340a3f70ab998b406424d7
