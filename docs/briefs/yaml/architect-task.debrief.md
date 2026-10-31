1. **Most turns:** working out what the stop actually was. `architect-input.txt` held only a `CUT` line, but `ls` also showed an empty `RED` file, so I read the driver code (`drive_action`, its input writing) and ran `git check-ignore` before I could rule out a red CI. After that, the failed `git pull --rebase` (refused because of another agent's uncommitted `nikaia.lock`) cost one more round before I pushed fast-forward instead.

2. **Looked up, could have been handed over:**
   - That `RED` in a lane dir is the driver's gitignored marker, not a CI state.
   - ADR-361 D9's exact wording ("a branch"), which decides whether #1021 is a defect or needs the owner.
   - Where the prover handles `Expr::If` and `Expr::Match` in `prover_walk.nika`.
   - That y2 had no stop-15-turns rule and was too big for one run.

3. **Checks:**
   - Useful: reading y7b's report and debrief, #928's *To build*, ADR-361 D9, a grep for the `If`/`Match` arms, and a grep of the new briefs for report paths and the stop rule.
   - Wasted: the 64 KB grep of Part III for `match` (spilled to a file, never read), and the `architect.md` State grep (nothing for this lane).
   - Gap: I didn't re-check y4c's facts (json's line 196, that dfe7bac6 is still on its branch) before putting it in the batch again.

4. **One change:** in `drive.nika`, write `architect-input.txt` with an explicit first line `STOP: CUT|RED|BATCH DONE`, plus the stopping report's `Found:` line in full instead of cut off. A note in the architect task saying `RED` files are markers would have done the same. Second: the architect brief should say how to push when someone else's unstaged files block `git pull --rebase` (fetch, then push fast-forward).
