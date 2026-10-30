1. **Most turns:** finding where the lowering decides between `impl Fn` and `Kept`. I spent about five grep and sed calls across `emit_core_types.nika` and `emit_function.nika` before finding `emit_fn_keeps` and `code_parameter_runs`. Building the probe matrix (p1–p4) took four runs. Each was needed, but writing p4 first would have covered most of what I learned.
2. **What I had to look up that the task could have handed me:**
   - That RED was the empty state file again, not a failed job. That cost two `gh api` calls.
   - Whether the saved k3a2 patch in another session's `/tmp` still existed.
   - The trait and `fn`-parameter lowering path (ADR-277 D11, `emit_fn_keeps`).
   - The format and placeholder of `common.part`.
3. **Checks:**
   - Useful: the CI run list and the failed-job query (they showed no red), probes p1–p4 (they turned one Found into two defects plus a control row), the patch existence check, and the check that the push reached `origin/develop`.
   - Wasted: two failed git calls (a pathspec after `git mv`, and `git status --cached`), plus re-reading the long *State* history in `architect.md` when only the newest entry mattered.
   - Not done: I did not probe the recursive trait row; the brief says so.
4. **The change that would save the most:** have `drive.nika` write the CI state into `architect-input.txt` (red job names, or "in progress, none failed") instead of a bare `RED` from an empty file. Second: show only the three newest *State* entries in `architect.md` and move the rest to an archive file, which would save about 25k tokens per architect run.
