1. **Most cost:** the full `tests/compiler --both-settings` run. I started it in the background and ended my turn to wait. The session was torn down before it finished, so after ~407 programs I had a partial log and no failure details. Next came three scratch-project runs chasing six spawn-by-path failures. These could never pass outside the repo layout, because the tests look for `target/fast/nikaia` relative to the project.

2. **What I looked up:**
   - The report field list in `docs/supervisor.md`. The brief only said "Fields of …".
   - Where an arm decides `ref` versus by value: `check_core_flow_arm` and `parts_lent`, and where a name gets handed over (`hands_over_name`).
   - The #1158 probe itself. The brief said "verbatim in #1158" but didn't include it, so I wrote my own.
   - Where tests go: the brief named `tests/compiler/NAME.nika`, but tests live in `tests/compiler/tests/`.

3. **Checks:**
   - Useful: the old/new binary probes (p, q, r), one build cycle with the fixpoint check, the `it` module filters (339 tests), clippy, `pre-push`.
   - Wasted: the scratch-project re-runs of the six path-dependent tests, a `sleep 1` call, and an empty read of the output file.
   - Half-wasted: the interrupted full `tests/compiler` run. The rules say the driver runs the suite, so I should not have started it.

4. **Most useful change:** paste the issue's probe into the brief and give the exact test path (`tests/compiler/tests/`). Also add a line to builder.md: "never run all of `tests/compiler` for a checker change; probe the shapes and run the `it` filters; tests that spawn `target/fast/nikaia` only run inside `tests/compiler`." Together these would have saved about a third of the run.
