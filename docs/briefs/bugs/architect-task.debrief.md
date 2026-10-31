1. **What cost the most:** working out what the `RED` line meant. `architect-input.txt` held `CUT …` and then a bare `RED`. Next to it were a stale `qa-input.txt` and a `suite.log` whose last run was green. I had to read `drive.nika` (around line 975) to see that the driver's RED always names a commit and says "The failing jobs…", and then check CI on `develop` to be sure. That took about 4 turns, and the last architect lost time to the same confusion.

2. **What the brief could have handed me:** most of it was in `docs/architect.md`'s *State* section and the reports. Still, I had to look up four things myself:
   - whether the last batch's commits were pushed;
   - the state of CI on `develop`;
   - which queued briefs carry `Blocks:` lines;
   - which issues the other lanes hold (`docs/briefs/batch.txt` and `yaml/batch.txt`).

   The driver knows all of these and could write them into `architect-input.txt`.

3. **Checks:**
   - **Useful:** `git log origin/develop..HEAD` (empty, so everything was pushed); the CI runs list; re-running the b18 and b20 probes on `target/fast` (both still fail); a grep of the queued briefs for `Blocks:` and report paths; and a check for a rebase in progress before committing.
   - **Wasted:** reading `qa-input.txt` and the tail of `suite.log`, both stale.
   - **Not run:** b13's probe.
   - **One claim I didn't verify:** my comment on #1077 says the gate's `lower-std` fixpoint plus `cargo check` would catch a missing trait import. I didn't check that the gate really runs `cargo check -p nikaia-std` with the tools package compiled. It should be verified, or the comment softened.

4. **The change that would save the most:** have `drive.nika` write a fixed state block at the top of `architect-input.txt`:
   - `Cut: <brief> <reason>`;
   - `Pushed: yes|no`;
   - `CI develop: <sha> <conclusion>`;
   - `Queued: <brief> Blocks: <x>`;
   - `Held elsewhere: #N…`.

   It should also delete `RED` and `qa-input.txt` after a green suite. Until then, the architect brief should say that a `RED` line without a commit and failing jobs is stale and only needs one CI check.
