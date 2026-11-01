1. **Most turns:** working out whether the RED in `architect-input.txt` was still live. The input named only 05fffc84 and the job. I had to compare the CI runs for `develop` and `ci/briefs`, list their jobs, and find out whether QA had already fixed it. QA had, and `ci/briefs` was green at fb4c04b6. That took about 4 of my roughly 12 calls.
2. **What I looked up that the brief could have handed me:**
   - The right QA report. I first read the old `docs/briefs/qa-task.md.report.md`, which talks about ADR-384. The current one is `docs/briefs/qa/qa-task.md.report.md`.
   - Whether the batch had run. It had not: there was no K3b run 3 report and no C1c report.
   - Whether b28's Found was already filed. It was, as #1124, which I found by searching the open issues.
   - Whether 707d5e53 still applied cleanly on the new HEAD.
   - I needed no ADRs, language rules or converter facts.
3. **Checks:**
   - Useful: the CI runs and jobs; `git log --since` over the reports; the `Found`/`Result` lines of the new reports; the #1124 body; the `merge-tree` check of 707d5e53; checking where `origin/develop` stood after the push.
   - Wasted: reading the old QA report; listing the bug-brief numbers, since I wrote no bug brief; grepping the K3b brief for its turn limit; checking which branches held 4001fe0b.
4. **The change that would have saved the most:** when QA has fixed the red, the driver should say so in `architect-input.txt`. That means the QA commit, the green CI run's sha, the batch briefs still unrun, and the paths of the new reports, the QA report included. The task should then say: "red already fixed and batch unrun → keep `batch.txt`, file new Founds, report." The run would then take 3–4 calls.
