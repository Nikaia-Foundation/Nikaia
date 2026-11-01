1. **What cost the most:** confirming cv2's `Found` before filing it. I had to learn the `tests/compiler` package layout, find the one-file test for #1023 and write a two-file scratch package. That was about 5 turns, and the 3-minute build was the slowest single step. Choosing between pushing and not pushing was the other cost: the task says push, my instructions say the main session pushes.
2. **What I looked up that the brief could have handed me:**
   - That `architect-input.txt` was a `CUT` on a `Found` from a run that otherwise finished. The bare `RED` file and the stale `qa-input.txt` (12:06) beside it looked like a red suite until I checked their times.
   - Where `code_on_a_cycle` is called, and how 72e7789c fixed the pausing graph.
   - The next free bug-brief number.
   - What `common.part` says.
   - That `nikaia` is not on PATH (use `target/fast/nikaia`).
3. **Checks I ran:**
   - Useful: the two-file probe (it confirmed the defect), `where.nika` on both names, reading `emit_reach.nika` and `git show 72e7789c`.
   - Partly wasted: listing the converter directory with times and the `qa/` folder just to rule out a stale red; a broad `grep beside` over the tools files; looking at the cv3 frontmatter, which was already fine.
4. **The one change that would have saved the most:**
   - The driver should write into `architect-input.txt` the stop reason, the unpushed commits, and whether the suite has passed on them.
   - It should delete `RED` and `qa-input.txt` once QA has handled them.
   - The task should say plainly that the architect commits and the driver pushes after its suite. Today step 2 says "push", which conflicts with the rule that the main session pushes.
