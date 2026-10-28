Result: partial
Files: .github/workflows/nikaia.yml
Commit: ci: the accepted sweep names its failures as annotations
Body: Both accepted-sweep steps tee their output to $RUNNER_TEMP/sweep.log under set -o pipefail; a following if: failure() step turns each failing program's hash line, rustc's first error line, the summary line, "holds no program" and the panic line into ::error:: annotations, in log order, at most 60. The red since c7e5b66 itself is not reproduced: c7e5b66 changed only crates/rs2nika/convert/src, and no workspace test reads those files (rs2nika is not a workspace member; no it test, tests/compiler program or script names convert/src; the repo-wide walks in fmt.rs and sequences.rs cover examples, tests, benches, packages and nikaia-std/src only, and a converter file is a project file, which dump_accepted skips). So the cause is probably not in the dump that c7e5b66 changed. The next CI run on develop will show the failing hash or the stale KNOWN entry as an annotation.
Fixes: none
Pushed: 60b1154f0a37ae644c1150c052ba41a0c0b6c6d3
Tests: grep -rn of crates/nikaia/tests, tests/compiler, scripts and tools/*.nika for rs2nika and convert/src: no test checks converter sources; the annotation grep was tried on a sample sweep log (program line, error[E0282], panic, summary all annotated); yaml parses; scripts/pre-push-check.sh passed; the suite was not run with the dump (needs the full suite)
Gate: green
Deviation: steps 1-2 not built: not reproducible from the tests named, so step 3 was built alone, as the brief's stop rule says
Found: none
Needs: the annotations of the next develop run's arm64 sweep step, then a new fix-forward brief for the program they name
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: none
