Result: done
Files: crates/nikaia-std/src/tools/text_tiers.nika crates/nikaia-std/src/tools/lowered/text_tiers.rs tests/compiler/tests/a_returned_local_views_the_parameter_it_was_assigned_from.nika tests/compiler/tests/a_returned_local_views_the_parameter_it_was_assigned_from.stdout
Commit: text tiers: a loop body is walked until its next turn reads nothing new, so a local read above its assignment holds the assigned view
Body: The tier walk visited a loop body once, in order: a `let mut previous = ""` read by `return previous` above `previous = line.trim()` was still only the literal there, so the result became `Lasting` and lowered as `&'static str` ("Lifetime may not live long enough"). The value reaching that read comes round the loop's back edge, not from the first initializer. `tier_loop_turns` walks a `for` or `while` body (and the `while` condition) again until the kinds and the literal binding of every name bound around it stop changing; each walk only joins kinds and adds to sets (ties are held once, `tie_add`), so it ends. The probe now lowers as `fn before<'r>(text: &'r str, name: &'r str) -> &'r str`, as the same function returning `previous` after the loop already did. The first commit of this brief (bdf3e2ff) is #1202.
Fixes: #1203
Pushed: local fed74cdc482142ab796cfed16d9130d9561f10c5
Tests: new tests/compiler a_returned_local_views_the_parameter_it_was_assigned_from fails on the unfixed binary (rustc lifetime error), passes after; tests/compiler 600 of 600 (both settings; 6 first failed only because target/fast/nikaia vanished mid-run, all 7 then pass singly); it grammar/tier/view modules 203 passed; clippy and lower-std fixpoint clean; corpus guard (red on develop, 18) not run
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A function handing back a local that a loop assigns a view of a parameter, read above that assignment, returns a view of the parameter instead of `&'static str` (#1203).
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
