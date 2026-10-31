1. **Time and turns.** Two things cost the most. The first was working out which lane the red belonged to. `docs/briefs/qa-input.txt` didn't exist, and the one I needed was under `bugs/`. The second was the test run: the debug build failed, the fast build took 3m44s, and the three tests took 227s. All the investigation before that came to only about 15 turns.

2. **What I had to look up and the brief could have given me.**
   - Which lane's `qa-input.txt` applied. `qa-task.md` doesn't say which lane started it.
   - Which commit and worktree the suite had actually tested. `qa-input.txt` lists unpushed commits, but local `develop` had diverged from `origin/develop`, so that list didn't match the tested tree.
   - That the gate uses `--profile fast`, so debug builds are not representative.
   - Where the driver's logs and locks live.

3. **Checks.**
   - Useful: `badges.py --check`, `pre-push-check.sh`, and the three tests rerun under the fast profile. These showed the red didn't reproduce.
   - Wasted: the first `cargo test` in the debug profile. It spent a build on a recursion-limit failure that has nothing to do with the gate.
   - Largely wasted: running `badges.py` in the old `nikaia-fix` and `nikaia-lane2` worktrees. They are stale and told me nothing.

4. **One change.** Have the driver write the tested commit sha, the lane, and the suite worktree path into `qa-input.txt`. Also have it write the exact failing command, with its profile, and the failing test output to a file. Add a sentence to `qa.md`: "if the failure does not reproduce on the tested commit, report `done` with no commit and stop". A "known transient: concurrent gates" note would also help. With any of these I would have gone straight to the rerun and the report.
