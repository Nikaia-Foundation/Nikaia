Result: done
Files: crates/nikaia-std/src/tools/check_numbers2.nika crates/nikaia-std/src/tools/check_overflow.nika crates/nikaia-std/src/tools/lowered/check_numbers2.rs crates/nikaia-std/src/tools/lowered/check_overflow.rs crates/nikaia-std/src/tools/lowered.rs docs/specification/adr/adr-293.md docs/specification/30-nikaia-tooling.md tests/compiler/tests/collect_into_optional_list.nika tests/compiler/tests/collect_into_optional_list.stdout
Commit: check: collect of T? items into a declared C[T]? is null at the first null item (#1161)
Body: ADR-293 D25 extended: where a let, field or result declares C[T]? and the items are T?, collect builds the C[T] and is null at the first null item. collect_by_use (check_numbers2.nika) looks through the option on both sides, accepts a Vec[T] (and the map, set and String it already took) and records collected_into, so the emitter writes no ::<Vec<_>> and Rust's collect into Option infers from the place. a_collect_into_an_option keeps check_widened_slot (check_overflow.nika) from wrapping such a collect in Some(..). Spec sentence in ADR-293 D25 and Part III's Seq paragraph.
Fixes: #1161
Tests: new collect_into_optional_list (--both-settings) passes, fails on the unfixed compiler with NK1104/NK1103 (`Vec[i64?]` vs `Vec[i64]?`); it collect_into:: nullable:: options_only:: 74 passed; clippy -p nikaia-std --features compiler clean; lower-std fixpoint clean
Deviation: test placed in tests/compiler/tests/ (the brief wrote tests/compiler/), the test directory's convention; brief's probe path no longer existed, probe rewritten from the issue title
Found: tuple match subject with a list element moves out of a reference: `match (xs[0], w) { (K::A(x), K::A(y)) => .. }` with `xs: ref Vec[K]`, K non-copy, lowers to `match (*index::get(&xs, 0), w)` and rustc refuses (cannot move out of a shared reference); per-part lending of tuple subjects needed (check_core_flow_arm by_reference, emit_match_subject). Not blocking (nested match used); please file
Needs: none
Changelog: `collect()` into a declared `Vec[T]?` (or map, set, `String` as `C[T]?`) from items of `T?` is `null` at the first `null` item; it was refused with NK1103/NK1104 (#1161).
Owner: none
Pushed: local ff731aa4414ea9394446b81c3f050aec63918e83
Disk: target/nikaia-project-tests 2.4G, / 3.5G free (91%); new cache entries over 100 MB: rlib/df8bcfe6 281M, run-bundle/c1ccf85a 288M, run-bundle/3f8b93b8 288M, build-time-parsers/768d2772 902M, build-time-bundle/4e06b518 488M (some may be other agents)
