Result: done
Files: see b896a-slice-in-tuple-to-string.md.step1.report.md and .step2.report.md
Commit: 3419c1dd emit: a list pattern at a tuple position matches that position as a slice; d724ac06 to_string() of a String parameter is handed on as the parameter is
Fixes: none
Part: #896
Pushed: d724ac06f4e4617e85ea5cbd1983e9f09148f9c1
Tests: a_list_pattern_in_a_tuple_over_a_lent_field and to_string_of_a_lent_parameter_stored (new, each failed on the unfixed compiler per the interrupted run's step reports); nikaia test --both-settings --project tests/compiler 191 passed; it handed_over text_tiers typecheck keeps + guard set 344 passed; lower-std fixpoint clean; clippy clean; pre-push-check clean
Gate: green
Deviation: none
Found: (1) `keep(ns, f"k{ns.len()}")` with `mut names` lowers to `keep(&mut ns, format!(.., ns.len()))`, which rustc refuses (borrow of `ns`); (2) `fn make(name: String) -> B { return B { name } }` called only with literals while B.name is mixed lowers to `&'static str` -> `B<'_>`, rustc warns mismatched_lifetime_syntaxes
Needs: none
Changelog: A tuple matched with a list pattern at one of its positions now lowers that position as a slice, and `.to_string()` of a `String` parameter stored in a struct or a list hands the parameter on as the bare name does; both failed in rustc before.
Owner: none
Disk freed: 0 (target/nikaia-project-tests 2.5G, / 19G free; new entry over 100 MB: run-bundle/65d964130cc06fb2 294 MB)
Rs2nika: none
