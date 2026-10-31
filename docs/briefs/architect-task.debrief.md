1. **Most turns:** reading the source to design the two host traits. That was about 550 lines of `comptime_run.rs` (716-1264) and about 450 of `project.rs` (`drive_to`, 2137-2586). I also had to find every outside caller of what moves (`run_fast.rs`, `grammar_run.rs`, `sysroot.rs`, the `project.rs` imports). The second biggest cost was a dead end. `docs/architect.md` State said GR was done. I only learned it had been reverted (0fbe3d6b) when `where.nika` failed on `grammar_run.nika`, and then I had to trace #1060 myself.
2. **What I looked up that could have been handed over:**
   - which plan rows are ready, and what blocks each (K3b needs #922 step 4);
   - the std entries a move needs (`process::run`'s signature, `fs::Metadata`, `in_nanos`, `hash::sha256_hex`);
   - the host-trait conventions from earlier moves (b070adb9's `impl Error for CheckWall`, the `ProjectMembersRefusal` pattern);
   - the free bug-brief number.

   `gh api search` returned 403, which cost one call.
3. **Checks:**
   - Useful: the `gh` issue listings, the `grep -n` passes for line ranges and callers, and the push that showed 11 commits held by the driver.
   - Wasted: the `where.nika --outline` call, and re-reading `selfhosting-plan.md` rows I had already seen before the summary.
   - Not run at all: no probe of the #1060 cause. The b15 brief marks it as a suspicion.
4. **Change that would save the most:** the driver should write a fact block into `architect-input.txt`:
   - reverts since the last run (`git log --grep=Revert`);
   - plan rows with their prerequisites and status;
   - open `ready` issues without `wip`;
   - the next free `b<N>` number;
   - whether local `develop` is ahead of or behind origin.

   Also, the State section should be rewritten by the step that commits a revert. A stale "GR done" sent this run in the wrong direction at first.
