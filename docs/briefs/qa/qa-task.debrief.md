1. **Most turns.** Working out why `badges.py --check` passed in the shared tree but failed in CI. I needed a `git archive HEAD` export to see that the 83.3 % only held with another agent's uncommitted `emit_core_leaves.nika`. After that, `commit.nika` refused my commit twice. The first refusal was a `Files:` field I had written with commas and a missing `Changelog:`/`Attribution:`. The second was because another session had already committed the same badge fix, so the file was unchanged.

2. **Lookups the brief could have handed me.**
   - `qa-input.txt` is mostly raw CI log noise. The failing line was one grep away, and the driver could have put it at the top.
   - The report format for `commit.nika` (space-separated `Files:`, required `Changelog:`, the `Attribution:` trailer) was only in the script's header comment.
   - `nikaia` is not on PATH. I had to find `target/fast/nikaia`.
   - That the share badge counts lines in the working tree, so uncommitted files from other agents change it.

3. **Checks.**
   - Useful: `badges.py --check` in the shared tree (rc 0, which showed the discrepancy), and the same check on the `HEAD` export (rc 1, then rc 0 with the fix).
   - Wasted: reading `qa.md` and the first 100 lines of the log, which were all runner setup, and `git log` on the badge file, which I did before checking whether someone else was already fixing it.
   - I did not rerun the check in the shared tree after the fix. It would now show 83.2 % against the other agent's uncommitted state, but that is not my change.

4. **One change.** Add to `qa.md` that the failing check must be reproduced on a `git archive HEAD` export, never in the shared checkout, and that `git log origin/develop..HEAD` should be rechecked just before committing, because other sessions commit fixes concurrently. Having the driver print the failing command at the top of `qa-input.txt` would save the rest.
