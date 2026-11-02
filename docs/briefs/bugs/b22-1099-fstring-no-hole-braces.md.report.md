Result: done
Files: crates/nikaia-std/src/tools/emit_core_leaves.nika crates/nikaia-std/src/tools/lowered/emit_core_leaves.rs tests/compiler/tests/fstring_no_hole_braces.nika tests/compiler/tests/fstring_no_hole_braces.stdout
Commit: emit: an f-string with no hole writes its braces once
Body: the hole-less branch of emit_core_string wrote format_of(parts) (doubled braces, for format!) into a plain string literal; it now undoes the doubling. Text parts are source text, so quotes and backslashes are already escaped as in the LitString arm; no second escaper.
Fixes: #1099
Tests: new fstring_no_hole_braces passes (--both-settings, one-program project); on the unfixed binary (/home/user/nikaia-suite, probe) it prints 9, a {{ b }}, 4: fails there. lower-std fixpoint clean; pre-push green.
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: An f-string with no hole used as a receiver (`f"{{}}".len()`) now writes its braces once.
Owner: none
Disk freed: 0
Rs2nika: none
Attribution:
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local eb8f97fe6e507b0e637d104a4db3a58e7098c0ff
