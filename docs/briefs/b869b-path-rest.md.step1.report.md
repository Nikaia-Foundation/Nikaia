Result: done
Files: crates/nikaia-std/src/lib.rs crates/nikaia-std/src/tools/check_narrow.nika crates/nikaia-std/src/tools/lowered/check_narrow.rs tests/compiler/tests/a_field_read_through_an_index_or_a_call_does_not_narrow.nika tests/compiler/tests/a_field_read_through_an_index_or_a_call_does_not_narrow.stderr docs/briefs/b869b-path-rest.md.step1.report.md
Commit: A field read through an index or a call is not a path: its NK1102 says so (ADR-382 D4)
Part: #869
Tests: nikaia test --both-settings tests/compiler 149 ok; it nullable narrow views_kept match_arms parse + guard set 158 ok; the unfixed compiler gave NK1102 without the D4 note (probe), so the new .stderr fails there
Gate: green
Changelog: A `null` test of `v[i].f` or `f().x` proves nothing, and the NK1102 at a use says the read is not a path (ADR-382 D4).
Model: Opus 5.5
