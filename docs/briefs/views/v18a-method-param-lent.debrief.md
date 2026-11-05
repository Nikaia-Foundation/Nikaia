1. **Most turns: a stale binary and the design of `by_value`.** `target/fast/nikaia` had been built from the first run's patched tree, so both new tests looked already fixed against "HEAD". I had to rebuild from the clean tree and keep a copy as `old/nikaia` before the fails-before check meant anything. After that, most of the time went into working out rules (a) and (b): where in the inference order they could be computed, why the views column couldn't be used (`lent_text` reads `lends_in_own`), and how to tell a method named as a value from a method that is called.

2. **What I looked up that the brief could have handed me:**
   - the inference order in `ledger_items.nika` (keeps → … → tether);
   - that `stores` is an unserialized FnContract column, the precedent for a new one;
   - the AST shapes `Expr::Path(Vec[Symbol], NodeId)`, `Item::Impl { target, methods }` and the method key format;
   - `visited_in_block` as the expression walker;
   - where "tethered" lives (`ledger.types[..].tethered`);
   - that free functions passed as values already get an adapter in `emit_core_args.nika`.

3. **Checks:**
   - **Useful:** `lower-std` (it caught a `ref` vs owned assignment), `cargo check` of `nikaia-std --features compiler`, the build (it showed exactly the patch's three Rust `&` errors), the two new tests with `--both-settings`, the five `tests/compiler` filters (214 ok), the `it` filters (98 ok), clippy and `pre-push`.
   - **Wasted:** `nikaia run examples/1brc.nika`, which failed for the missing `measurements.txt` (the `check` I ran next was enough). I also ran `pre-push` before anything was staged, so it checked nothing.
   - **Gaps:**
     - I judged the last fixpoint only by an unchanged `git diff --stat` summary, not a byte comparison.
     - Clippy ran with `--profile fast`, not the default profile the rules name.
     - The fails-before results came from before the compaction and were not run again.

4. **The change that would have saved the most:** a one-line note in the brief: "`target/fast/nikaia` may be built from a patched tree; rebuild from HEAD and copy it aside before the fails-before check." Close behind: a *Locations* line that names the `stores` column as the model and gives the inference order with line numbers.
