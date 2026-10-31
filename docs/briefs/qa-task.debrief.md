1. **Most time:** `suite.log` named only the red step (`test red tests/compiler`), not the failing test. I had to rerun the whole `tests/compiler` package (about 10 minutes) to find the one failure. I also spent several calls on three wrong leads:
   - `target/it-fail.txt` was stale (Oct 26).
   - The `docs/briefs/` listing was huge.
   - The first `qa-input.txt` lookup was in the wrong directory.

   After that I had to work out that 33ed1119 was already on `origin/develop` and missing from the tested HEAD.

2. **What the brief could have handed me:**
   - Which lane's directory holds `qa-input.txt`. The brief gave two paths and I guessed wrong first.
   - The exact command for the red step (`nikaia test --both-settings --project tests/compiler`). I read it out of `gate.nika`.
   - The fact that HEAD was behind `origin/develop`.
   - The failed-test names. The newest unpushed commit, 0a550483, adds exactly that logging to `gate.nika`, but the suite that failed ran before it.

3. **Checks:**
   - I ran the failing package twice: once to find the failure, once to confirm the fix. Both were needed.
   - The first call, a grep of `suite.log` for failures, found nothing because the log was only 13 lines.
   - I did not run clippy or `pre-push-check.sh`, because the change was one `.nika` test line. I said so in the report.
   - Nothing was wasted beyond the stale-file reads.

4. **Change that would have saved the most:** have the driver write the failing test names, and the first diff lines of each failure, into `qa-input.txt` when a suite step goes red. Add one line to `qa.md`: "if the cause is a test already fixed on `origin/develop`, compare with `git merge-base` and `git log HEAD..origin/develop` first."
