1. **What cost the most.** Getting oriented. `docs/architect.md` is about 34k tokens, mostly a long *State* diary, so I had to read it in pages. After that I had to work out the batch and CI state myself: which briefs had reports, which runs were red, and whether the red was already fixed forward. Last came the rejected push, then three amends while I tried to write the report's own commit sha into the report, which can't work.

2. **What I looked up that the brief could have handed me.**
   * The CI state and failing job. `architect-input.txt` said only `CUT`.
   * Whether 0b8a3acb already fixed the red.
   * ADR-277's path (my first glob, `docs/adr/`, was wrong).
   * The emitter locations for `fn` parameter shapes.
   * That a call graph already exists (`pausing_reach`).
   * That k3a2's two functions call each other, which I found in the run 4 patch.

3. **Checks I ran.**
   * Useful: probes d2, d3, d3m (the mutual-recursion case) and d3s (the case that already works), `git apply --check` on run4.patch, the CI job annotations, and `git log HEAD..origin/develop` before deciding whether to rebase.
   * Wasted: probe d1, since a user program never reaches the crossing form; the wrong ADR glob; the two extra amends.

4. **One change that would have saved the most.**
   * `drive.nika` could write more into `architect-input.txt`: the CI line from `release.nika --dry-run` (red sha, failing test, newer commits that touch it) and the report status of each brief in `batch.txt`.
   * Move old *State* entries out of `architect.md`, so the charter is one read.
   * The architect's report form should drop `Pushed:`, or the brief should say how to push from a dirty shared tree, since the driver has to rebase anyway.
