1. **Time spent.** About a dozen read-only calls went on one question: how does the emitter learn a call's checked result type? I looked in the `collect` turbofish code, the `collected_into` fact, the fact-map list in `check_facts.nika`, the `std_receivers` writer and the #931 diff. That search ended in a stop, not a build. No cargo build or test ran, so none of that time went to builds.

2. **What the brief could have handed me.**
   - The brief's *Locations* named only the emitter site and #931's checker files. It never said how a type reaches the emitter. A line such as "emit reads checker facts by node key; there is no type-at-call fact, so add one in `check_facts.nika`" would have saved the search.
   - The brief's stop clause forbids edits outside `emit_core_calls.nika`. It is also the likeliest outcome, since no fact exists. The brief should have said whether a new fact is allowed.
   - I also had to find where `check_core_method_called` records facts.

3. **Checks.**
   - I ran no tests, no build and no gate, so none were wasted.
   - My reads were all useful. The `git show 5162370e` read was the least necessary, but it confirmed `open_items` covers only open literals.
   - I did not probe the actual lowering (`nikaia lower` on the probe). That would have confirmed the bare `.sum()` quickly, but it was not needed to decide the block.

4. **One change.** When the architect writes a brief for an emitter fix, the *Locations* should include "which fact carries the needed information from checker to emitter, or: none exists, add `X` in `check_facts.nika`". The stop condition's file list should include the checker files the fact needs. That would have turned this into a normal build instead of a block.
