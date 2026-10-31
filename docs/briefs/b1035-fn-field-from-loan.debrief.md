1. **Most time:** the two rebuild cycles for the unfixed run. To show the test fails without the fix, I removed the fix, ran `lower-std`, built, probed, then put it back, ran `lower-std` and built again. After that came the long `tests/compiler --both-settings` run, and sorting out its 11 failures, which all came from other people's uncommitted work (#999 `text_tiers.nika`, cargo file-lock noise in one test's output). Two small slips cost a turn each:
   - I backgrounded a whole `S=... && (clippy) &` chain, so `$S` was empty afterwards. `builder-facts.md` already warns about exactly this.
   - A `sed` command broke because `#` was both the delimiter and in the text.

2. **What I had to look up:** whether to adopt the fix and test an earlier run of this brief had left uncommitted. The brief's "every file modified at your start is not yours" rule, read literally, forbade it. A line like "an earlier run may have left your own change in the tree: check the diff and adopt it" would have settled it. I also had to check whether a binary without the fix existed (there was no `target/release`). Locations, the cause and the test text were all in the brief.

3. **Checks:**
   - Useful: the probe with the fixed binary, the `nikaia lower` check that the clone comes out, the unfixed rebuild, the `lower-std` fixpoint, the guard set, clippy and `pre-push-check`.
   - Mostly wasted: `tests/compiler --both-settings`. Its red came from a shared dirty tree, so it could not judge my change. A run over the NK1131 tests alone (`where.nika -- NK1131`) would have answered the question in a fraction of the time.

4. **Change that would save the most:** have the brief name a binary built without the fix (or keep `target/fast/nikaia.prev` from the last build), so the unfixed run costs no rebuild. Second: in builder.md, say that when `tests/compiler` failures sit in files of another agent's uncommitted work, the builder escalates them under `Needs:` and does not treat them as its own gate red.
