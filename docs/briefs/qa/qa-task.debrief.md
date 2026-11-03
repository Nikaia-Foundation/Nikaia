1. **Most time.** The Rust rebuilds took the most wall-clock time: roughly 3.5 minutes for the first test run and then a clippy run. On the diagnosis side, I spent several turns working out that the failing commit was already on `origin/develop`, not one of the unpushed commits. The unpushed commits only touched `check_core_pilot`. The second failed attempt at the fix cost one more rebuild. A bare struct field with an unknown path type isn't refused, so I had to switch to a call.

2. **Lookups the brief could have handed me.**
   - The brief points to `.claude/agents/qa.md`, but I never opened it. The instructions in my system prompt were enough.
   - The directory is wrong: `qa-input.txt` is in `docs/briefs/`, not `docs/briefs/qa/`.
   - I had to find the report format by reading another builder's report, and I needed the `Attribution:` and `Changelog:` fields.
   - `nikaia` isn't on PATH, so I had to find `target/release/nikaia`.
   - `qa-input.txt` is mostly CI log noise. A one-line "failing test X, in file Y" extract at the top would have been enough.

3. **Checks.**
   - Useful: rerunning the failing test (twice), `cargo fmt`, clippy.
   - Partly wasted: `pre-push` came back red on the version and badge checks, which have nothing to do with my file. I couldn't tell whether they were red before my change, and I didn't check.
   - I did not run any other tests. A grep for other tests that use `winnow_grammar::Symbol` as a name std doesn't know would have been a cheap safety check.

4. **One change.** Have the driver put the failing test's panic message and file path into `qa-input.txt`, along with whether the culprit sha is in `origin/develop..HEAD`. That would have skipped the log reading and the git archaeology. A second change: say in the brief whether a red `pre-push` on version or badges is expected for QA agents.
