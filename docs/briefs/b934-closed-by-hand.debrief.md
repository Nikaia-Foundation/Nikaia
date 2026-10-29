1. **Most turns: finding the mechanism the fix hangs on.** The brief named where NK2605 is raised, but not three other things I needed:
   - that `close` is recorded as a hand-over with `to == "closed by \`close\`"` (`check_core_call_on.nika:765`);
   - where a block ends (`check_core_block`, `check_core_stmt.nika:1463`);
   - that `CheckState` is built in Rust (`check/core.rs` `hollow()`), so a new field needs an edit there too.

   On top of that, paths: `branch`, `ways_out` (returns only), and `break`/`continue` going through `check_core_*_jump`. That was about 12 grep/sed rounds.
2. **What the brief could have handed me:**
   - `nikaia test` takes no name filter. Its *Done when* says "filtered to their names", so I had to read `--help` and build a two-test scratch project.
   - The `.stderr` format, including the summary line.
   - The real stdout. The brief's expected text was wrong: the runtime wraps the error in ``cleaning up a `Flaky` failed:``, and the `cleanup_points` test at :210 already says so.
   - That `binding_id` is private to `check_takes.nika`.
3. **Checks.**
   - Useful: the unfixed run of both new tests; five probes with `nikaia check`; one build cycle plus the fixpoint check (one md5 of the lowered diff); `it` and the guard set beside clippy; `check::` unit tests; the full `tests/compiler` gate; `pre-push-check`.
   - Wasted: `nikaia check` on the two test files inside `tests/compiler`. It checks the whole project, so it showed an unrelated test's parse error. Also the `--help` rounds.
   - Unclear value: the `check::` unit filter matched 0 tests.
   - I wrote the edits as Python replace scripts: fast, but outside the "tools in Nikaia" rule.
4. **One change that would have saved the most:** a *Mechanism* line in a checker brief, giving the state record the fix reads or writes and its file:line (here the "closed by \`close\`" hand-over, `branch`, `ways_out`, `check_core_block`, `hollow()`). Second: add to builder.md how to run single `tests/compiler` programs, i.e. a scratch project with `src/` copied and the tests in `tests/`.
