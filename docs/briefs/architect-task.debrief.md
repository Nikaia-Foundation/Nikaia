1. **Most cost:** reading `docs/architect.md`. Its *State* section is about 110 lines of history, so the file went over the read cap and took two reads, which was the largest share of my context. The stop line in `architect-input.txt` was cut short (`...`), so I also had to open b995's report to get the full `Found:` text.
2. **What I looked up that the brief could have handed me:**
   - the full `Found:`/`Deviation:` text of the report that caused the cut;
   - whether k3a2 actually uses the failing form: I grepped `comptime_run.rs` and found the `let known = |name| ...` at :393;
   - where lambdas are lowered (`emit_core_closure`, `emit_core_flow.nika:229`) and where the NK2302 message is built (`views.nika` `rsrc_report`);
   - the text of `common.part` and an earlier bug brief to copy the layout from.

   No ADR lookups were needed. ADR-293 D15 came from b995's own report.
3. **Checks:**
   - Useful: the three probes (a, b, c) on `target/fast`, which confirmed both finds and the map-read tail row; and the k3a2 grep.
   - Partly wasted: printing about 60 lines of `emit_core_closure` and the `views.nika` code around the message when a few lines would have done; the CI run list, which only said "running" and changed nothing I decided.
4. **One change that would save the most:** have `drive.nika` write the cut report's full `Found:` and `Deviation:` lines into `architect-input.txt` without cutting them off. Separately, move *State* entries older than about three cycles out of `docs/architect.md` into an archive file, so a fresh architect reads the charter plus a short current state in one read.
