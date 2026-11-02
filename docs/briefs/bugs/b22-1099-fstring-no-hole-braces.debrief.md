1. **Time and turns.** Nothing was expensive. The slow part was the verification trap. I copied `target/fast/nikaia` as the "old" binary, but it had already been built with run 1's uncommitted edit. So my first fails-before run passed, and I only found out after two extra probes. The one cost of that was finding a truly unfixed binary (`/home/user/nikaia-suite/target/fast/nikaia`). The 4-minute build cycle was also on the critical path.

2. **Lookups.** The brief could have told me three things:
   - The tree's binary already contained run 1's edit, so a truly unfixed binary had to come from somewhere else.
   - The text parts of an f-string are raw source text, so quotes and backslashes are already escaped. That is why no second escaper is needed. I had to read `format_of` and the `LitString` arm to confirm it.
   - My first guess at the test's expected length was wrong (I wrote 16, the right figure is 14). A line in the brief working it out would have saved a pass.

3. **Checks.**
   - Useful: `lower-std` and its fixpoint, the one-program test on the fixed binary, the probe on the unfixed binary, and `pre-push.nika`.
   - Wasted: the "OLD" test run with my copied binary. It proved nothing.
   - Skipped: clippy and the Rust unit tests. I changed no Rust by hand, only the lowered twin. I did not grep the `it` tests for the old lowering text (`.to_string()` of a hole-less f-string).
   - The `it` tests could still be red if one asserts the doubled-brace text. I didn't check.

4. **One change.**
   - Add to the facts: "When a brief says a previous run left an uncommitted edit, build the unfixed binary first by running `git stash` on your own files only, or use the nikaia-suite binary. `target/fast/nikaia` may already contain the edit." That would have saved the confused probe.
   - `builder.md` should say which binary is the canonical unfixed one when the tree is dirty.
