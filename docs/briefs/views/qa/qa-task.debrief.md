1. **Most time:** The 10-minute `nikaia test --both-settings` run on the old binary. It passed, and it told me nothing, because the failure was in a commit older than my HEAD. The slow part of the diagnosis was finding the failing program's name. `qa-input.txt` only had the last 200 log lines, and those were all passing tests. I found the name with `gh run view --log-failed`.

2. **Lookups the brief could have handed me:**
   - The CI run's head SHA, and whether it is an ancestor of the current HEAD. This was the actual cause: the run tested `d0855a7a`, before the fix `e95ea023`.
   - The failing program name. The driver's grep matches `panicked at` and `^error`, but not `program "X" ... FAILED` or the `--- program` block.
   - The `gh` command that fetches the full failed log.
   - The fact that 727 + 1 = the 728 committed programs, so the untracked test files in the shared tree don't count.

3. **Checks:**
   - Useful: the `gh` log grep (found the failing program), the ancestor check (found the cause), and the `cargo build` no-op (showed the binary was current).
   - Wasted: copying the old binary into the scratchpad, and the full both-settings run, which I started before knowing the CI commit. Reading `agent_facts.nika` and the diffs of `agent_facts.stdout` and `agent_script.nika` was partly wasted for the same reason.

4. **One change:** Have the driver put the CI run's head SHA and the names of the failing programs in `qa-input.txt`. Also add one line to `qa.md`: first run `git merge-base --is-ancestor <fix> <run sha>` for the failing commits. If the red comes from a commit before a later fix, write the report and stop. Do this before any long rerun.
