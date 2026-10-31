Result: done
Files: crates/nikaia-std/src/num.rs crates/nikaia-std/src/tools/check_core_places.nika crates/nikaia-std/src/tools/check_facts.nika crates/nikaia-std/src/tools/check_reads.nika crates/nikaia-std/src/tools/emit_core_places.nika crates/nikaia-std/src/tools/lowered/check_core_places.rs crates/nikaia-std/src/tools/lowered/check_facts.rs crates/nikaia-std/src/tools/lowered/check_reads.rs crates/nikaia-std/src/tools/lowered/emit_core_places.rs docs/specification/10-nikaia-light.md docs/specification/30-nikaia-tooling.md tests/compiler/tests/an_integer_as_scalar_is_refused.nika tests/compiler/tests/an_integer_as_scalar_is_refused.stderr tests/compiler/tests/an_integer_cast_as_scalar_is_checked.nika tests/compiler/tests/an_integer_cast_as_scalar_is_checked.stdout
Commit: checker: an integer other than a u8 `as scalar` is refused (NK1241), help names scalar::from (#1003)
Body: Takes wip dfe7bac6 over: check_core_cast refuses a variable, an open number or a literal of an integer type other than u8 cast `as scalar`, the help naming `scalar::from(n)`; the run-time ToScalar narrowing (FactNarrowing::ToScalar, emit kind 3, num::to_char) goes. json's hunk was already built by 4e30179a (step 2). The wip's two `Part III 13.5 -> 14.2` edits in Part I were dropped (13.5 is the ledger, which those sentences mean); a misplaced doc comment in check_core_places restored; check_core_places_what_casts and emit_place_narrowing declared sync (#1014). No other integer `as scalar` outside rs2nika (the rest are u8 operands).
Fixes: #1003
Pushed: 28a0c1a2 (step 2: 4e30179a)
Tests: tests/compiler --both-settings 385/385 (an_integer_as_scalar_is_refused ok; the unfixed compiler accepted these casts, as the deleted an_integer_cast_as_scalar_is_checked shows); packages/json 7/7, json check --asserts lists scalar::from as proved and hex4's assert proved through the loop; packages/yaml green; nikaia-std num:: 10/10; clippy clean; pre-push-check ok; sequences guard now ok; scripts/commit.nika refused (git add of the deleted an_integer_cast_as_scalar_is_checked.* fails), so the step-3 commit was made by hand in its message form
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: An integer other than a `u8` cast `as scalar` is refused with `NK1241`, whose help names `scalar::from(n)`, instead of being checked at run time.
Owner: none
Rs2nika: none
Disk freed: 0 GB (scratch binary copy 22 MB); target/nikaia-project-tests 5.8G, / 11G free; new cache entry over 100 MB: run-bundle/b2db8b4e959f1cfc 341 MB
