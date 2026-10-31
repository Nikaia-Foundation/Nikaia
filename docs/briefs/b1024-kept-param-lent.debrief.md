1. **What cost the most.** About eight turns went on probes that wouldn't parse:
   - The brief's sample wrote `mut self`, which doesn't parse; it has to be `ref mut self`.
   - `spawn` took three tries before I reached `spawn fn { … }`.
   - `nikaia check` run inside `tests/compiler` checks the whole project, not the one file.

   Finding why a parameter differed from a local took another six turns of reading (`binding`, `takes_away`, `check_function_frame`). The answer: a plain `String` parameter is bound as a view only when it is lent.

2. **What the brief could have handed me.** Three things:
   - A probe that compiles as written.
   - The fact that a lent `String` parameter is a view type (`check_function_frame`, `check_function_lends`), while `spawn` makes it owned through `keeps.nika`.
   - That the kept-lambda entry point is `takes_of_a_lambda` in `check_lambdas.nika`, not the `NK2101` reporting in `check_takes.nika`.

   No ADR or converter lookup was needed.

3. **Checks.** I ran:
   - one build cycle, which reached a fixpoint at once;
   - five probes on the scratch binary;
   - one gate run in the background: `tests/compiler` at both settings, the guard set and clippy, all green;
   - `cargo fmt` and the pre-push check.

   Wasted: the failed `spawn` probes and one extra `cargo build` to confirm the fixpoint. I didn't stash the fix to re-run the test on the old compiler; the before-fix evidence is a probe of the same program on the old binary.

4. **One change.** Probes in a brief should be run on the current binary before the brief goes out, with a note on which pass decides the behaviour (here: the keeps walk versus the checker). That would also have shown up front that the case with no later use is outside the brief's edit set, and the brief could have said whether to fix `keeps.nika` too.
