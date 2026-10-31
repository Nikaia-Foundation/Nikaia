1. **What cost the most.** Reading `docs/architect.md`. It is about 34k tokens, so it went over the read cap: one read was truncated, one failed, and a third read with a line limit was needed just to reach *State*. After that, finding where the function's parameters enter the scope took two reads, and the first one landed in the grammar-action code at `check_function.nika:280`, the wrong place.

2. **What the input could have handed me.** `architect-input.txt` only said `CUT b1023…: partial`. I had to open the report and the brief myself to learn that the fix was in the checker (`check_fields.nika:213`, `check_references.nika:295`). I also had to grep for these:
   - whether k3a2's precondition named b1023;
   - how `CheckLocal`, `binding` and `frame_holding` are shaped;
   - that `run_code` already handles shadowing (`check_core_stmt.nika:500`);
   - the other callers of `a_lent_parameter`.

   None of this needed an ADR, the language spec or the converter.

3. **Checks I ran.**
   - **Useful:** re-running the issue's probe at b4938048 (still E0308, as the charter requires); checking the issue's label; checking that `origin/develop` reached ce3dcfce after the push.
   - **Wasted:** the read of `check_function.nika:280-312`, and the `architect.md` read that failed.
   - **Not run:** I wrote down two claims without probing them: that a lambda's capture still finds the parameter's frame, and that the other callers in `check_arm_leaves.nika` should give the same answer. If either is wrong, b1023b could stop on it.

4. **The one change that would save the most.** Have `drive.nika` copy the cut report's `Deviation:`, `Found:` and `Needs:` lines and its path into `architect-input.txt`. Also move *State* to its own short file (for example `docs/architect-state.md`), so a fresh architect reads 2k tokens instead of 34k. Together these would have saved about four turns and most of the context.
