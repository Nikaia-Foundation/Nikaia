Result: done
Files: crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/check_reads.nika crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/check_reads.rs tests/compiler/tests/a_map_or_list_element_past_a_jump_compares_with_a_value.nika tests/compiler/tests/a_map_or_list_element_past_a_jump_compares_with_a_value.stdout docs/briefs/bugs/b27-1112-element-compared-with-a-value.md.report.md
Commit: check: a list or map element past a jump compared with a value is a view
Body: New `check_core_stmt_got_compared`, asked by `a_side_is_a_view`: a name whose binding has `view_of` (#1044: a `std` list/map read past a jump, element not copying) or a `??` of a `std` get (`flow.std_gets`) whose fallback jumps, element neither copying nor text, is a compared view; the emitter writes `*` from `compared_views` as before. Not `lent_lets` (made `let m = found.field` NK2106 in emit_sums.nika and leaked across sibling blocks in check_expect.nika), not `got_past_a_jump` (any `get`, so a program's own `get` handing back an owned E got `*f`: rustc "cannot be dereferenced").
Fixes: #1112
Tests: before (HEAD 9a2da839 binary): rows a-d rustc refused (a,c,d "Can't compare `&E` with `E`", b "Mismatched types"); e,f true. After: all six true; a user `get` returning an owned E compared with E::B still runs. tests/compiler --both-settings 522 passed 0 failed (incl. the new test and every compare/coalesce/view name); lower-std fixpoint clean (only my two twins); cargo fmt, clippy -D warnings, pre-push-check clean.
Gate: not run (cheap checks only)
Deviation: none
Found: `Vec[String].get(0) ?? return false` compared with an owned String still reaches rustc (`&str` vs `str`): #947's, untouched. `lent_lets` is keyed by (name, depth) and never cleared at a block's end: a lent `let` in one block leaks into a sibling block of the same depth (seen only with the discarded lent_lets variant; latent).
Needs: none
Changelog: A list or map element read past a jump (`let x = m.get(k) ?? return`, or that `??` inline) compared with a value of a type that does not copy is read through the view, where rustc refused `&E == E` (#1112).
Owner: none
Disk freed: none (no clone)
Rs2nika: none
Model: Opus 5.5
Pushed: local a0a48d32f8c3f0d87223abc1aae767ad2c28addc
