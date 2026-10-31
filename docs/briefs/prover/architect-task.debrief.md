1. **What cost the most:** collecting the open items. I read 12 reports, 13 debriefs and #1053, then ran three GitHub searches to see which `Found:` defects were already filed. Second was a confusing state at the start: an empty `RED` file, an old `qa-input.txt` naming red tests, and `batch.txt` still listing p12. I had to check file times and running processes to learn that p12's suite was still going and nothing was red.
2. **What the brief could have handed me:**
   - That an empty `RED` means the last gate was green (it is in `agent.nika`, I grepped for it).
   - That p12 was committed but unpushed and its suite would run alongside my run.
   - Which `Found:` lines had already become issues: p6 → #1074, p9 → #1081, p10 → #1090.
   - The current figure (I took it from p12's report).
3. **Checks:**
   - Useful: `ps` for the suite, the `grep` of `Found`/`Needs` across the reports, the issue searches, and grepping `builder-facts`/`formal-verification` before adding traps so nothing was duplicated.
   - Wasted: reading the p13 brief's tail and the long rules block in full; reading the whole of #388 when its title was enough. I did not run a fresh figure, since p12's report already gave one on today's tree.
   - Expected: the push was rejected because HEAD held the untested p12. One `git status -sb` before committing would have shown that.
4. **One change:** the landing section of `architect-task.md` should list, from the driver, the unpushed commits, whether a suite is running, and every report `Found:` with its issue number if one exists. That would have saved about a third of the turns.
