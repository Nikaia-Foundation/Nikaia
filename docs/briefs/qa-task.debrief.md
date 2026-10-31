1. **Time and turns:** Finding the actual failure cost the most. `qa-input.txt` said only "pre-push red failed without output", and its 30 KB was mostly passing test lines. I had to rerun each sub-check of `pre-push-check.sh` by hand to see which one failed. The first run hid the cause because I piped it through `tail` and `check-adr-refs.py` was silenced with `>/dev/null`. Locating the ADR index (`docs/specification/adr/README.md`, not `docs/adr*`) took a couple more calls.

2. **What the brief could have handed me:**
   - The path of the ADR index and the row format (number, summary, status, built).
   - Which unpushed commit touched what. Here the cause, 91d5eb6b, was already pushed. The brief's "unpushed commits" framing sent me to look at three unrelated commits.
   - The fact that `commit.nika` needs the built binary at `target/fast/nikaia`, and that the report file already existed.

3. **Checks:**
   - Useful: `pre-push-check.sh`, then its four sub-checks run separately.
   - Wasted: the initial `ls docs/briefs`, which dumped a huge listing, and the `grep` for ADR-384 across the whole repo, which a targeted look at the index would have replaced.
   - I ran no clippy or tests, which was right for a docs-only fix.

4. **Change with the biggest saving:** make the driver's `pre-push` step capture the failing sub-script's stderr and stdout into `qa-input.txt`, instead of "failed without output". Then I would have read the ADR-384 error straight away. Failing that, put in `qa.md` or the brief a line saying that for a pre-push red the first move is to run each line of `pre-push-check.sh` separately, and that the cause may lie in already-pushed commits.
