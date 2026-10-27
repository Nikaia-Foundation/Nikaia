Result: done
Files: crates/nikaia-std/src/tools/check_core_expr.nika crates/nikaia-std/src/tools/check_core_flow.nika crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/emit_core_flow.nika crates/nikaia-std/src/tools/emit_state.nika crates/nikaia-std/src/tools/emit_stmt.nika crates/nikaia-std/src/tools/lowered/check_core_expr.rs crates/nikaia-std/src/tools/lowered/check_core_flow.rs crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/emit_core_flow.rs crates/nikaia-std/src/tools/lowered/emit_state.rs crates/nikaia-std/src/tools/lowered/emit_stmt.rs tests/compiler/tests/a_catch_on_a_call_statement_drops_its_value.nika tests/compiler/tests/a_catch_on_a_call_statement_drops_its_value.stdout
Commit: a `catch` on a call used as a statement drops the call's value (Fixes #885)
Fixes: #885
Tests: committed by hand with git, not scripts/commit.nika (it has no body field for the position note the brief asks for, and misses the brief's Attribution under the .md.report.md name); new a_catch_on_a_call_statement_drops_its_value failed on the unfixed compiler (NK1103 on the `"text"` handler; rustc "match arms have incompatible types" without it); nikaia test --both-settings --project tests/compiler 170 passed; it filter catch + guard set green; clippy clean; pre-push-check green
Pushed: rejected
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: a `catch` on a call used as a statement no longer needs its block to hand back the call's type
Owner: none
Disk freed: 0 GB (no own clone; target/nikaia-project-tests 2.3G, / 19G free, no new entry over 100 MB)
