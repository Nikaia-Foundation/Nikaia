Result: done
Cause: d0855a7a (the commit CI ran on ci/views) failed tests/agent_script.nika; its dry-run prompt gained the Core facts (23febfe0) and the test read words[7..] of the old layout. Already fixed by e95ea023, which is after the CI head and is not in that run.
Fixed: nothing new; no commit needed (e95ea023 reads the flags after ` --agent `).
Tests: `nikaia test --both-settings --project tests/compiler` on HEAD (target/fast/nikaia, build up to date): 729 passed, 0 failed, agent_script and agent_facts ok. arm64 and bootstrap_compiler failed only on this one program (727 passed; 1 failed).
Found: none
Files: none
Commit: none
