1. **Most turns:** finding where the failure was. The brief pointed at `docs/briefs/qa-input.txt`, but this lane's input was in `docs/briefs/async/`. That input held only the tail of `suite.log`, which showed no panic. I had to pull the CI log with `gh run view --log-failed` to find the failing test. The cause then took one regenerate-and-diff of `EXPECTED.txt`.

2. **Lookups the brief could have handed me:**
   - The correct driver directory, `docs/briefs/async/`.
   - The name of the failing test.
   - The regenerate command for `EXPECTED.txt`. It is in the test's assertion message, but I only saw it after fetching the CI log.
   - What NK1181 means. I checked it against sibling block #105 to confirm the change was meant.

3. **Checks:**
   - Useful: regenerating the specification report and diffing it against the baseline, the 6 specification tests, and pre-push.
   - Wasted: the first `docs/briefs/` file listing and the `nikaia` command lookup. The binary was not on PATH. I also grepped `parser/mod.rs` for NK1181, and that returned a large output I did not need.
   - Skipped: clippy, because no Rust changed.

4. **One change:** have the driver put the failing test names and their panic text in `qa-input.txt`. It should extract them from the log with `grep -A15 "---- .* stdout ----"` instead of passing the last 200 lines. Also make the brief name the lane's input directory. A short rule in `qa.md` would help too: for a drifted `EXPECTED.txt`, regenerate it, diff it, and check the diff against the unpushed briefs.
