1. **What cost the most.** Step 1 (#1239). The brief says it is only a keep-inference fix, but it is a missing form in the lowering: nothing can lower a non-text view result such as `fn hand<T>(x: &T) -> &T`. Proving that took many probes on the old binary, plus reading `keeps.nika`, `lends.nika` and the emitter, before I could call it blocked. Second place: probe 2 did not compile as written. `time::Duration::from_millis` is refused with NK1245 and `.as_millis` with NK1171 (the std names are `3.millis()` and `.in_millis()`), so I spent turns telling a std naming gap apart from the defect itself.

2. **What I looked up that the brief could have handed me.**
   - Where the operator check and the `*` deref live: `check_core_ops_outcome`, the `compared_views` set, `emit_core_binary`.
   - That `time::Duration` has `copies = true` in the ledger.
   - That the lowering has no non-view-of-text result type.
   - That `COMPILES.txt` is written from the specification test's `left:` output.

   One line each would have saved roughly 15 turns.

3. **Checks.**
   - Useful: the old-binary probes, the new test against the unfixed binary (NK1104), the filtered `nikaia test`, the `lower-std` fixpoint, `specification::`, fmt, clippy, pre-push, and the commit script.
   - Partly wasted: the first `specification::` run. It failed as expected because `COMPILES.txt` was stale, and it cost one extra 35-second round.
   - Probably wasted: clippy `--all-targets` over the whole workspace, when only a lowered file had changed.
   - Not done: the cache is 3.4G, above `NIKAIA_CACHE_LIMIT=2G`. The rule only clears it when free disk is under 3 GB, and 3.8G was free, so I left it.

4. **The change that would have saved the most.** Before a step goes into a brief, the supervisor should run each probe on the current binary and check the expected lowering by hand, writing the target Rust signature (e.g. `fn hand<T>(x: &T) -> &T`). Step 1 would then have come back as a decision, not a build order, and probe 2's std names would have been fixed beforehand. Also, my final line said `partial`, but the brief's format allows only `done` or `BLOCKED`. `builder.md` should say what a partial run's final line looks like.
