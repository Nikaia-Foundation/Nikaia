1. **Most cost:** tracing the red CI. The red signal was in `architect-input.txt`, not in `RED`, which was empty, and I misread that at first. From there it took three `gh api` calls to find the first red run (a99ce32), check that no prover commits were in it, and get the names of the failing tests. The job logs can't be fetched here, but the check-run annotations had the test names.

2. **Looked up, could have been handed over:**
   - Which file carries the red: `architect-input.txt` or `RED`.
   - The last green sha.
   - Whether the lane's commits are in the first red run.
   - That check-run annotations, unlike the job logs, list the failing tests.
   - That the uncommitted `lowered/project_members.rs` is already known from the GR and p3 reports.
   - That 84aa402 is on no branch.

3. **Checks:**
   - Useful: the list of runs to find the first red; `merge-base --is-ancestor` for the prover commits; the annotations; `git diff --stat` of the shared tree.
   - Wasted: `git log -S TrustedMap`, which found nothing, and most of the 84aa402 search.
   - Not run: a fresh count of the figure (I quoted p3's), and a check that the uncommitted file is exactly what `lower-std` writes at HEAD.

4. **One change:** when CI is red, the driver should write into `architect-input.txt` the first red sha, the last green sha, the failing tests (from the check-run annotations), and whether any of the lane's commits are in the first red run. The task template should say that `RED` is the driver's file and that rule 7 can then be settled with no tool calls.
