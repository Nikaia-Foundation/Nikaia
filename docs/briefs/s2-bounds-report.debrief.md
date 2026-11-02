1. **Time and turns.** Three things cost the most:
   - Each `lower-std` round was slow.
   - The `cargo build` ran about 4 minutes.
   - The shared tree was broken by another agent's uncommitted `threads.nika` and `grammar_run.nika` after my first successful build. That blocked the rebuild, clippy, the `it` modules and the `lower-std --bounds` before/after, so the run ended partial. The session also died once mid-`lower-std`, and I redid that step.

2. **Lookups the brief could have handed me.**
   - The `Stmt`, `Expr::Binary` and `Item` shapes, and what `check_visit_stmt` returns. It returns a flat preorder list, so the holes-after-each-node order needs a separate roots function, which I wrote as `bounds_report_roots`.
   - The syntax gaps I hit one error at a time:
     - `grammar` is a reserved word.
     - `.clone()` on a text literal is NK1301.
     - `use std::i64` is NK1156, and the way to widen a `u32` is `span.start as i64` (seen in `bounds_walk.nika`).
   - Where `check --bounds` and the probe `s2_walk.nika` live. I never found the probe and wrote the file from scratch.

3. **Checks.**
   - Run and useful: `lower-std` (clean and fixpoint), and old-vs-new `check --bounds` on the 15 `.bounds` programs, which were identical.
   - Wasted: the first sample, `examples/*` and `tests/compiler`, which was mostly refusals, and one `lower-std --bounds` pair whose output was an error from the other agent's file.
   - Not run: the `it` tests, clippy, the lib tests.

4. **One change.** Give the brief a "tree health" pre-flight: run `cargo check -p nikaia-std --features compiler` first and note any other agent's modified `.nika` files. Also tell builders to build before touching any shared file. A second change would be to hand the builder a stash-free way to build against a clean checkout of the touched files.
