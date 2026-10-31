Result: done
Files: crates/nikaia-std/src/tools/check_takes.nika crates/nikaia-std/src/tools/lowered/check_takes.rs tests/compiler/tests/takes_an_optional_handed_over_twice_says_its_copy.nika tests/compiler/tests/takes_an_optional_handed_over_twice_says_its_copy.stderr
Commit: check_takes: NK2105's help writes `t?.clone()` for an optional handed over twice
Body: The plain-call NK2105 (`data_handed_over_is_used_again`) wrote `{name}.clone()` whatever the type, which the compiler refuses on a `T?`. A shared helper `a_copy_said(path, ty)` now gives `?.clone()` when the type's text ends with `?`, used by this site and the repeat-frame refusal that already did so inline. (`a_copy_of` was taken by threads.nika.) No existing `.stderr` changes: the three with this help line hand over non-optionals.
Fixes: #1056
Tests: new takes_an_optional_handed_over_twice_says_its_copy passes; its output on the unfixed binary differs on the help line (`t.clone()`), so it fails there; `t?.clone()` in the probe compiles and runs (prints 1, 1); tests/compiler --both-settings 456 passed, 0 failed; packages/yaml 14 passed; lower-std fixpoint reached; clippy and pre-push-check clean; fmt::every_nika_file not run (no file under packages/ changed)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: NK2105's help for an optional handed over and read again suggests `t?.clone()`, the copy that compiles, instead of `t.clone()` (#1056).
Owner: none
Disk freed: 0 GB (cache 6.5G kept, 3.9G free on /; scratch binary copy removed)
Rs2nika: none
Pushed: local 9c7f0298e6024c2c10b83c614d7caa9a80b2e4f9
