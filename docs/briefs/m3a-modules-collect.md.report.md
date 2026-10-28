Result: done
Files: crates/nikaia-std/src/tools/modules.nika crates/nikaia-std/src/tools/lowered/modules.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/modules.rs docs/selfhosting-plan.md
Commit: modules: collecting the units into Nikaia (M3a)
Body: The walk that finds a program's files moves into tools/modules.nika: modules_collect_with/_one, package_at, read_unit, one_namespace, check_imports, the project-root search for foreign_beside and described_beside, modules_qualify, modules_sources_of and modules_believed (shipped ledger read, stale check, published), with ModulesRefusal for the refusals. Parsed stays Rust behind ModulesHost (parse_unit keeps the file in an arena and answers by index; declared, use_refused, foreign_crates over it) and ModulesLedgerHost (sha256_hex over text, read_beside); collect, collect_with, collect_one, Unit::qualify and package_ledger* keep their Rust signatures as adapters, and the inference (Noted, Checked: Rust types) stays in package_ledger_checked. Nikaia's read_dir gives names sorted by bytes, and Rust sorted PathBufs that differ only in their last component, which compares by the same bytes, so the order with `.`, `_` and digits is the same. Every refusal text is unchanged; an unreadable file keeps "cannot read PATH" as context, its cause is now std's IoError text rather than io::Error's (no test reads it).
Part: #125
Pushed: 1890fc9dc1f3066c2eec7b4b674f4097f76477d5
Tests: nikaia test --project tests/compiler 243 passed; it filters modules package use_ import namespace collect + guard set 155 passed; --lib modules:: (no unit tests in the module) passes; clippy clean; lower-std fixpoint; self-hosting 82.1 %
Gate: green
Deviation: none
Found: another session reset the working tree at 22:41 (reflog `reset: moving to HEAD`) and a lower-std rewrote lowered/modules.rs and lowered.rs without this change; regenerated
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: kept 12 of 109 lines; costliest: (1) `--prefix modules_` not applied to the functions and types (collect, package_at, read_unit, Unit, Standing: every signature and call renamed); (2) the draft parses in Nikaia (parser::parse_to_ast, parsed.interner, Unit holding Parsed) where the brief's host-by-index shape was needed, so a host parameter in every function; (3) Rust error and path API kept (throw refuse(..), error.context, ancestors().skip(1).find, as_os_str, PathBuf::from) and the message's `with` turned into `with_`
