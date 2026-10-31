1. **Most turns: finding why CI was red.** `architect-input.txt` said `RED` but listed no failing jobs, though the brief says it should. `gh run view --log-failed` and the jobs/logs API both returned Forbidden. What finally worked was `check-runs/<job>/annotations`. After that I still had to go back through about 15 runs to find which commit turned it red (36761b04, main lane) and to rule out the yaml commits. That took about 8 of my roughly 15 tool calls.
2. **What I looked up that the brief could have handed me:**
   - the failing test names and the first red commit with its lane (the input should carry them);
   - that the search API is blocked, so only `repos/...` endpoints work;
   - where ADR-361 D9 sits and the decision-paper rules in `docs/supervisor.md` (fine to read, but one line would have pointed there);
   - whether a `Found` should still be filed when CI is red and `batch.txt` stays unchanged. Step 3 doesn't say; I filed it anyway (#1022).
3. **Checks I ran:**
   - Useful: listing the runs, the failed jobs and steps, the annotations of the first red runs, the `git log` of `touch_sets.rs` (it pointed to the fix forward 0b8a3acb), and grepping #928 before the PATCH.
   - Wasted: `--log-failed`, `actions/jobs/<id>/logs` (twice), and `search/issues`. All were Forbidden, about 4 calls.
   - Not checked: whether 0b8a3acb actually turns `develop` green. Its run was still going, and I didn't poll.
4. **The change that would save the most:** the driver should write into `architect-input.txt`, on every `RED`, the failing jobs, the failing test names from the annotations, and the first red commit with its title. Then step 3 is one read instead of a search. Second, `docs/architect.md` should say that CI logs are Forbidden headless: use `check-runs/<job>/annotations`, and only `repos/...` endpoints work, no search. I did not commit either change this run; one of them should be made next run.
