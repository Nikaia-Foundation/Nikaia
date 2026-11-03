1. **What cost the most:** the search for the "existing not built yet path" the brief told me to use. I spent about 8 turns grepping the checker, the emitter and the Part III code table before I was sure it doesn't exist. The longest wait was the whole `it` binary, about 25 minutes. builder-facts says a parser change needs the whole binary, while the brief named only four filters. For a change that only shrank the reserved-word list, the filtered run would probably have been enough.
2. **What the brief could have handed me:**
   - that `async` was already in `RESERVED_C`, which makes half of step 1 a no-op;
   - that the AST is declared in `tools/ast.nika`, not `ast/mod.rs`, so a new node touches about 36 files of exhaustive matches;
   - which NK code, or which helper, the "not-built path" means;
   - a sample `overlap { }` program to copy into a test.
3. **Checks run:**
   - Used: a `--both-settings` bless run on a scratch project, the fails-before check with the old binary, `cargo fmt --check`, clippy, `pre-push`, and the whole `it` binary (green).
   - Partly wasted: the whole `it` run, as above. I also ran clippy while it was going, which risked a relink; it turned out harmless.
   - Not wasted: `lower-std`, which I skipped because no `.nika` changed.
4. **The change that would have saved the most:** a brief pre-flight against the tree. Before ordering an "existing path", the supervisor should grep for it and put the exact function and NK code in the brief, or the reserved NK block if none exists. It should also count the `Expr` match sites first, so a node-adding step is sized as its own brief, not tucked into "S=1, C~3".
