Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs tests/compiler/tests/a_text_literal_local_changed_in_place_is_text.nika tests/compiler/tests/a_text_literal_local_changed_in_place_is_text.stdout
Commit: text tiers: a literal bound by a let is declared String, never a String? taken from the program (#943)
Body: The cause was not the tools profile: any `String?` field, parameter or result in a file (the first written `String` node `string_type` found, `plain_string` ignoring nullability) became the type of every `let` of a literal kept as text, so `let mut out = "a"` was `String?`; `nikaia run` fails the same once the program has a `String?`. `string_type` now skips nullable nodes.
Fixes: #943
Pushed: 417fc0476da144edb8bbfd2b62704e82c6d401ff
Tests: new test fails on the unfixed compiler (NK1125+NK1104), passes now; tests/compiler --both-settings 248 ok, 1 failed (fmt_keeps_a_nested_tuple_closure_whole, from 7c1643d: prints 1 without newline, .stdout has one); it nullable:: text_tiers, range_spelling, shipped_ledger, contracts::every_std_module, dependency_ledger, fmt::every_nika_file ok; probe put into scratch tools/modules.nika passed lower-std, restored by name; clean lower-std changes only lowered/text_tiers.rs; clippy clean; pre-push-check ok
Gate: green
Deviation: none
Found: tests/compiler/tests/fmt_keeps_a_nested_tuple_closure_whole fails (print without newline vs .stdout "1\n", 7c1643d)
Needs: none
Changelog: A literal bound with `let` is no longer typed `String?` when the program writes a `String?` anywhere (#943).
Owner: none
Disk freed: 0
Rs2nika: none
