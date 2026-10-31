1. **What cost the most turns:** stale files in `docs/briefs/bugs/`. `RED` (an empty marker from 18:43), `qa-input.txt` (from 17:50) and an unfinished `suite.log` all looked like a red suite. It took two reads and a timestamp check to see that the batch was only cut on b15's `Found:`. The next cost was the push. It was rejected, and when I tried a second commit I found another driver halfway through a rebase in the shared checkout. I had appended a line to my report before I checked for that, so I had to put the file back.

2. **What the brief could have handed me:**
   - whether b15's commit was already on `origin/develop`;
   - whether its `Found:` had been filed (it had: #1081, by the previous architect);
   - which queued briefs carry `Blocks:` lines;
   - the status labels of the queued issues.

   All of these are facts the driver already knows. Each one cost me a git or `gh` call.

3. **Checks:**
   - Useful: the `git branch --contains` check (is b15 pushed), the issue labels, the grep for `Blocks:`, the check that each brief has its report path.
   - Wasted: `ps | grep -c gate.nika` (a count of 3 tells nothing), reading `qa-input.txt` (stale), `cat -A batch.txt`.
   - Skipped, and the charter requires it: I did not re-run the probes of b17, b18 and b19 on today's binary before queueing them again. I relied on the previous architect's checks.

4. **The change that would save the most:** have the driver write a short state block into `architect-input.txt`:
   - the reason for the cut (Found, red or done);
   - whether the last commit is pushed;
   - the issue number if the `Found:` is already filed;
   - the queued briefs, with their `Blocks:` targets;
   - whether a rebase is in progress in the checkout.

   The driver should also delete `RED` and `qa-input.txt` once a later suite run is green. And `architect.md` should say: check `git status` for a rebase in progress before any commit. If one is running, commit nothing and leave the push to the driver.
