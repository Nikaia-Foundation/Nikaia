Result: done
Files: crates/nikaia-std/src/tools/check_narrow.nika crates/nikaia-std/src/tools/lowered/check_narrow.rs crates/nikaia-std/src/lib.rs tests/compiler/tests/a_field_read_through_an_index_or_a_call_does_not_narrow.{nika,stderr} tests/compiler/tests/an_if_let_binding_in_a_condition_is_refused.{nika,stderr} tests/compiler/tests/an_is_binding_in_a_condition_is_refused.{nika,stderr} crates/rs2nika/convert/src/main.nika crates/rs2nika/convert/tests/{if_let_narrow.in/f.rs,if_let_narrow.stdout,host_b.stdout} docs/selfhosting-gaps.md docs/specification/adr/README.md
Commit: 441382d (step 3, D4), a9978f9 (step 5, D6 tests only), 65e011e (step 6, converter)
Fixes: #869
Pushed: 65e011ee
Tests: nikaia test --both-settings tests/compiler 149 ok; it nullable narrow views_kept match_arms parse 158 ok; rs2nika convert 39 ok; guard set 22 ok; clippy clean; pre-push-check ok; the D4 test fails on the unfixed compiler (no note)
Gate: green
Deviation: step 5 needed no compiler change: the parser already refuses `if let`, `if x is Some(v)` and `if e is Leaf(v)`, but its message does not name `match`; clippy::unnecessary_unwrap allowed on the tools module (lowered narrowing)
Found: none
Needs: none
Changelog: A `null` test of `v[i].f` or `f().x` proves nothing, and NK1102 at a use says the read is not a path; `if let` and `is` bindings in a condition stay refused (ADR-382 D4, D6).
Owner: none
Disk freed: 88 KB scratch; target/nikaia-project-tests 4.5G (new entry over 100 MB: run-bundle/65d964130cc06fb2 293 MB); / 17G free of 252G
