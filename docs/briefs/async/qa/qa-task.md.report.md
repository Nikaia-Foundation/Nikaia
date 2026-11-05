Result: done
Cause: 15e380dd (a11, the fallback `catch` after `let x = async e`): CI bootstrap_compiler arm64, specification::the_specifications_programs_are_the_ones_in_expected_txt; block #107 of Part I 8.1.2 now parses and is refused like its sibling #105 instead of being a fragment.
Fixed: tests/specification/EXPECTED.txt regenerated; one line, #107 "fragment (as written)" to "refused (a body) [NK1117,NK1181,NK2110]", the change a11 meant.
Tests: `cargo test -p nikaia --test it specification` 6 passed; pre-push run below.
Found: none
Files: tests/specification/EXPECTED.txt
Commit: specification: EXPECTED.txt takes block #107, the async fallback now parses (the fix for the drift after ADR-async-5 D22)
Changelog: none
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 3fd4cc70c1fc632c2239362dfe323c91725edc01
