1. **What cost the most.** Probing #1016's leftovers took about 10 of my roughly 30 calls. I sorted b4's deviation (nullable) and its Found (typed `let`, function type) into four separate causes, one probe at a time. On top of that, `architect.md` came to about 38k tokens, so it was read in pages. I also read two files' output as one stream at first, and briefly took a stray line for a `RED`.

2. **What the brief could have handed me.**
   - That b4's commit was already pushed (6832794b). Its report said `Pushed: rejected`, so I checked origin myself.
   - That b5 and b6 were already written and queued but never run.
   - How to see the lowered Rust (`nikaia lower F.nika` writes `F.rs`). I grepped `builder-facts.md` for it.
   - The emit locations for named lifetimes and the nullable `.as_ref()`. These came from greps, since `where.nika` only finds definitions by name.

3. **Checks.**
   - Useful: n1–n9 and t7 (each separated one cause), the b5 and b6 probes re-run, and the lowered signatures.
   - Wasted:
     - Compiling the hand-edited `.rs` with plain `rustc`. It cannot link `nikaia_std`, and `builder-facts.md` says so.
     - The first version of n1 (`pick(ref xs)` hit NK1137, a probe-writing mistake).
     - n7b (the `String` form ran into the separate NK1261 / `ref String` matter).
     - The issue-list dump, of which I used only the `ready` rows.

4. **The change that would save the most.** The driver could put two things into `architect-input.txt` on a `CUT`:
   - whether the cut brief's commit is on `origin/develop`;
   - which briefs in `batch.txt` have not run yet.

   `architect.md` could also add a line under *Briefs and reports*: "re-probe a partial report's Deviation before planning its step, since a later commit may already cover it". That would have caught at once that the nullable case mostly runs today.
