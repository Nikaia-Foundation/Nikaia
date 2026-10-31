1. **Most cost:** reading the prover walk. I needed about 8 reads of `prover_walk.nika`, `leaves.nika` and `bounds_body.nika` to see how `match` arms were walked before: through `prover_blocks_in`, then `prover_nested`, with a blind scope. That is also why `prover_follow_mutables` was dropping what the arms assign. The longest single wait was the gate run in the background, about 15 minutes. Writing the tests took one extra turn: the `let mut w = 0` I first wrote left dead stores that only show as warnings, so I had to rewrite them.
2. **Looked up that the brief could have handed me:**
   - The shapes of `MatchArm` and `MatchPattern`. In particular, a `Range` pattern includes both ends, and an arm body is an `Expr`, not a `Block`.
   - The fact that `prover_an_if` gates three places: the block walk, the mutable follower and the length follower.
   - That `rebind` also adds the name to `locals`.
   - That `assigned_in` and `changes_length_in` only take a `Block`, and that `tally_expr` is private.
   - Where `bind_pattern` lives, as a pattern to copy.
3. **Checks run:**
   - One `lower-std`, one build, one more `lower-std`; that reached the fixpoint and found no refusals.
   - The probe script on the new binary and on `nikaia.before`.
   - One gate run: clippy, the `it` filters plus the guard set, `tests/compiler --both-settings`, and `packages/yaml`.
   - `pre-push-check`.

   One wasted check: `nikaia check tests/X.nika` from inside `tests/compiler` checks the whole project, so it showed another test's parse error. I had to copy the program out to get its `.stderr`. The gate run was not wasted, but it confirmed that the `sequences` guard is no longer red, so the *Red on develop now* entry in `builder-facts.md` is stale.
4. **One change:** in the brief, add a *Locations* line listing the three gates on `prover_an_if`, the `MatchArm`/`MatchPattern` lines in `ast.nika`, and "an arm body is an `Expr`, so add `*_in_expr` variants in `bounds_body.nika`". In `builder.md`, add: "to get a test's `.stderr`, run `nikaia check` on a copy outside the project and fix the path with `sed`."
5. **Process:** the push was rejected because `develop` moved, and the rules forbid me to rebase. 541c387d is waiting for the supervisor.
