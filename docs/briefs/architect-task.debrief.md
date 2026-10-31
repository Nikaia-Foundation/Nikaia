1. **Most turns:** finding why CI was red. I could not download the job logs (the proxy blocked them), so I worked through check-run annotations. Then I installed clippy 1.99.1 and reproduced the failure in a scratch worktree. After that, choosing the third brief took a while: rows C7-a and C7-b of the plan turned out to be built already, and I only learned that by reading the code.
2. **What the brief could have handed me:**
   - The text of the failing clippy error, and that CI uses the newest stable while this machine has 1.97.
   - Which plan rows are really still open.
   - That `nikaia` is not on PATH (it is `target/fast/nikaia`) and that `nikaia test` has no name filter.
   - After the interruption: that the driver had already committed my files (6cbf22d9) and run the first brief. I found that out from `git log`.
3. **Checks:**
   - Useful: clippy 1.99.1 on the scratch worktree, the reads of `known_default` and its caller, the issue text of #920, and `git log`/`git status` on resuming.
   - Wasted: a log download that could never work, a spec grep that returned 52 KB, the C7 lookups, and one `CLAUDE.md` edit attempt with the wrong anchor text.
4. **The one change that would have saved the most:** `drive.nika` should write the failing job's annotations, or the clippy output, into `architect-input.txt`. On a resumed run it should also list which briefs in the batch have already run.
