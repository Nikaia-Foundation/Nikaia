Result: done
Files: crates/nikaia-std/src/tools/check_boundary.nika crates/nikaia-std/src/tools/check_arguments.nika lowered/check_boundary.rs check_arm_leaves.rs check_arguments.rs
Commit: e8877caf An Expr read off a place is a view, not a copy (Part of #655); then check_arguments: the detour check_arguments_at goes (Part of #655)
Fixes: none
Pushed: yes (both)
Tests: it range_spelling shipped_ledger every_std_module dependency_ledger element_views views_kept nullable match_arms copies (118 ok); nikaia test --both-settings tests/compiler 134 ok; clippy clean. Cause: an enum of another file or a ledger has no enum_payloads entry, a_payloadless_enum answered true. Repro was a two-file project (a program test needs a second file; the lower-std diff is the test, not run against the unfixed compiler as a test).
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: An enum declared in another file of the package, or a library's, with payload variants is no longer taken for a word (false NK1257 on `let h: ref E = given[0]`).
Owner: none
Disk freed: 0
