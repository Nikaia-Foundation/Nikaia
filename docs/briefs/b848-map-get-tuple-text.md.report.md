Result: done
Files: crates/nikaia-std/src/index.rs tests/compiler/tests/map_get_tuple_fallback_bool_text.nika tests/compiler/tests/map_get_tuple_fallback_bool_text.stdout tests/compiler/tests/map_get_tuple_fallback_i64_i64_text.nika tests/compiler/tests/map_get_tuple_fallback_i64_i64_text.stdout tests/compiler/tests/map_get_tuple_fallback_i64_text.nika tests/compiler/tests/map_get_tuple_fallback_i64_text.stdout tests/compiler/tests/map_get_tuple_fallback_i64_text_text.nika tests/compiler/tests/map_get_tuple_fallback_i64_text_text.stdout tests/compiler/tests/map_get_tuple_fallback_text_i64_text.nika tests/compiler/tests/map_get_tuple_fallback_text_i64_text.stdout tests/compiler/tests/map_get_tuple_fallback_text_text_text.nika tests/compiler/tests/map_get_tuple_fallback_text_text_text.stdout
Commit: std: m.get(k) ?? (tuple) answers views for every tuple that holds text
Fixes: #848
Tests: 6 new tests/compiler programs; failed with index.rs stashed (6 FAILED), pass fixed; project 108 ok at --both-settings
Gate: green
Deviation: none (macro or_tuple! replaces the two #846 impls; 3 pairs + 7 triples)
Found: none
Needs: none
Changelog: std: m.get(k) ?? (tuple) answers views for every tuple of arity 2 and 3 that holds text
Owner: none
Disk freed: 0
Pushed: 39ebe788434b26ad2cf4d47bfe4515e05d888c15
