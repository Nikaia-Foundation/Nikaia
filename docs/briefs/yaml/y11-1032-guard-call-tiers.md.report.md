Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs tests/compiler/tests/grammar_a_call_in_a_guard_is_a_call_site.nika tests/compiler/tests/grammar_a_call_in_a_guard_is_a_call_site.stdout
Commit: text tiers: a call inside a grammar guard is a call site (#1032)
Body: bind_grammar_pattern dropped a Pattern::Guard in its else arm, so a function called only from a guard had its text parameters tiered from its other callers alone (NK1185 at a `??` in a callee). The guard's condition is now walked with tier_expr in pattern order, after the labels before it are bound. A rule call's value arguments and a counted repeat's bounds are still dropped by the same else; no probe showed them mattering.
Fixes: #1032
Pushed: 297eefc8 (pushed by drive.nika)
Tests: new grammar_a_call_in_a_guard_is_a_call_site failed on the unfixed binary (NK1185) and passes; nikaia test --both-settings --project tests/compiler 411 passed 0 failed
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A function called from a grammar guard is now a call site for the text-tier pass, so its text parameters are no longer tiered from its other callers alone (#1032).
Owner: none
Disk freed: 0
Rs2nika: none
