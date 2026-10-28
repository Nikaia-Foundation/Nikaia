1. **What cost the most:** working out the cause of the red without any logs. That took about 8 calls: check-runs for several commits, the run list, the failing step of one job, then `git log` over the red range. The cause only showed up when I asked for check-runs on eeaefb1 itself and got none back, so eeaefb1 never had a run of its own.
2. **What I had to look up:**
   - Which commits were in the first red push. `git log 590cd7c..c7e5b66` gave that in one call, but the input named only c7e5b66.
   - How the sweep dumps programs (`accepted_sweep.rs`, `check_against`).
   - The workflow's line numbers.
   - The `Get` impls in `index.rs`.

   The task's input line (`CUT ...: partial`) carries no CI state. A `RED` line with the last green sha and the first red sha would have handed me the range.
3. **Checks:**
   - Useful: the run list, the failing-step query for one red run, `git log` over the range, and `git show --stat eeaefb1`.
   - Partly wasted: check-runs for 60b1154 and 3719095 (still running) and for a783081 (I needed the run ID, not the job list).
   - Not done: I never confirmed the cause, so the brief names three suspects rather than a known one.
   - The disk note about `b-fixforward-sweep2.md` was my own `sed` edit (the `mktemp` path), not someone else's change.
4. **The one change that would save the most:** `drive.nika` or `release.nika` should write into `architect-input.txt` the last green sha, the first red sha, and the commits between them (`git log --oneline`). It should also list the failing job/step pairs from `actions/runs/<id>/jobs`, and the sweep's annotations once they exist. That would have taken me straight to eeaefb1 in one read, and spared the first fix-forward its 12 greps for a converter test.
