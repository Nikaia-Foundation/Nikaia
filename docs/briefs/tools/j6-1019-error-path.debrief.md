1. **What cost the most:** reading the `winnow-grammar` source to see how frames are made: `push_rule`, the live `rules` stack, `.item(n)` in `rt.rs`, the code for rule calls, `extern` calls and template inlining. That took about 12 of my ~20 turns. The key fact, that `__nikaia_list` gets no frame, only showed up once I ran the probe and read the generated `probe.rs`. My first probe run failed because I cut the brief's lines one off and kept the code fence: one wasted turn.

2. **What the brief could have handed me:**
   - Template rules (`rule X<T>(item: Rule<T>)`) are inlined by the macro and push no frame.
   - `item N` frames are pushed only for Backtrack errors, never for Cut (`rt.rs` ~1005/1082/1248).
   - Every `members[S]` failure is a Cut.

   The brief's suggested route ("push a marked segment frame `[1]`") rests on frames that can carry a run-time index, and there are none. The brief also said `in item N` "are not the list index" without saying they vanish on Cut.

3. **Checks:**
   - I ran the old binary on the probe, read the generated Rust, and ran the disk check.
   - None of them was wasted apart from the fence slip.
   - I ran no build, `lower-std` or tests, which was right because nothing could be built.

4. **One change:** a brief that says "one way needs no backend change" should rest on a probe of that way by the architect. Here that would have meant the existing probe's generated Rust plus one list-of-objects case with a cut. Whether a run-time index survives a Cut was the whole question, and a one-line note under *Locations* would have settled it, or turned the brief into a backend decision before any builder started.
