Result: done
Files: scripts/drive.nika docs/briefs/architect-task.md tests/compiler/tests/drive_actions.nika tests/compiler/tests/drive_actions.stdout docs/architect.md
Commit: scripts: drive.nika runs builders and the architect until an escalation
Fixes: none
Tests: drive_actions (nikaia test --project tests/compiler, 103 ok; --dry-run probes of NEXT, BATCH DONE, CUT, RED, BUMP, OWNER, unknown in its .stdout); guard set 22 ok
Gate: green
Deviation: added `--lines FILE` (cycle output from a file, implies --dry-run) so the test is deterministic; a RED sha already handed over runs the NEXT builder if there is one (else escalates) so a fix-forward builder can run; loop not run live (needs agents)
Found: `return s.bytes[0..n].text()` in a `-> String` fn lowers to Rust "Cannot return value referencing local variable" (replace used instead); not reported to the compiler
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Pushed: 1bba03363692626b8083026c15ee2cca7db41319
