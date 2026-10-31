1. **Most turns:** narrowing #1101's probe into a table of shapes (four runs) and tracing the likely fault through `tuple_literal` and the `return` check. Second was making sense of the state: what the empty `RED` file meant, whether a suite was running, and that HEAD had been rebased under me while I worked.
2. **What I looked up that the brief could have handed me:**
   - `architect-input.txt` holds only a cut-off `CUT …` line, so I had to open the report to get the Found.
   - Whether a suite is running on which sha, and whether the empty `RED` file is current.
   - How the tuple and text-tier checks are split across `check_texts.nika`, `check_core_leaves.nika` and `check_function.nika`.
   - That earlier architect commits sat unpushed, which settles whether I should push at all.
3. **Checks:**
   - Useful: the probe and its shapes on `target/fast/nikaia`, re-running the three queued probes, `grep` for the NK1104 message, and listing the issues.
   - Partly wasted: the outline from `where.nika` (it showed definitions, not the `return` path), reading `check_function.nika`'s tail path (the error comes from the `return` statement), and the first `ls` of the whole bugs directory (very long).
   - The issue list could be cut down to `ready` issues without `wip`.
4. **One change:** have the driver write into `architect-input.txt` the full Found line, the suite state (running on sha X, or green/red), whether a `RED` file is current, and an explicit "push: driver / architect". The task brief's step 8 ("push") clashes with the driver's gate whenever untested builder commits sit ahead of `origin/develop`. Add to `architect.md`: "when HEAD holds builder commits the suite has not passed, commit locally and leave the push to the driver."
