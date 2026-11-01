Result: partial
Files: none
Commit: none
Fixes: none
Pushed: none
Tests: probe reproduced on the unfixed binary (9, `a {{ b }}`); fix not built or tested
Gate: red build (another agent's untracked comptime_bundle.nika)
Deviation: none
Found: none
Needs: the owner of the untracked tools/comptime_bundle.nika (and lowered/comptime_bundle.rs) must fix or remove it; it breaks `cargo build -p nikaia-std` (E0119 CheckWall Display in grammar_run.rs, E0592 duplicate `message`, E0277 PathBuf From EitherText)
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: none
Body: the edit is in the working tree, uncommitted, in emit_core_leaves.nika (and its lowered twin): the hole-less branch replaces "{{" with "{" and "}}" with "}" in format_of(parts) before writing the literal. The test fstring_no_hole_braces.nika is not kept; its content was the probe, f"{{}}".len(), and a hole-less f-string with \" and \\ as receiver.
