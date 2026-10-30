1. **What cost the most:** the lowering cache. After my first edit, `prover_bodies` and `prover_passes` were lowered as `async` and the build failed. A touch and a cache-salt run did not refresh the stale `prover_passes.rs`; only declaring both functions `sync` fixed it. That took about 8 turns and two extra `lower-std` runs. Second came step 2: three probes to learn that `peek` cannot be bound and that a published field may not hold both kinds of text, both after the step was already written.
2. **What the brief could have handed me:**
   - That the prover never walks grammar actions (`hold_unreached` in `prover_passes.nika` says so outright), and where to add the walk: `prover_bodies`, plus `prover_if` as the pattern for a guard becoming a fact.
   - ADR-282 D22 (no published field of both kinds), which contradicts the `Scalar.value` it asked for.
   - The Part II 10 element table, and that winnow-grammar refuses binding `peek`, `not` and `=>` (`winnow-grammar-model/src/parser.rs`).
   - That `nikaia fmt` does not cover `tools/*.nika`, and that `commit.nika` takes the attribution from the brief's own report name.
3. **Checks:**
   - Useful: the scratch probes with `check --asserts`, `lower-std`, `tests/compiler --both-settings` (it caught my `.stderr` column), the `it` guard set plus a `grammar` filter, clippy, pre-push-check, and the yaml and json package tests.
   - Wasted: `cargo test --lib prove` (0 tests), the `NIKAIA_LOWER_CACHE_SALT` run, and the step-2 yaml check (refused before I knew about D22).
   - Weak: I rewrote the `.stderr` from captured output without a second run, and the refusal test ran on the unfixed compiler only as a probe of the same form, not as the file itself.
4. **One change that would have saved the most:** a pre-flight line in the brief, *"probe every grammar form the step needs (`x:peek(text(..))`, a mixed `pub` field) before writing it"*, and an architect check of the brief's design against ADR-282 D22. With both, step 2's blockers would have shown up in the first five turns instead of after the code was written.
