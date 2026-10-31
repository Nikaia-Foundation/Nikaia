1. **Most turns:** finding out why CI was red. The job logs come from a blob URL that the proxy blocks (403), so I fell back to check-run annotations. Those only said `failed: signature`, and from there I had to map it to the json ledger and confirm that 71aa29ae fixes it. Running all the examples with `--locked` was the slowest single step.
2. **What I looked up that the brief could have handed me:**
   - `architect-input.txt` held only the CUT line, while `docs/briefs/RED` was an empty, git-ignored leftover. I had to work out whether CI was really red myself.
   - The id of the last red run and its failing jobs. The driver knows these but did not pass them on.
   - The architect.md excerpt (*Briefs and reports* plus *State*), since the whole file is over the read limit.
   - On the compiler side: where the kept-lambda cast is written (`emit_flow_kept`), the `kept_lambdas` fact, and `plain_rust_spelling`. Those were real research for #1029, so I'd keep them as my job.
3. **Checks run:**
   - Useful: the 14 probe programs, which turned "a match tail" into "any integer literal result"; reading the generated `.rs`; diffing the new brief's tail against `common.part`; the open-issue list, to rule out a duplicate.
   - Wasted:
     - the job-log download, which the proxy always blocks;
     - one probe written in `catch` syntax I didn't know;
     - the struct-field probe `(op.run)(3)`, which is not a Part I form.
   - Not essential: the 16-example `test --locked` loop. 71aa29ae's commit message already stated the cause, so this only confirmed that no other example ledger was out of date.
4. **One change:** when the driver hands over `RED`, write the failing run's id, its jobs and the check-run annotation text into `architect-input.txt`. It should also clear `docs/briefs/RED` once it has done so. Add one line to architect.md: "job logs are not reachable headless; read `check-runs/<job>/annotations`". That would have saved about 8 turns.
