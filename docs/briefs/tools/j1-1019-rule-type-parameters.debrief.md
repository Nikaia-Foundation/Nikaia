1. **Most turns and time:** the rustc E0277 from the lowered `grammar_types.rs`: one `&` too many on a kept-buffer local passed to `contains_key`. I spent several probes (p2-p5) trying to reproduce it and none did, then redesigned `GrammarTypesCall` to carry `Symbol`s. Strictly, that was a lowering defect, and the rules say it should have stopped the slice, not been designed around. The reviewer should weigh that. In wall time, the `it` binary (about 26 min) and the test project run cost the most.
2. **What I had to look up:**
   - where the post-parse expansion passes sit in `parse()`, with `one_variant.rs` as the model for walking and rewriting expressions;
   - how a `Finding`/`refuse_finding` is built, and the existing NK1230 wording;
   - the copy naming (`check_specialised`);
   - the Rust shapes of `Expr` and `Type`, and `Pattern::Ref` vs `Pattern::Call`;
   - that a new tools `.nika` needs a row in `scripts/self_hosting.py`;
   - that adding a field to `GrammarRule` forces an edit to `node_numbers.nika`;
   - Nikaia slips in tools code: `grammar` is a reserved word, `BTreeMap()` not `::new()`, `high?.clone()`, `f"{...}"` for a resolved symbol.
3. **Checks run:**
   - fast build and `lower-std` fixpoint, `cargo fmt`, clippy, `pre-push-check.sh`;
   - p1 and both new tests against both binaries;
   - the whole `tests/compiler` project at `--both-settings`;
   - the whole `it` binary;
   - `--lib parser::`.

   Wasted: `--lib parser::`, which ran 0 tests. The full `tests/compiler` run was also mostly waste, because `nikaia test` has no filter for running only the `grammar_*` tests. Some of the p2-p5 probes were waste too.
4. **One change:** the brief should list the AST's "must follow" files (`node_numbers.nika`, the `self_hosting.py` row) and the expansion hook with its model (`one_variant::expand`). Separately, `nikaia test` should take a name filter, so "run the grammar_* tests" costs minutes, not a full project run.
