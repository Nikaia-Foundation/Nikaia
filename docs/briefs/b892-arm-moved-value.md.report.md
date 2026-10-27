Result: done
Files: crates/nikaia-std/src/tools/check_arm_leaves.nika crates/nikaia-std/src/tools/lowered/check_arm_leaves.rs tests/compiler/tests/a_value_moved_in_an_arm_is_handed_over.nika tests/compiler/tests/a_value_moved_in_an_arm_is_handed_over.stderr tests/compiler/tests/a_value_moved_in_an_arm_not_read_again.nika tests/compiler/tests/a_value_moved_in_an_arm_not_read_again.stdout
Commit: check: a value moved in an arm of an `if` or `match` is handed over (NK2105)
Body: `hands_over` walks the tails of an `if` with an `else`, a block and each `match` arm, as `narrowed_reads_given_back` does, so a name or field path moved in an arm and read after the statement is NK2105 at every caller (a `let`, an argument a function keeps, a struct field, a list, a variant). The read is looked up at the tail's own statement; the taking stands at the whole statement, so a read in the other arm is not after it. A choice of text (`String`, `String?`) is left to ADR-282 D13 (view or copied arm, nothing moves), which keeps `let x = if c { h.name } else { "none" }` over a `ref` parameter and a text moved in an arm and read later running. Steps 1 and 2 in one commit; no tools file hit NK2105 (`lower-std` changed only check_arm_leaves.rs).
Fixes: #892
Pushed: 74ff66f24494f1a4c0bcb9115c5d72c5632485c2
Tests: new a_value_moved_in_an_arm_is_handed_over (if, match, proved, kept argument, struct field: 5x NK2105) and a_value_moved_in_an_arm_not_read_again; the if/match/proved probes passed `nikaia check` on the unfixed binary (the test file itself not run there); `nikaia test --both-settings --project tests/compiler` 182 ok; it: handed_over nullable defects_found_running pausing_lambdas text_tiers typecheck articles + guard set 308 ok; clippy clean
Gate: green
Deviation: none
Found: `fn keep(p: P) -> String { return p.name }` is lowered to take `&P`, and then `keep(if c { t } else { P { name: "b" } })` (t a lent parameter) fails in rustc with mismatched types (`&if c { t } else { P {..} }`); not touched by this change (the lent path never calls hands_over), not probed on the old binary
Needs: none
Changelog: A value moved in an arm of an `if` or `match` is handed over: reading it after the statement is NK2105, as after `let s = t` (#892).
Owner: none
Disk freed: 0
