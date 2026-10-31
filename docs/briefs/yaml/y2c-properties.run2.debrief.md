1. **Biggest cost:** I ran the fmt check only at the end. Applying the draft, writing the test, blessing it and fixing the comments all came before the `fmt::every_nika_file` scan (about 50 s), which then failed on #1015. Running `nikaia fmt` on a copy of the draft right after applying it would have shown the blocker in the first five turns. The fast build at pre-flight was needed: the binary was older than 262d5fa6, and I could only tell from the rebuild because commit times are rebase times.
2. **What I looked up that the brief could have given:**
   - Whether #1015 was still open. The brief listed the open defects #1038 and #1037 but not #1015, though builder-facts mentions it.
   - What fmt does with a hand-broken rule line. It moves the continuation to one tab, which ruled out the only layout fix.
   - The pattern of the earlier package commits: they include `nikaia.contracts`, `nikaia.derived`, `nikaia.lock` and `nikaia.proofs` along with the source.
3. **Checks run:**
   - Useful: the two pre-flight guard tests (via `nikaia run` copies), `nikaia check`, three package test runs, the fmt scan, `pre-push-check`, and four small fmt probes that isolated the trigger.
   - Wasted: one package run that only printed error messages for inspection. The message edits and the final package run after comment-only changes were also moot once fmt blocked.
   - Not needed: the #1037 test on its own, since the package tests cover it.
4. **The change that would have saved the most:** add to the brief's pre-flight, and to builder.md's step for starting from a draft: "after applying a draft, run `nikaia fmt` on a scratch copy of every changed `.nika` before writing anything". The architect checked `check` and `test` on the applied draft but not fmt. A pre-flight that runs the whole package gate (including fmt) on the bare draft would have stopped this run, or the brief itself, at once.
