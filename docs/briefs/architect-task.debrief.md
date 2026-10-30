1. **Most cost.** Reading `docs/architect.md` cost the most. It is about 30k tokens, and roughly 110 lines of that are the *State* history, of which I needed only the latest entry. Reading `drive.nika` to find out why `Pushed: rejected` was not rewritten came second.
2. **What I looked up that could have been handed to me:**
   - Whether b998's commit was on `origin/develop`, which took a fetch and a log.
   - Where ADR-282 lives. My first grep used the wrong path, so it took two calls.
   - The `<full brief path>` placeholder in `common.part`.
   - b998's brief, used as a template for the new one.
   - `where.nika` for `method_kinds` and `bind_a_payload`.

   None of this concerned the converter.
3. **Checks:**
   - **Useful:** the git log showing f2599c1c is `develop`'s tip, the two #1005 probes, and the read of `drive_rewrite`/`drive_commit` that found the cause.
   - **Wasted:** the first ADR grep and the `cost.txt` tail.
   - **Weak:** `target/fast/nikaia` was built at 19:17, before f2599c1c (19:29). So my probes ran on a binary without b998's fix. On the second row I relied on the builder's word that it still fails.
   - **Not checked:** I hid the push output (`-q | tail`) and never confirmed that `origin/develop` reached 84290d72. I also did not test that the hand check in the drive brief's step 2 can be done as written. That breaks the rule to run every test row a brief prescribes.
4. **The change that would save most:** when the stop line is `Pushed: rejected`, `drive.nika` should write into `architect-input.txt` whether the report's `Commit:` subject is on `origin/develop`, with its sha. It should also note how old `target/fast` is compared with `develop`'s tip. A further saving would be moving *State* history older than two entries out of `architect.md` into a separate log, which would cut the charter read by about two thirds.
