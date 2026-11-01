1. **Most turns and time:** I tried to reproduce one flaky failure. I built scratch projects and ran cold-cache loops of 3 to 6 parallel runs, and most of them showed nothing. The last probe filled the disk. The suite log kept only stdout from the failing test, so I never saw why the first dry run exited 1.

2. **What I had to look up that the brief could have handed me:**
   - The test files live in `tests/compiler/tests/`, not `tests/compiler/`. I first guessed the wrong path.
   - `nikaia test` has no name filter, so running one test means building a scratch project.
   - That scratch project has to sit under the repo, because the test's root detection falls back to `../..`.
   - The cache is `~/.cache/nikaia`, and `XDG_CACHE_HOME` did not move it. `HOME` did.
   - `qa-input.txt` showed the failing output, but the gate log did not keep stderr.

3. **Checks:**
   - **Useful:** reading the per-setting output in `gate-tests-compiler.log`, which showed one setting failing and one passing. The `--no-cache` run also reproduced something once.
   - **Wasted:** the repeated `XDG_CACHE_HOME` runs, which never made a cold cache. The 12-run loop with a fresh `HOME` may have been wasted too: I filtered its output and never confirmed those runs were cold.
   - **Never run:** I did not check whether the suite checkout's disk or cache was full when the failure happened. I only noticed disk pressure at the end, and I caused part of it.

4. **One change:** the gate should keep the stderr of a failing test's process. Better, `qa.md` should tell the agent to look first for a flake: rerun the failing test at both settings and read the disk before building probes. The qa brief should also say that `/tmp` is small and that a probe must clean up after itself. With stderr in the log, I would have seen the real error in one step.
