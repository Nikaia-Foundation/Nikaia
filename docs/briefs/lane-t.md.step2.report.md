Result: done
Files: crates/nikaia-std/src/tools/view_changes.nika crates/nikaia-std/src/tools/lowered/view_changes.rs tests/compiler/tests/for_mut_read_help_names_list.nika tests/compiler/tests/for_mut_read_help_names_list.stderr docs/briefs/lane-t.md.step2.report.md
Commit: view changes: NK2305's for-mut read help names the list's place
Body: The help for a looped view read while the loop changes it suggested for i in 0..<self.len() (the root); it now names the list as written, for i in 0..<self.pending.len().
Fixes: #971
Tests: 1 new tests/compiler program (for_mut_read_help_names_list, .stderr), fails on the unfixed compiler (help names self); tests/compiler --both-settings 348 of 348 ok; clippy clean; pre-push-check ok; lower-std at fixpoint
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: NK2305's help for a for mut loop whose list is read in the body names the list's place (for i in 0..<self.pending.len()), not its root.
Rs2nika: none
