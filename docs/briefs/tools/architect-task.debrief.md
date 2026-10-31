1. **Most cost:** tracing the cause of #1108. That took about 8 calls: grep for the message, then `bt_may_not_run`, `callee_named`, and the `sync.nika` and `touch.nika` arms. Setting up the probes cost more turns:
   - My first probe used `sync fn` and failed to parse; `sync` goes after the return type.
   - Several runs printed a page of rustc warnings, so I had to rerun each one with `2>/dev/null` and a grep.
2. **What I had to look up that the brief could have handed me:**
   - The brief says "j1, j2 are done", but j1-j7 were done. One line naming the last finished brief would help.
   - Whether `RED`, `qa-input.txt` and `suite.log` were stale. `qa-input.txt` was from 19:58 and `suite.log` was cut off. The input file decided it, but the brief could say which of these files are current.
   - The spec's file names (`20-nikaia-advance.md`) and where `NK1244` and Part II 10.2 sit.
   - That #97 already tracks boxed patterns.
   - Whether a push could ever succeed with untested builder commits on HEAD. The task says to push; it is always rejected or risky here.
3. **Checks:**
   - Useful: probes of both Found defects, the panic-free turn shape for j9, `NK1244` on an overflow, and the per-field `wire_name` run at build time for 3c.
   - Wasted: the first `sync fn` probe, an unfiltered run, and a `gh` search that only found #97.
   - Not done: j8's test rows are still unprobed. The previous architect flagged this and I did not fix it. j9's `NK1244`-on-panic row cannot be probed before the fix.
4. **One change:** in the tools `architect-task.md`, replace "j1, j2 are done" with "read `batch.txt` and the newest reports; push only your own commit with the merge-tree recipe (`docs/architect.md`), never a HEAD that carries unpushed builder commits". Also add a line to builder-facts on where `sync` and `throws` go in a signature, for probes.
