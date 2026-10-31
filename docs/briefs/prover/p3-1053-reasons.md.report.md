Result: done
Files: crates/nikaia-std/src/tools/bounds_reasons.nika crates/nikaia-std/src/tools/lowered/bounds_reasons.rs crates/nikaia/src/bounds_report.rs assets/badges/self-hosted.svg
Commit: bounds: the catch-all reason of --bounds split by how the unbounded name was bound
Body: Where the site's syntax names no reason, bounds_reasons.nika now looks up the first name the site reads in the enclosing function (the Walk passes its args and body, plus the interner, through the two adapters; the classification is all Nikaia). It checks: an index of one list in a loop whose while condition or range is bounded by `other.len()` (#388); a `+=`/`-=`/`= n + k` on that name inside a loop (counter, #1053 step 4, #432); a `let` from `if`/`match` (#1053); a `let` from `xs[k]`/`get`/`first`/`last`/`pop` (with or without `??`), or a `for` over a collection (#797, #798); a parameter that no `let`/`for` rebinds (ADR-269 D5). Anything else keeps the old text. The reasons carry no names, so proved.nika groups them unchanged. lower-std --bounds over tools, unproved by reason: 692 old catch-all, 582 counter, 379 parameter, 127 list element, 96 another list's length, 60 call result, 37 list of lists, 7 if/match value, 6 product, 4 negation, 3 field. Proved 1514/2620 indexes, 718/1605 operations.
Part: #1053
Tests: probe of the brief with check --bounds gives the five: "a counter a loop changes, with no bound tied to the loop's turns (#1053 step 4, #432)" / "an `if` or `match` value, which the walk does not read (#1053)" / "the length of another list, which bounds the loop and not this index (#388)" / "a parameter no precondition bounds: an `assert` at the top of the function makes it one (ADR-269 D5)" / "a value read out of a list, of which the walk knows no fact (#797, #798)"; table: 692 catch-all, 582 counter, 379 parameter, 127 element, 96 other length, 60 call, 37 list of lists, 7 if/match, 6 product, 4 negation, 3 field; it bounds_report:: field_paths:: facts_of_the_walk:: prove_through_loops:: expression_functions:: fold_counters:: value_bounds:: 65 passed; clippy 1.97 and 1.99.1 clean; pre-push-check ok
Gate: green
Deviation: none
Found: none
Needs: lower-std on the current tree rewrites lowered/project_members.rs (TrustedMap to HashMap, 14 lines) without any change of mine; left uncommitted in the working tree, so the driver or supervisor should check develop's lower-std fixpoint
Changelog: `--bounds` names why a check stayed from how the name was bound: a counter a loop changes, an `if`/`match` value, a parameter, a value read out of a list, or another list's length (#1053)
Owner: none
Disk freed: 0 GB (cache 3.6G, disk 17G free; new cache entries over 100 MB: rlib/df8bcfe6 987M, build-time-parsers/768d2772 869M, tools/target 402M, run-bundle/3f8b93b8 341M, rlib/af187e66 265M)
Rs2nika: none
Pushed: local 41ea2c357ec0f99d81a0a4f5a973c499460be559
