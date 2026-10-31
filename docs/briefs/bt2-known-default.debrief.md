1. **Turns and time:** The run was short, about 5 tool calls. Most of it went on finding out that the pre-flight could not pass. I looked for commit 4de39626, then for the test file, then fetched `origin/develop`. None of the three exists.

2. **What I looked up that the brief could have given me:**
   - The brief says b9 is fixed in 4de39626, but that commit was never in the checkout I was given. The brief should say whether the fix is on `develop`, and give the `develop` sha it was written against.
   - I had to find out for myself that the `Ledger` struct and the `config` field are in `ty.nika`. The brief already gave those line numbers, so that look-up was avoidable. I did not need the converter or any ADR.

3. **Checks:**
   - Useful: `git status`, `git log`, `git cat-file -t 4de39626` and the `ls` for the test file. These settled the question.
   - Mostly wasted: the first command also read `comptime_run.rs`, `build_eval.nika` and the binary's timestamp, before I knew the pre-flight would fail. I should have run the pre-flight first.
   - Not done, and not the brief's fault: I did not check whether the `4de39626` hash was a typo for a nearby commit. I only grepped the log for "coalesce" and "1055", which found nothing but the brief commits.
   - Not run, because nothing was built: the build, `lower-std`, the tests and the cleanup.

4. **One change:** Add a one-line pre-flight to the top of every brief that depends on another lane's fix. For example: `git cat-file -e <sha> && test -f <test>` must pass, or the run reports `blocked` immediately with no other reads. The driver could also check this before it launches the agent. That would have saved the run and the cost of a launch.
