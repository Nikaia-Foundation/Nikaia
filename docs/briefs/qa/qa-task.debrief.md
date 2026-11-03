1. **Time and turns.** The run was short, about 8 tool calls. The most expensive step was the first read of `qa-input.txt`. It is 36KB, mostly CI runner boilerplate, and the one real error sits at line ~170. After that, `nikaia` wasn't on PATH, so `pre-push` and `commit.nika` each failed once before I added `target/release` to PATH.

2. **Lookups the brief could have handed me.**
   - The brief points to `.claude/agents/qa.md` for the rules, and I never opened it. I only listed the agents directory.
   - I didn't know where the `nikaia` binary lives. It is `target/release/nikaia` here, and `docs/builder-facts.md` names `target/fast/nikaia` for tests.
   - I had to find the report format by reading the `commit.nika` header and a builder's report. That covers the `Changelog:` and `Attribution:` fields and the attribution lines.
   - I found the replacement ADR by grepping `docs/history/adr/README.md`. I didn't open the ADR-279 text to confirm the debrief's sentence still fits it.

3. **Checks.**
   - Useful: `check-adr-refs.py` directly, which gave a fast green result.
   - Slightly wasted: the first `pre-push` run. It checks staged content, so it reported the old citation as red because my edit wasn't staged yet. That was my ordering mistake, not a real failure.
   - Not done: I did not rerun `pre-push` after staging. I only confirmed with `git show` and `git grep` that the commit holds the fix.

4. **One change that would have saved the most.** Have the driver put the extracted error lines at the top of `qa-input.txt`, here the "1 citation(s) of an archived ADR" block, and trim the runner preamble. The brief should also say to export `PATH=$PWD/target/release:$PATH` before running `nikaia`.
