Result: done
Files: crates/nikaia-std/src/tools/ledger_items.nika crates/nikaia-std/src/tools/lowered/ledger_items.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/contracts/mod.rs docs/selfhosting-plan.md assets/badges/self-hosted.svg
Commit: contracts: the stored-code fold moves into ledger_items.nika; the dead LedgerOps::candidates goes (C1b)
Body: `ledger_items_stored` merges every unit's stored code (resolved and free extended, unresolved or-ed) with `CheckStoredCode::empty` via `or_insert_with` (CheckStoredCode has no Default). The map has no `drain`, so the loops borrow and clone the sets; the lowering takes a slice, so Rust passes `&checked...collect::<Vec<_>>()`. `LedgerOps::candidates` (trait and impl) had no caller and is deleted. Plan row C1: C1b built, C1c = `lookup` waits on #1107. Self-hosting share 83.3 % of the toolchain.
Part: #125
Pushed: local 547e64622e0b7c06d8f518cc0bbea2c3350557ca
Tests: it -- throws locks touch ledger contract 211 ok; lib contracts:: 36 ok; five tests/compiler programs named throws/stored/view_stored in a scratch project, both settings, ok; no nikaia.* ledger diff in tests/compiler; clippy 1.97 clean (1.99.1 not run)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: The stored-code fold of the contracts ledger is written in Nikaia; the unused LedgerOps::candidates is removed.
Owner: none
Disk freed: 0 GB
Rs2nika: kept 2 of 9 lines (the draft inlines the fold, the move is a function: signature, loops over a unit's map instead of flat_map/fold, `.iter().cloned()` for extend)
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
