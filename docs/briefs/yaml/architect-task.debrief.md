1. **Biggest cost:** finding out that #1000 had been closed by mistake. It took about 6 calls: issue events, the commit message, and the `fixed-issues.py` regex. After that I re-ran both of y3's examples to make sure the defect was still there. Next came the grammar examples. One failed because I wrote a character class as `"0".."9"`, which is wrong; the right form is `digit`. Another first looked like a new defect but was only my mistake: a lowercase rule skips whitespace, so the `" "` element never matched. A small cost: my first CLAUDE.md edit missed because I guessed the line wrapping without reading the file.
2. **What I looked up that the brief could have handed me:**
   - That y3's issue #1000 was already closed. One line in `architect-input.txt` giving the state of each brief's issue would have caught it.
   - That y16 is still unrun, so `batch.txt` already holds it.
   - Which brief has the newest rules block (y16, not y2b, whose block is followed by an "Earlier attempt" section).
   - That `Found:` (1) parses: its line break is already in the committed file.
   - The character-class syntax for a grammar example.
3. **Checks:**
   - Useful: y3's two examples on develop (both still fail), the `peek`/`not` example plus its variants, which narrowed #1049 to those two built-ins, the closing-keyword regex, and the `gen/` path.
   - Wasted: the `text("-" Gap)` variant (it failed on whitespace skipping, my own mistake), the first `dec[i64]` example, and the failed Edit.
4. **The one change:** have `drive.nika` write the open/closed state of each issue the queued and next briefs name into `architect-input.txt`, with a warning when a brief's issue is already closed. That would have caught #1000 at once. It would also catch any other issue closed by prose before the lane sends a builder to fix something already marked done.
