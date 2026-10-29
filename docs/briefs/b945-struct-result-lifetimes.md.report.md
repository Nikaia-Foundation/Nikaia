Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs tests/compiler/tests/a_struct_result_viewing_a_local_lives_in_the_callers_keep.nika tests/compiler/tests/a_struct_result_viewing_a_local_lives_in_the_callers_keep.stdout tests/compiler/tests/a_text_parameter_moved_into_a_local_struct_is_lent.nika tests/compiler/tests/a_text_parameter_moved_into_a_local_struct_is_lent.stdout docs/briefs/b945-struct-result-lifetimes.md.report.md
Commit: text tiers: a struct result viewing a local takes the caller's keep
Body: ADR-353 D10: tie_leaves no longer unlends a callee's text parameter when the callee hands back a declared struct or enum (TextTie.through_value, TextDeclared.types) that views a local; the value goes into the caller's keep (__keep.put_viewed) instead of owning the field. Restricted to struct and enum results: unrestricted, keeps.nika itself was refused (NK2305 through String? helpers). #942 step 2 (a text parameter moved only into a local struct is lent) was already built on develop before this run: its test passes on the unfixed compiler; this commit adds the test. Earlier commits of this run: 7d9efd7 (a jumping ?? left of a comparison keeps its parentheses, found on the way), ed4c3ec (#945 step 1).
Part: #942
Pushed: none
Tests: 4 new tests/compiler programs; jumping-coalesce, constructor and caller's-keep tests fail on the unfixed compiler (Expected expression found ==; explicit lifetime required; NK2105), the step-2 test already passed there; nikaia test --both-settings tests/compiler 255 of 256 ok, commit_dry_run failed only because the binary under test was named nikaia-new (it falls back to target/fast/nikaia); it tether_keep result_source plus guard set 119 ok; clippy clean; pre-push-check ok
Gate: green
Deviation: brief names Parser::new(text.trim()), which is NK1149; the test calls Parser(...); D8 applied to struct literals only, not tuple, list or variant literals
Found: return (a ?? return false) == "x" lowered unparenthesised (fixed in 7d9efd7); a named method moving a text parameter into a struct, called with literal.trim(), is still NK1102 (text tiers treat the parameter as Lasting, so keeps keeps it); xs.get(k) ?? return false == "a" over a list of literals gives a &&str vs &str rustc error
Needs: none
Changelog: A struct or enum result viewing text the callee made takes the caller's keep instead of owning the field (ADR-353 D10).
Owner: none
Disk freed: none
Rs2nika: none
