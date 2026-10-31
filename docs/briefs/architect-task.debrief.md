1. **What cost the most.** Tracing finding (c) took about 12 of my roughly 45 tool calls. The first cause I guessed, `x + 1` on an untyped `x`, was wrong; probe c7 (`fn(x) { 5 }`) ruled it out. Only after that did I reach the stored-code frame being popped. Before that, I spent about 5 greps on Part I looking for a "first use" rule for lambdas. The rule turned out to come from #997 alone, not from the specification. Reading `docs/architect.md` also cost calls: it is too big for one read, so I needed several reads and a heading grep to reach *State*.
2. **What the brief could have handed me:**
   - the line where *State* starts in `docs/architect.md`, or the last *State* entry pasted into `architect-input.txt`;
   - that `target/fast/nikaia` predated the commit that fixed b1043, so it needed a rebuild before any probe;
   - the rows from b1043's report as ready probe files with their outputs on the unfixed compiler;
   - that the "lambda typed by its first use" rule lives only in #997 and `check_core_stmt.nika:1830`, with no Part I section.
3. **Checks:**
   - Useful: the rebuild; probes c through c7 and d through d6, each changing one form (c5, c7 and d5 found the causes); re-running #1036's example; making sure no issue was labelled `wip` and reading the other lanes' `batch.txt`; checking that the push reached `origin/develop`.
   - Wasted: the Part I greps (one output was 52 KB); an `ls` of the whole `docs/briefs` directory (hundreds of names, when I needed one file); probe d6, which failed on a quoting mistake of mine inside an f-string.
4. **One change.** `drive.nika` could write the b1043 report's `Found:` rows into `architect-input.txt` as probe files, each with its output on the binary that ran. It could also say whether `target/fast/nikaia` is older than HEAD. That would have saved the rebuild check and about a third of the probing.
