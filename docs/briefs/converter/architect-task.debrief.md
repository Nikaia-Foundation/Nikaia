1. **Most turns:** working out which signal was current. The converter directory held an empty `RED` file, a `qa-input.txt` from 12:06 saying the suite was red, and a `suite.log` still compiling, while `architect-input.txt` said `CUT`. Comparing timestamps, checking `unpushed.txt` and running `git fetch` took about 3 of my roughly 13 tool calls. The other cost was deciding about the push: the task says push, the charter says don't, and local `develop` held untested commits.
2. **What I looked up that the task could have handed me:**
   - the Deviation line itself, which the cut line already quoted in full;
   - the converter's existing rule for standard trait impls (`tr_items.nika:567`, std trait impls become ADAPTER comments), needed to judge the deviation;
   - whether b29 and b30 were built, which I found by reading `git log`.

   One line in `architect-input.txt` would have saved most of this: the shas of commits on top of `origin/develop` that are still unpushed, and each one's brief.
3. **Checks:**
   - Useful: the cv1 report, its test files, a search for Display in the converter, the Deviation rule in `common.part`, cv1's own brief (it had promised "stays an impl"), and the heads of cv2 and cv3 for staleness.
   - Wasted: the `fetch` and `merge-base` call, which failed with exit 1; tailing `suite.log`; a separate search for b29/b30 lines before the edit; and the builder.md Deviation search, since that rule already exists.
4. **Change that would have saved the most:** in `architect-task.md`, have the driver write two things into `architect-input.txt`:
   - the suite state (running, red or green) and the unpushed commits;
   - for a deviation-only cut where the report says `Result: done` and `Gate: green`, a note that the queued briefs are unchanged.

   Separately, step 2 should say "commit; push only when local `develop` holds nothing beyond your commit", so it agrees with the charter's rule against pushing. Also, cv1's brief set the expected output (an impl that stays an impl) without running it first. That is the existing "probe every test row" check (`docs/architect.md`), which was not followed; no new rule is needed.
