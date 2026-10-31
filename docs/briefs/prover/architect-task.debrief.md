1. **What cost the most:** tracing why p8 turned four `it` tests red took about 12 of my roughly 30 turns: reading both test sources, then six small probes to find which shapes lose a length. One probe was my own mistake. I swapped the `push` loop for a single `push`, which gives a length of 1, and read the failure as a gap. Chasing a wrong lead about calls inside f-strings then cost about 3 more turns.

2. **What the brief could have handed me:**
   - p8's report named the four tests but not their files or line numbers, and not the one-line cause (`Bytes(v)` loses `v`'s length). I found those with `grep`.
   - That GitHub's search endpoint is blocked by the proxy, and that `--paginate` fails. Only the plain `repos/.../issues` list works.
   - What `<full brief path>` in `common.part` looks like.
   - That `common.part`'s `Attribution:` block is hard-coded to another session's ID, so every brief must replace it. I only noticed after writing p10.

3. **Checks:**
   - Useful: the shape probes on today's binary (repeat, literal argument, `Bytes`, `resize`), the file-clash check against the other lanes' `batch.txt`, the `where.nika` lookups, and reading the p7 and p8 reports.
   - Wasted: searching the specification for a `Vec.filled` constructor, the f-string detour, the blocked search call, and re-reading the whole p9 brief when only its opening check needed fixing.

4. **The one change that would save the most:** in `.claude/agents/builder.md`, require that a report with `Needs: architect decision` on newly refused tests gives each test's file and line plus a probe of a few lines that shows the cause. Every architect run after a `partial` starts by redoing that trace. I should also have made `common.part`'s `Attribution:` block a placeholder that `agent.nika` fills in, at the source. That fix is still open.
