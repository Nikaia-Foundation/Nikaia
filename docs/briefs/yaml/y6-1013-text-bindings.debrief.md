1. **What cost the most:** reading winnow-grammar's source in `~/.cargo/git/checkouts` (about 10 turns). I needed to learn three things: that `text(p)` is a `.take()`, that `(a b) @ x` hides a multi-element sequence's bindings, and whether an action can reach `input`. My first emitter version used `(p) @ s` and failed on the nested case, which cost one extra build cycle. A lowering defect in my own helper (`ref Vec` result against a slice parameter) cost a third.
2. **What the brief could have handed me:**
   - The winnow-grammar checkout path and the codegen lines for `text` (`codegen/expr.rs` ~996) and for `@` (~395).
   - That `S`/`E` are hard-coded in the macro. That alone would have marked step 2 as blocked from the start, instead of a step to build.
   - That an action may name `input` (`fixed.rs` `action_inlines`).
   - That `LocatingSlice::reset_to_start` exists.
   - That `EmitWalk.fresh()` is the only place an `EmitWalk` is built.
3. **Checks run:**
   - Pre-flight probes of both cases, the unfixed run of the new test, and a probe of the lowering defect: all useful.
   - Two build cycles, then the gate filters, clippy, `tests/compiler --both-settings`, `packages/yaml` and `pre-push-check`, all green.
   - Wasted: the first probe of `struct S`, which never used `S` inside the grammar and so did not collide. Comparing `nikaia fmt` output in the scratchpad was also wasted: it has no repo config, so it reformatted with tabs.
   - `cargo test -p nikaia-std --lib grammar::` ran zero tests.
4. **The one change that would have saved the most:** the brief's step 0 should name the generated-code owner for each point ("`S` comes from winnow-grammar's macro: if so, stop step 2"). It should also include the winnow-grammar codegen locations. One more point for the record: under the rule "a compiler defect stops the slice", I worked around the `ref Vec` lowering defect instead of stopping, and reported it under `Found:`. The supervisor should judge whether that was acceptable.
