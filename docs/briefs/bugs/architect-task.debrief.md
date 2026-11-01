1. **Most turns:** working out what the driver's `CUT b22` line meant. `architect-input.txt` named a report from 2026-10-31. The batch I had just reviewed was b29–b32, all done. Comparing file times, `git status` and the State section took about 4 turns before I saw the cut was a stale report, not a new failure.
2. **What I looked up that could have been handed over:**
   - which reports belong to the last batch (the driver could list them by name);
   - whether run 1's uncommitted edit was still in the tree;
   - the badge rule (it was already in `docs/builder-facts.md`, but I had to search for it);
   - where the text-tier code lives, to trace b30's Found;
   - how `badges.py --check` interacts with the pre-push check.
3. **Checks run:**
   - Useful: the b30 Found probe plus its one-arm variant (reproduced it, needed before filing #1132), and re-running the b33 and b21 probes on today's binary (cheap, and confirmed the briefs still hold).
   - Partly wasted: `where.nika` failed (`nikaia` is not on PATH), so I fell back to grep. Reading `badges.py` was also wasted, because builder-facts already covers the trap.
   - Fetching the open issues was needed, but most of the 264 entries didn't matter for a batch of three.
4. **Change that would save the most:** the driver should write into `architect-input.txt` which briefs are cut, done or unrun, with each report's time, and flag a report older than the last architect run as stale. Separately, the driver should move a stale `partial` report to `.runN` itself. A smaller fix: name the binary (`target/fast/nikaia run scripts/where.nika`) in the architect task, not a bare `nikaia`.
