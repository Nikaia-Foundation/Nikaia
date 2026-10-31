1. **What cost the most turns:** working out what stopped the batch. The first `cat` seemed to show a `RED` line after the CUT line, and I still can't say where it came from. `architect-input.txt` holds only the CUT line. Next to it were a `qa-input.txt` from 17:50 naming other commits and a `suite.log` with only the build's first lines. I spent about 3 turns making sure no CI was red. The second cost was deciding whether the brief's step 8 meant I should push while HEAD held untested commits. I chose not to push.
2. **What I looked up:**
   - The open-issue list (all pages, filtered).
   - The previous architect report, to find the queued b14 and b16 briefs and why #1060 has `wip`.
   - The `Seq::flat_map` signature in `std.contracts`, and `ty.nika` `bind`.
   - Whether Part I-III says anything about `flat_map`: it doesn't.
   - The lane's *State* section.

   The input could have handed me the previous batch's queue and the commits not yet pushed. I did not read all of `docs/architect.md` or `docs/priorities.md`; I ordered the batch from the brief's rules.
3. **Checks run:**
   - Useful: the three Found probes re-run on `target/fast`, all still failing; `git log origin/develop..HEAD`; the contract and the `bind` source.
   - Wasted: the `od`/`cat -A` checks on `architect-input.txt`, the stale `qa-input.txt`, and grepping the spec and guide for `flat_map`.
   - Not run: I never checked the current CI status through the API, so "CI is not red" rests on reading the input file, not on CI.
4. **One change that would save the most:** the driver should write a dated `architect-input.txt` with four parts: the stop kind (CUT/RED/END), the commits on HEAD not yet pushed, whether the suite is running, and the previous batch's queue. It should also delete `qa-input.txt` and `suite.log` from earlier runs. The brief's step 8 should say: "do not push while the driver has commits on HEAD it has not pushed yet; commit only."
