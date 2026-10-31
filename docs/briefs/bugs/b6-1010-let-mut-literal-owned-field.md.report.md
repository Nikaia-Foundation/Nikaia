Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs crates/nikaia-std/src/tools/lowered/check_defaults.rs tests/compiler/tests/a_let_mut_literal_into_an_owned_variant.nika tests/compiler/tests/a_let_mut_literal_into_an_owned_variant.stdout
Commit: text tiers: a let mut literal assigned another literal stays one binding of literals
Body: The defect was not the join over the `if`: `let mut why = "a"; why = "b"; held.push(E::V(why))` failed the same way, while `let why = "a"` and an unassigned `let mut` ran. A plain reassignment dropped the binding's literal mark, so its keeping in an owned field never declared the `let` `String`. Now an assignment of another literal (also an `if`/`match` of literals) keeps the mark: the binding is declared `String` and the emitter builds each assigned literal (ADR-282 D17, one binding). The struct-field shape was refused NK1106 before, the variant shapes reached rustc E0308. Side effect in tools: `check_defaults_fold`'s `owner` (a literal reassigned, then lent to a `String` parameter) is now declared `String`, as `let p = "x"; g(p)` already was.
Fixes: #1010
Tests: new a_let_mut_literal_into_an_owned_variant (if, match arm, struct field) failed on the unfixed compiler (NK1106 on the struct field; the variant probe rustc Mismatched types); passes now; 44 tests named tier/literal/let_mut green at both settings; sequences guard figure unchanged (gate guards green)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A `let mut` text literal assigned another literal and then kept in an owned field or variant is declared owned text (#1010), instead of reaching rustc with a type mismatch.
Owner: none
Disk freed: 0
Rs2nika: none
Pushed: a788d16db25bb30250b0b4c417d5d317e6e9ac5c
