1. **Turns and time.** Finding the driver's directory cost the most. The brief said `docs/briefs/qa-input.txt` or `docs/briefs/bugs/qa-input.txt`, and neither existed. I had to `find` the file, which turned up under `docs/briefs/yaml/`. My first `ls` of `docs/briefs` also dumped hundreds of lines. The fix itself was a one-line index row.

2. **What I looked up that the brief could have handed me.**
   - The lane directory. The task text should have named it, as the instructions say it would.
   - That `pre-push` failing "without output" means one of four sub-checks in `scripts/pre-push-check.sh`. I only learned it was `check-adr-refs.py` by running them one by one.
   - The ADR index format and where it lives (`docs/specification/adr/README.md`).
   - That `nikaia` isn't on PATH. I had to use `target/fast/nikaia`.
   - The commit script's report fields. I read the header of `scripts/commit.nika` to get them.

3. **Checks run and wasted.**
   - Useful: `pre-push-check.sh` alone, then its sub-checks separately, then `pre-push-check.sh` again after the fix.
   - Wasted: my first `pre-push-check.sh | tail` hid the exit code, so I ran it twice.
   - Wasted: two `grep`/`ls` guesses for an `adr-384` index row in the wrong files.
   - Wasted: I did not need `git diff --stat` or the ADR-384 header reading.
   - The report's `Cause:` line is slightly muddled: 91d5eb6b was not in the unpushed list. The unpushed commit was 0c3303d1, so the broken state was probably already on `develop`. I should have checked that.

4. **One change.** The driver should write its directory into the task text. For example, "Input: docs/briefs/yaml/qa-input.txt". Failing that, `qa.md` could say to run `ls docs/briefs/*/qa-input.txt`. A second fix: `pre-push-check.sh` should print which sub-check failed, so "failed without output" is no longer opaque.
