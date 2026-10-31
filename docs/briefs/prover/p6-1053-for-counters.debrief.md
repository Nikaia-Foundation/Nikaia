1. **Most turns:** reading `bounds_walk.nika` to see how facts flow: the loop walks, the `if` join (it forgets every changed name, so a solver-checked turn end could not prove `some`), and `BoundsContext`. Then tracing why a `for`'s collection has no length. That trace led into the checker, which the brief's file list ruled out, so step 1 stayed partial.
2. **Lookups the brief could have handed me:**
   - The walk has no type for a `for`'s iterable: `context.lengths` holds only `.len()` receivers. That one line would have made the brief's file scope visibly too narrow before I started.
   - The AST forms: `Stmt` variants, `Spanned` has more fields than `node` and `span`, `Expr::LitInt { value, negative }`.
   - Nikaia has no `i8` (the soundness rows asked for one), and a `Vec[ref String]` literal does not pass as `ref Vec[String]`.
   - The `.bounds` header written by `nikaia test` has no `- written without their check` suffix, unlike `check --bounds`.
   - p5's `bounds_expectation_guards_a_proof` embeds an `off` report that this step was bound to change.
3. **Checks run:**
   - Useful: probes on the old binary, the first `lower-std` plus `cargo check --features compiler` (caught the `??` lowering at once), one build, the probes again, the `it` filter, the `tests/compiler` run (caught the guard test), the figures, pre-push.
   - Wasted:
     - running `check --bounds` inside `tests/compiler`, which checks the whole project;
     - running the guard test directly with `nikaia run`, where its relative binary path fails;
     - the `bounds::` unit-test filter, which matched 0 tests.
   - Not run: clippy 1.99.1 (disk).
4. **One change:** a brief whose decision assumes a fact the walk must have (here "a collection with a length") should name where that fact comes from, or allow the one checker file that supplies it. Better: have the architect probe it once before writing the scope. That would have turned a partial result into a done one.
