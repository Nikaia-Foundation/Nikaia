Result: partial
Cause: none of the unpushed commits (they are briefs, a std Path conversion and a check change); `tests/compiler: agent_effort` failed once: at user-parallelism=no the first dry run (`low`) of `nikaia run scripts/agent.nika` exited 1 with empty stdout, at `yes` it passed. A flake of a cold-cache first build, not a wrong expectation.
Fixed: nothing; no commit (a half-made fix is not committed).
Tests: agent_effort alone, 2 settings: ok; with agent_facts, agent_script, focus_rotation at --both-settings, 3 runs on the warm cache: ok. 6 more runs with stderr captured: no failure. 15 parallel cold-cache dry runs: no failure.
Ruled out: the expected .stdout (matches), the brief's effort logic in agent.nika (dry run and --info right), a build race on `~/.cache/nikaia/run/agent-*` with 3 to 6 parallel cold runs.
Found: with `--no-cache`, 4 programs that all `nikaia run scripts/agent.nika` at once (agent_effort, agent_script, agent_facts, focus_rotation) failed 1 time in 3 (agent_effort once, agent_script and focus_rotation once): "ld terminated with signal 7 [Bus error]", "no such file ... tokens.txt". Probe: copy those four tests into a scratch project, `nikaia test --both-settings --no-cache`, repeat. Suspects: concurrent writes to the shared rlib cache and to `run/<name>-<key>` (`write_if_changed`, project.rs ~l.694-715), or a full disk (it filled to 0 MB free while I probed; I deleted my own scratch caches in /tmp, now 7 GB free). Cause of the suite's red is not shown; check the disk and rerun before looking further.
Files: none
Commit: none
