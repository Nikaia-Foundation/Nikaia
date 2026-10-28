1. **What cost the most turns.** Finding out that CI was red, and why. The input said `BATCH DONE`, but `develop` had been red since c7e5b66. I spent about 8 calls walking the run list, the failed jobs per run, the annotations, the job steps and a log download the proxy blocked. Even then I never learned the cause, because the sweep step writes no annotations.
2. **What I looked up that the brief could have handed me:**
   - **CI state.** `architect-input.txt` should carry the CI line (`status.nika`) and the failing job and step, not just the stop word.
   - **Code locations.** P5a and M3a needed function line numbers, the existing Nikaia twins (`project_settings.nika`, `modules.nika`) and the host-callback precedents (`SysrootHost`, P4b's `sha256_hex`). P5-plan's debrief already asks for these.
   - **`std` coverage.** I couldn't cheaply check whether `std::fs` has `append` in Part III 17: the grep came back as 50 KB. So I left `trace` as the builder's choice.
3. **Checks run and wasted:**
   - **Useful:** run and job lists, `where.nika` for callers, a range sanity check against a4b33a0 (no change since), and a grep of the cut ranges for `std` and host calls.
   - **Wasted:** the job-log download (proxy-blocked, as the State already said), a search for which test dumps converter files (no answer), and two Part III greps that were too broad.
   - **Missed:** `git pull --rebase` failed on another lane's unstaged files. The push went through anyway as a fast-forward (origin/develop is fb686ff), but I didn't check the push output directly.
4. **One change that would have saved the most:** make `drive.nika`/`cycle.nika` emit `RED` whenever the latest completed run on `develop` failed, with the failing job and step, and put that into `architect-input.txt`. Also add the sweep annotations as a standing CI rule. Together they would have saved about 10 calls, and the red would have been caught five hours earlier.
