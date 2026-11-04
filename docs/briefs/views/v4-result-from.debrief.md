1. **What cost the most.** The `it` binary ran three times. In the first run my `grep 'test result'` also matched test names (`result_source::…`), and `head -40` cut off the summary. The second run went to the background at the 10-minute foreground limit and was killed 10 minutes later. Only the third, with a long background timeout, gave the result (2886 passed in about 23 minutes). Before compaction, earlier context was also lost to a stray `cd $S` with `S` unset.
2. **What I had to look up that the brief could have handed me:**
   - **Expected `.stderr` files.** `nikaia test --bless` writes only `.stdout`, and `nikaia check` inside `tests/compiler` checks the whole project. I had to check a copy outside the project with a `tests/` prefix to get the right path in the message.
   - **ADR-views-3 D8 on traits and function types**, to settle what counts as a view without a body and what `from` names on a function type.
   - **The self-hosted badge procedure** in builder-facts.
   - **Text forms:** `String::from` does not exist (write `f"x"`), and `.clone()` on a literal is NK1301.
   - **How result sources reach the ledger:** `views.nika::result_sources` → `ledger_defaults::write_result_sources`.
3. **Checks run:**
   - Useful: `lower-std` with the old binary, then build and the fixpoint; the four probes on the new binary; fails-before checks on the old binary; clippy; `tests/compiler --both-settings`; the full `it` run; pre-push.
   - Wasted:
     - The first two `it` runs.
     - The lib unit test filter `check:: views:: parser::`: it matched 0 tests.
     - The step 2 probes written with `String::from`, which had to be run again.
     - The first `.stderr` generation inside the project, which produced the wrong file.
4. **The one change that would save the most:** builder.md should say how to write an expected `.stderr` (`nikaia check tests/NAME.nika` from a directory outside the project). It should also give the exact command for the full `it` run: `run_in_background`, a timeout over 30 minutes, output to a log, then `grep '^test result:'`. That alone would have saved about 35 minutes and several turns.
