Result: done
Files: crates/nikaia-std/src/tools/order.nika crates/nikaia-std/src/tools/touch.nika crates/nikaia-std/src/tools/lowered/order.rs tests/compiler/tests/overlap_stdout_stderr_note_names_no_seq.nika tests/compiler/tests/overlap_stdout_stderr_note_names_no_seq.stderr
Commit: order: the NK2104 note for stdout and stderr no longer names seq (#994)
Body: The note ends after "which might be the same place"; the help names the way out. The comment in touch.nika now names ordinary statements (ADR-292 D1). The test's .stderr is the compiler's output as printed by nikaia test.
Fixes: #994
Pushed: 70c1baaf (pushed by drive.nika)
Tests: new test passes via nikaia test --project tests/compiler (379 passed); not run against the unfixed compiler (the old note held the removed sentence, so the .stderr differs by construction)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: The NK2104 note for stdout against stderr no longer sends the reader to the removed `seq`.
Owner: none
Disk freed: 0 GB (no clone)
