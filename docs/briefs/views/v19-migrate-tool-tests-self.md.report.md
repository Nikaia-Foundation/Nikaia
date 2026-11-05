Result: done
Files: crates/nikaia-std/src/tools/migrate_views3.nika crates/nikaia-std/src/tools/lowered/migrate_views3.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/migrate.rs tests/compiler/tests/migrate_views3_reads_package_tests_with_the_package.nika tests/compiler/tests/migrate_views3_reads_package_tests_with_the_package.stdout tests/compiler/tests/migrate_views3_writes_keep_self.nika tests/compiler/tests/migrate_views3_writes_keep_self.stdout
Commit: migrate: `nikaia migrate views-3` reads a package's tests with the package's ledger and writes `keep self` for a consumed by-value `self` (ADR-views-3 D2)
Body: (2) A file under a package's `tests/` is read as `nikaia test` reads it: the package's own ledger (its `src/`, `ref` dropped, inferred as one graph) is absorbed under the package's name into the library, and the test is inferred against it, so `json::parse(text)` no longer makes `line(text)` fail closed to `keep`. (3) The keep inference never decides `self` (keeps.nika leaves `self.field` out); the checker does: an inherent method with a bare `self` whose body draws the NK1131 "give away `self.field`" refusal is named by its start offset, and `migrate_views3_code` writes `keep self` there only where the receiver was written bare in the original (`migrate_views3_selves`, read before `ref` goes, since `ref self` and `self` read alike after it); `self` that only looks and an old `ref self` stay as they are. Rust added for #557 / #125 (the views-3 door, #1214): migrate.rs `keeps_in_package` reads `tests/`, `keeps_of` takes the library and hands over the checker's findings, `Package::Keeps` passes `by_value`. Both steps are in one commit (they share both files). The json test still shows `show(keep v: json::Value)` in five tests: a `match` over a plain parameter counted as kept, v17's finding (1), v18b's checker rule; the test's .stdout loses those lines when that lands.
Part: #1213
Tests: migrate_views3_* (4 programs) ok at --both-settings; migrate_views3_writes_keep_self FAILED on the HEAD binary before the change; the json case checked by hand on the HEAD binary (6 fail-closed `line(keep text)`/`number(keep text)`, now none); lower-std fixpoint clean; cargo fmt, clippy -D warnings, pre-push green
Gate: green
Deviation: none
Found: none
Needs: target/fast/nikaia at 12:30 had been built from another agent's uncommitted state (v18 by the time): its lower-std rewrote 40 lowered files into Rust that does not compile; I restored those files (clean before me) and rebuilt from HEAD. v18b: re-bless migrate_views3_reads_package_tests_with_the_package.stdout when the matched-parameter rule lands (the `show(keep v` lines go)
Changelog: `nikaia migrate views-3` reads a package's tests with the package's ledger, and writes `keep self` where an old by-value `self` is consumed.
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 1f842fa70755743386fda19db2a4149e60ae8bbb
