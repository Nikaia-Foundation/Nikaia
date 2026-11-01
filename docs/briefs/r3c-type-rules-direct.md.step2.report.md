Result: done
Pushed: local 5a18a962e4363d9a9911c3ddf6dd8a93e228a398
Tests: tests/compiler --both-settings 539 passed 0 failed; the new test failed on the unfixed compiler (NK1131 You can't give away `s.node`)
Found: kind 3, fixed: a part of a `for mut` element handed to a `mut` parameter was refused as given away (NK1131) when its type is declared in another file or has no stand-in
Changelog: Fixed: a part of a `for mut` element handed to a `mut` parameter is lent to it, not given away, wherever its type is declared.
Files: crates/nikaia-std/src/tools/check_core_args.nika crates/nikaia-std/src/tools/lowered/check_core_args.rs tests/compiler/tests/a_part_handed_to_a_mut_parameter_is_lent.nika tests/compiler/tests/a_part_handed_to_a_mut_parameter_is_lent.stdout
Commit: A part handed to a mut parameter is lent, not given away
Part: #125
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
