1. **Most turns and time:** build cycles. One foreground cycle went past its 600 s timeout and had to finish in the background. A lowering defect (a `mut Vec[ref Expr]` parameter lowered without a lifetime) cost a whole extra cycle and a rewrite. Before that, three `lower-std` refusals took one round each: `??` mixing a map view with an owned fallback, and a `ref …?` that was not narrowed after `!= null`. Before the context reset, much time also went into working out group ids and the spans where tasks are waited for (`CheckAsyncWait`, `-1 - stmts[0].span.start`).
2. **Lookups the brief could have handed me:**
   - ADR-async-5 D7's exact text, including its stricter NK2107 sentence.
   - That the issue's probe (`async { total += 1 }`) is NK2111 and the intended form is `async fn { … }`.
   - Where the overlap NK2104 check lives (`check_walks.nika`) and the `verdict`/`accounted` API.
   - The NK2101 note's location (`check_takes.nika`, `check_lambdas.nika`).
   - That no existing test shows the `spawn` note. The brief's pointer to `async_parse_detached_runs` led nowhere.
3. **Checks:**
   - Useful: the old-binary runs in a scratch project, a copied subset project with only `async_*`, `task_*`, `spawn_*` and `overlap_*` tests, clippy, `pre-push`, and regenerating the badge from the index.
   - Wasted: a `grep -rln` over `tests/` that timed out, `nikaia run` on test files inside the project (refused), and `nikaia test --filter`, which does not exist even though the brief told me to use it.
4. **The one change that would have saved the most:** a brief fact block with three things:
   - the correct probe form;
   - the current way to run a few `tests/compiler` programs (copy them into a scratch project; there is no `--filter`);
   - the existing tests the new rule will refuse (the three `async_*` tests that print from two tasks), and whether to adapt them.

   Together these would have saved roughly a third of the turns.
