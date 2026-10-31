1. **Most cost:** checking the probes. `target/fast/nikaia` was older than b11's commit, so I had to find the suite binary and re-run row d with it. A side-trip on row d also cost two runs: writing `String` instead of `ref String` gets NK1106 at the field, which is a different matter from the defect.
2. **Looked up:**
   - which binary is current;
   - where #1057 rows b and c and #947 go wrong: the field branch of `check_core_a_lent_place_given_away`, the places that set `copied_jump_reads`, and the callers of `views_the_call_s`;
   - whether #938, which held #947 back, was closed;
   - the format of the previous brief, to copy its rules block.

   The task brief could have said that #938 is done, and that the suite binary is the current one.
3. **Checks:** the issue list, b11's report and debrief, the probes (#1061 a, #1057 b and c, #947 a, b, c and the typed-list row), and `where.nika`.
   - Wasted: the first round of probes on the stale `target/fast` binary. Its #1057 results matched the suite binary, but only by luck.
   - Not run: #947 row d, which the b14 brief tells the builder to re-probe.
   - The push failed: `git pull --rebase` cannot run while the tree holds other agents' unstaged files, so a rejected push can never be repaired by the architect.
4. **Change that would save the most:** the architect task should say "probe with `/home/user/nikaia-suite/target/fast/nikaia`, the last tested commit". It should also say "on a rejected push, leave the commit local; the driver pushes it", instead of an instruction to rebase that cannot work in the shared checkout.
