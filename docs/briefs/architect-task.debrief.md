1. **Most turns: finding why CI was red.** The task file said `CUT ... partial` and nothing about a red build. I found the untracked `RED` file only by chance. Then I spent about 8 turns on the cause: listing runs, finding the last full green run (docs-only runs also show "success"), and trying the job logs, which the proxy blocks. Only the check-run annotations showed the test name. I also read the long `architect.md` in parts and hit the size limit twice.
2. **What I looked up that could have been handed to me:**
   - The failing test and job, from the annotations.
   - The last green full run.
   - ADR-285 D34/D43 for the nested-list question.
   - The D34 code locations: `check_core_stmt.nika` for the `let` and the `for`, `check_numbers3/4.nika`, and `CheckOpenNumber`.
   - The fact that `nikaia test` has no name filter. My first draft of b938b got this wrong; I fixed it before committing.
3. **Checks:**
   - Useful: lowering the failing program, which showed the stale text; the grep for other tests asserting the same text; rows a–g plus the typed versions; the #938 issue state; the scan of `bred1.md.report.md`.
   - Wasted: two failed log downloads, the arm64 job loop over 40 runs (one look at a single run's jobs would have done), the GitHub search API (blocked), and the long `sed` on the State section that overflowed.
4. **Biggest saving:** `architect-input.txt` should carry the CI state with every stop line, not only on `RED`. That means the last green full sha, the first red sha, and the failing test names from `drive.nika` (via annotations). `architect.md` should also say in one line that job logs are unreachable and annotations hold the test name. A size-limited *State* (the last 3 entries, older ones moved to an archive) would save the paged reads.
