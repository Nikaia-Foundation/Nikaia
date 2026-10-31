1. **Most turns:** finding out what caused the red. The job logs come from a blob URL that the proxy blocks (403), so I could not read them directly. I had to put it together from the failing steps, the annotations, the commits since the last green run and the y3 report, then build the compiler and run `lower-std` to confirm. That took about 8 turns.
2. **What the brief could have handed me:**
   - The red's failing test names, plus the first red and last green sha. `architect-input.txt` gives only the job names.
   - The fact that the driver runs the suite and pushes after the architect. I read `drive.nika` to learn this, and it decided both `done` vs. `blocked` and whether I could push at all.
   - That y19 had no report line and that the rules block still says push and `owner decision`.
3. **Checks:**
   - Useful: the CI runs and jobs lists, the commit stats, the annotations, the build plus `lower-std`, the duplicate search that found #927, and reading the tree types in `main.nika`.
   - Wasted: two attempts to fetch the logs (`gh api` returning a redirect, then curl) and one grep over an empty log file.
4. **One change that would save the most:** have the driver write into `architect-input.txt`, on a RED, the failing tests (from the annotations), the first red and last green sha, and the unpushed shas with their reports. Also correct the brief's rule 5: "push" is wrong while the driver holds untested commits locally.
