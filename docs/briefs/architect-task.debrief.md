1. **What cost the most.** Reading `docs/architect.md` took two reads, about 33k tokens, and most of that is the *State* history. It took four `gh api` calls to get from "RED 8e67ee37 arm64,bootstrap_compiler" to the failing test and the first red push: runs, then jobs, then annotations, then `git log`. Reading the test's long doc comment in `sequences.rs` to find its ceiling and convention also cost time.
2. **What I had to look up.**
   - The failing test's name, and which commits were in the first red push. The driver could put both into `architect-input.txt`.
   - Where unanswered calls are counted (`check_core_call_on.nika:107`).
   - The builder gate wording in `builder.md`.
   - Whether #1012 had been answered (`QUESTION` file plus the issue).
   - I needed no ADRs or converter facts.
3. **Checks run.**
   - Useful: CI run, job and annotation listing; `git log a5663bf6..75e25ed8` with `--stat`; the #994 probe with `target/fast/nikaia`; the `seq` grep.
   - Wasted:
     - `git pull --rebase` failed because other agents' changes were unstaged. A plain fetch and push worked.
     - The broad `seq` grep returned names like `read_seq`.
     - Listing all `ready` issues was larger than I needed.
   - Not done: I didn't confirm which calls are unanswered (that needs a build), so the fix-forward brief names its candidates as guesses.
4. **Change that would save the most:** on RED, `drive.nika` should write into `architect-input.txt`:
   - the failing test names from the check-run annotations
   - the last green sha
   - `git log --stat <last green>..<first red>`

   That would have saved about six turns. Second: move *State* entries older than a day out of `architect.md`, about 25k tokens per architect run. Third: `architect-task.md` should say "fetch, then push fast-forward; do not `pull --rebase` with others' files unstaged".
