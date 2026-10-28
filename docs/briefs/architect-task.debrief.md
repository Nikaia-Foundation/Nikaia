1. **Most cost:** reading all of `docs/architect.md`. About 100 of its 374 lines are the *State* history, and only the newest entry mattered. After that, two lookups: the stop line and the builder's report.
2. **What the brief could have handed me:**
   - The newest *State* entry.
   - The commit each pending brief's line ranges were taken at (a4b33a0). I first diffed from fb686ff, which was the wrong commit, and needed a second call.
   - The report path of the brief that cut the batch. The input line was truncated in the middle of the deviation text.
3. **Checks:**
   - Useful: `git log a4b33a0..HEAD` on `project.rs`/`modules.rs` showed both briefs' ranges still hold. Reading the report and `git show --stat 1f3adf2` confirmed the fix-forward landed.
   - Wasted: the first `git log` from fb686ff, and the CI query. All six runs were still running, so it gave no verdict.
   - Not done: I didn't confirm the push to `origin` (the push was silent), and I didn't re-paste the new rule into the `common.part` copies already inside `p5a`/`m3a`. Those two builders won't see it.
4. **Best change:** have `drive.nika` write into `architect-input.txt`:
   - the full deviation text and the report path;
   - the current CI line (from `status.nika`);
   - for each brief left in `batch.txt`, whether its `Cut:` files changed since the commit the brief names.

   That turns my 7 calls into about 2. Second best: briefs should name `common.part` by reference, not paste it, so a rule change reaches briefs that are already written.
