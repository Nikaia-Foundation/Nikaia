Result: done
Files: crates/nikaia-std/src/tools/check_core_call_on.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/check_expect.nika crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/emit_expr.nika crates/nikaia-std/src/tools/lowered/check_core_call_on.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/check_expect.rs crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/emit_expr.rs tests/compiler/tests/a_copy_view_is_copied_where_the_value_is_wanted.nika tests/compiler/tests/a_copy_view_is_copied_where_the_value_is_wanted.stdout tests/compiler/tests/a_view_that_does_not_copy_is_not_copied.nika tests/compiler/tests/a_view_that_does_not_copy_is_not_copied.stderr docs/briefs/bugs/b33-1107-copy-view-copied.md.report.md
Commit: check: a view of a type that copies is copied where the value is wanted (ADR-387)
Body: A new fact `copied_views` (check_facts) records a name that is a `ref C` of a copying type where `C` or `C?` is wanted; `CheckState::a_view_copied_out` (check_expect) sets it from `expect_kept` (so `let`, `return`, field, map key) and from a described method's argument (`check_core_call_on_given`, `kept.push(e)`). The emitter writes `*name` in `emit_expr_variable`'s last branch, after the derefs already there (`mut` lambda param, `let mut` view, copied-in-place binding) and not where `copied_args` (ADR-233 D4, calls) writes it. `check_core_stmt_a_lent_struct` drops `does_not_copy` (b26's note): a copy struct unwrapped past a jump is a view too, and NK1183 skips a view copied out. Bootstrap: the first build doubled the `*` at 12 tools sites (pattern bindings already derefed), fixed by moving the `*` into `emit_expr_variable`; the lowered files are at the fixpoint. Slip: after pre-push I ran `git reset -q` to unstage my files; the index held only them (empty before), working tree untouched.
Fixes: #1107
Pushed: local d64b3978c0744cb1c376d2e9c0a28262302a98bb
Tests: new a_copy_view_is_copied_where_the_value_is_wanted (prints `1 2 2 1 3 5` / `5 5 5`) and a_view_that_does_not_copy_is_not_copied (NK1103) pass --both-settings in a scratch project, both failed on the unfixed binary (the first refused with NK1104); an_unwrapped_map_view_stays_a_view ok; it `coalesce view map_read element_read copy lending` 255 passed; lower-std fixpoint clean; cargo clippy (1.97 and 1.99.1) clean.
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A view of a type that copies is copied where the value is wanted: `let g: C = e`, `return e`, `Holder { c: e }`, `kept.push(e)` and `let i: i64 = f` with `e: ref C` (C copies) now compile, as a call argument already did (ADR-387 D1); a view of a type that does not copy stays refused with the same codes (D2).
Owner: none
Disk freed: none (target/nikaia-project-tests 2.0G, / 6.0G free; new cache entries over 100 MB since the brief: run-bundle/3f8b93b88f7f213d 342, build-time-parsers/768d2772d43bb4cc 949, build-time-bundle/efd5abe9209adbd7 486, from the it run or other lanes)
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
