Result: partial
Pushed:
Tests: a_mut_argument_read_in_another_argument (new, #899 probe 1; failed unfixed per the issue, immutable borrow while mutably borrowed); tests/compiler --both-settings 348 passed; it guard set 23 passed
Deviation: #899 step 2 (mismatched_lifetime_syntaxes) does not reproduce on develop with three probes; left open for the supervisor to close or re-probe
Found: none
Needs: none
Changelog: An argument whose `f"…"` reads a `mut` argument of the same call is made before the `&mut` is taken (#899, step 1). Decided: the hoist's mention scan reads the holes of an interpolated text.
Gate: green
Files: crates/nikaia-std/src/tools/emit_core_below.nika crates/nikaia-std/src/tools/lowered/emit_core_below.rs tests/compiler/tests/a_mut_argument_read_in_another_argument.nika tests/compiler/tests/a_mut_argument_read_in_another_argument.stdout docs/briefs/lane-e.md.report.md
Commit: Hoist an argument whose f-string reads a mut argument of the same call
Part: #899
