Result: done
Files: tests/compiler/tests/agent_script.nika tests/compiler/tests/batch_frontmatter.nika tests/compiler/tests/batch_lines.nika tests/compiler/tests/drive_actions.nika
Commit: tests: the script tests run the nikaia that tests them, not target/fast (CI red since 6370390)
Fixes: none
Tests: CI situation probed (no target/fast, copy at target/release): before 106 passed 4 failed, after 110 passed; cargo it nikaia_test::the_compiler_test_package ok
Gate: green
Deviation: none
Found: drive.nika/agent/batch/cycle/status look for target/fast/nikaia then target/release/nikaia themselves, so the probe needs a target/release copy as CI has; drive.nika prints that path, so drive_actions now maps target/release/nikaia to target/fast/nikaia in its output (.stdout unchanged)
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Pushed: 1aa6a97f6eb89269d10f114d3097384ede6d7650
