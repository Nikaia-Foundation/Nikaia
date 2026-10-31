1. **Most cost: reading.** `docs/architect.md` is about 36k tokens, and its *State* section is a long diary of very long lines. I needed only the cut line, the last report and the next brief. After that, the probing took about 6 turns, and writing the issue took 2 rounds because I wrote expected outputs before running the combined program.

2. **What I looked up that the brief could have handed me:**
   - The format of the input file: what a `CUT` line means and where the full report is.
   - Whether `b1029` had already run. I had to infer it from the missing `.report.md`.
   - Where #1018's fact and its emit site are. Reading the commit diff worked, but a pointer to "the cut brief's commit" would have done it in one step.
   - How the argument path already clones a kept fn (`hands_on_kept`). I found it with a grep.
   - How to see the generated Rust. `nikaia lower` wrote no `.rs` where I expected one.

3. **Checks:**
   - Useful: the probes a, b, c, d, g and the combined `t.nika` (all fail in rustc today as expected, and `c` confirmed the working path), `fc.nika` (shows a field call alone runs), and the #1018 diff.
   - Wasted:
     - The first `d.nika`: `fs[0](1)` is not valid syntax.
     - `e.nika` with field calls in one expression: it led to the NK2605 side issue, which I left unresolved.
     - Two failed `nikaia lower` greps.
     - Re-reading the brief's head and tail after generating it.

4. **Most useful change:** split `docs/architect.md`. Move *State* into its own file that keeps only the last 3 entries, older ones archived. Have the architect task brief say which three files to read: the input line, the cut report, and the queued briefs. That alone would have saved about 30k tokens and several turns.

   **Second change:** add a line to `docs/builder-facts.md` saying how to get the lowered `.rs` of a scratch file, so probes can be traced to a cause without guessing.
