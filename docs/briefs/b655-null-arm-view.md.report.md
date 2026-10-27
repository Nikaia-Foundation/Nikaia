Result: partial
Files: crates/nikaia-std/src/tools/check_arguments.nika crates/nikaia-std/src/tools/check_arm_leaves.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/lowered/check_arm_leaves.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs tests/compiler/tests/a_null_arm_beside_an_element_view_is_an_optional_view.nika tests/compiler/tests/a_null_arm_beside_an_element_view_is_an_optional_view.stdout
Commit: A null arm beside a view of an element is an optional view
Part: #655
Pushed: da5a86ea46b05376deac621f1095e711da60470c
Tests: probes (a) let+if, (b) let+match and `-> ref E?` result print 2/0/2/0/2/true; (c) controls ok; test program fails on the unfixed binary (NK1103 x2, NK1104, seen before the fix); tests/compiler --both-settings 134 ok; it filters element_views views_kept nullable match_arms + guard set 107 ok; clippy, fmt, pre-push clean
Gate: green
Deviation: check_arguments_at is kept (comment corrected): the detour cannot go, see Found
Found: `let h: ref Expr? = if c { given[at] } else { null }` over `ref Vec[Expr]` is NK1257 "An Expr read out of a place is a copy" (the ledger counts Expr, Ty as copied, a_ledger_copies), also as a struct field (NK1106); probe: `let here: ref Expr = given[at]` in a tools file gives NK1257; with a user struct it works
Needs: owner/architect decision whether Expr and Ty may be viewed (ADR-354 D2 vs ledger copies) before check_arguments_at can go
Changelog: A `null` arm beside an element read of a `ref` list, in an `if` or `match` under `ref E?` (a `let`, or a function result), is an optional view: it lowers to `Some(&..)`/`None`, no longer NK1103/NK1104.
Owner: none
Disk freed: 0
