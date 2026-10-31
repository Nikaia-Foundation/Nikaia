1. **Most turns:** finding the code for the two bug briefs took about 8 grep rounds. For #1017 I had to find where a `-> T?` function wraps its tail values in `Some` (`some_at_the_tails` in `check_function.nika`). For #1018 I had to find where index reads are emitted and copies decided. I used plain grep rather than `scripts/where.nika`, which the charter prescribes. A smaller cost: `docs/architect.md` is about 35k tokens, so one read failed and I had to read it in slices.

2. **What the input could have handed me:**
   - The report path and a one-line state of the previous brief's preconditions (was k3a2 on `develop`?). I got these from `git log`.
   - Whether Part III specifies `fmt`'s tab style, to judge the Found.
   - Part I's rule that a copy of a function value shares the one closure.
   - For each `ready` issue, the `file:line` of its likely site.

3. **Checks:**
   - Useful:
     - Re-running the #1017 and #1018 examples. This caught that the `fs[0](2)` row doesn't parse.
     - The `fmt` probe on a copy of `lends.nika`, which showed the Found was specified behaviour.
     - The spec greps for `fmt` and for function-value copies.
     - Checking CI status and the `ready` issue list.
   - Wasted: the failed full read of `architect.md`, the `fs[0].clone()` probe (out of scope), and one grep for wrap or lift helpers that found nothing.
   - Not run: I didn't probe #1017's one-line lambda row, and I didn't trace either fix through the emitter. The brief marks the one-line row unverified and tells the builder to check the emitter path.

4. **One change:** move the dated *State* history out of `docs/architect.md` into its own file, keeping only the latest entry. Then have `drive.nika` write the cut report's path and the preconditions it checked into `architect-input.txt`. Those two changes would remove the slice-reading and the git archaeology.
