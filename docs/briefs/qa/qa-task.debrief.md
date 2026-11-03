1. **Most turns:** Finding the real failure. `qa-input.txt` was about 17 KB, and nearly all of it was CI setup noise (checkout, git hints, cache config). The actual error, `out of date in assets/badges/: self-hosted.svg`, was one line near the end. It took me three or four greps and cuts to reach it. A smaller cost was the commit script refusing the report because of a stray comma in `Files:`.

2. **Looked up:**
   - The brief points at `.claude/agents/qa.md` and the input paths, but not which file is current. Both `docs/briefs/qa-input.txt` and `docs/briefs/bugs/qa-input.txt` existed, and I had to check timestamps.
   - The required report fields for `scripts/commit.nika` (`Changelog:`, `Attribution:`, `Commit:`, space-separated `Files:`). I had to copy them from another builder's report.
   - Which `nikaia` binary to use. `which nikaia` found nothing, so I used `target/release/nikaia`.

3. **Checks:**
   - Useful: `python3 scripts/badges.py --check`, before and after the regeneration.
   - Partly wasted: my first `scripts/pre-push.nika` run. It checks only staged files, so it reported "badges red" even though the fix was in the working tree.
   - Not run: clippy, since no Rust changed. I also did not rerun pre-push after the commit.
   - I also did not check whether `origin/develop` had the same badge drift.
   - Cost: the first commit attempt was refused because of the comma typo.

4. **One change:** Have the driver put the failing step's error lines at the top of `qa-input.txt`, with the log tail filtered of setup lines. Add a qa-report template (or `commit.nika --template`) that includes the required fields. Either would have saved most of my turns.
