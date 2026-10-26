Result: done
Files: crates/nikaia-std/src/tools/check_arith.nika crates/nikaia-std/src/tools/check_core_expr.nika crates/nikaia-std/src/tools/check_core_ops.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/check_arith.rs crates/nikaia-std/src/tools/lowered/check_core_expr.rs crates/nikaia-std/src/tools/lowered/check_core_ops.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs docs/specification/30-nikaia-tooling.md assets/badges/self-hosted.svg tests/compiler/tests/an_operator_on_text_is_refused.nika tests/compiler/tests/an_operator_on_text_is_refused.stderr tests/compiler/tests/a_minus_on_a_bool_is_refused.nika tests/compiler/tests/a_minus_on_a_bool_is_refused.stderr tests/compiler/tests/a_bang_on_text_is_refused.nika tests/compiler/tests/a_bang_on_text_is_refused.stderr tests/compiler/tests/a_bang_on_an_integer_flips_its_bits.nika tests/compiler/tests/a_bang_on_an_integer_flips_its_bits.stdout
Commit: NK1191 also refuses arithmetic on text and bool, - on a non-number and ! on a non-bool non-integer
Part: #655
Pushed: deed9ad912a8a82492e423bc723c9f357f9aec9a
Tests: project tests/compiler both settings 126 ok; it operator arith + guard set 42 ok; probes of the brief all NK1191, "a"+"b", !5, -5 pass; not run against unfixed compiler (architect's probes show those passed before)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: NK1191 also refuses arithmetic on text and `bool`, `-` on a non-number and `!` on a non-bool non-integer.
Owner: none
Disk freed: 0
