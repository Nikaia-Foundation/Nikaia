1. **Most turns lost:** a stray `cat > $TMPDIR/c.md` with no input sat waiting on stdin and hung the `gh` call. It ran into the 120 s timeout and went to the background, and killing it and rewriting the scratch file cost about 5 turns. After that came the probe syntax: grammar lists (`**` versus `list(…)`) and the `std::collections` import took 4 failed probe runs.
2. **What the brief could have handed me:**
   - The grammar list form: `list(item, sep)`, Part II 10.8, line 667.
   - The fact that a single-file probe needs `use std::collections`.
   - The location of the `ParseError` rendering: it lives in the external `winnow-grammar` checkout, `error.rs` (`rule_stack`, `render`).
   - That `Gate: none` cuts the batch, and how the driver's `CUT … Gate: none` line maps to "j4 is in fact done".
   - Whether an open issue already held j4's Found. I searched and found #1073.
3. **Checks I ran:**
   - Useful: the j6 probes (nested members, `[1,2,x]`); the NK1111 probe; the duplicate search for issues; `grep` on `batch.nika` for the cut rule.
   - Wasted: the user-program reproduction of j4's lowering defect, which didn't reproduce because the trigger is the Keep frame in `lower-std`; the first two `gh` search calls, which lacked `is:issue`; the `Map`/`use` probe attempts.
4. **One change that would save the most:** in `architect-task.md`, add a *Probe kit* line: a grammar-probe template that runs (with `list(…)`, `members`, `use std::collections`), a reminder to run `gh` with `</dev/null` and search with `is:issue`, and where `ParseError` is rendered. That would have saved about 10 turns.
