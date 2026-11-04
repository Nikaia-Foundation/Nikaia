1. **Most turns:** hunting for a cause that probably isn't in the unpushed commits. The test passed locally and on develop. I spent about a dozen calls reading the grammar-check chain (`check_core_a_block_over_a_grammar`, `run_grammar`, `Workshop::run`) to explain an empty findings list, and never found how it could happen. The CI-log fetch that showed `[]` came late.

2. **Lookups the brief could have handed me:**
   - The brief points at `docs/briefs/qa-input.txt`, but this lane's input is `docs/briefs/async/qa-input.txt`. I only found it by listing directories.
   - The input held only the tail of `suite.log`, so I had to run `gh run view --log-failed` to see the actual panic.
   - The `.claude/agents/qa.md` pointer was fine.
   - I found the test file, the check path and the shared workshop by grep.

3. **Checks:**
   - Useful: the single failing test (passed), the `dsl_grammar` module on a cold cache (passed), the arm64 history on the two earlier develop runs, and the diff review.
   - Wasted: the cold-cache run of `dsl grammar` filtered across the whole `it` binary. It filled the disk and gave unrelated failures, and I then had to clean up the scratch cache. I also read `parser/mod.rs` and `check_core_expr` diffs that had nothing to do with the failing test.
   - I ran no fmt, clippy or pre-push, because nothing changed.

4. **One change:** have the driver put the failing test's panic output, not just the last 200 lines, in `qa-input.txt`. Add a rule to `qa.md`: if a failing test passes locally and on the last develop CI, and the diff doesn't touch its path, report it as a flake after at most 10 turns. Don't run cold-cache runs on a disk that is almost full.
