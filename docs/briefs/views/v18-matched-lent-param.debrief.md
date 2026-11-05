1. **What cost the most.** The keeps rule itself was quick (`Expr::Match` in `keeps.nika`: the names an arm binds now count as parts of the parameter). What followed was not. Once matched parameters became lent, the fixpoint lowered `lowered/` into Rust that does not compile: 19 rustc errors across 42 changed files, in three separate classes:
   - **`ref` in a pattern over a lent subject:** `Some(ref x)` against `&Option`. The cause is `check_core_flow.nika` `by_reference`.
   - **A lent `&Op` handed to a method's by-value parameter.** `lends_with` never lends a method parameter, but `kept_by` takes the method's column as "not kept". This defect was already there before my change (probe `p5` fails on the old binary).
   - **A move out of a lent parameter** in `prove_run.rs`.
   
   Tracing each through the checker and emitter took most of the run.
2. **What the brief could have handed me:**
   - the brief's own *Stop and report if* risk, spelled out: that the fixpoint rewrites about 40 tools signatures, and that the checker's view of a matched subject (`lent_bindings`, `by_reference`) and method parameters (`lends_with`'s receiver rule) were built assuming the parameter is kept;
   - for (4): that `html::escape` is the only `-> ref String` in std and returns a `Cow` in Rust, and that the no-`&` decision is in `the_compiler_writes_the_reference` (`check_references.nika`).
3. **Checks.** Useful: the probes on the old binary (p1–p5) and the first build cycle, which showed the cascade. Partly wasted: the fixpoint build went off with no estimate of how many signatures would flip. A dry `lower-std` diff count first (`git diff --stat lowered/`, which said 42 files) should have sent me straight to sorting the error classes, or to stopping.
4. **What I left behind:**
   - Nothing is committed.
   - Uncommitted in the tree: the edits to `keeps.nika` and `check_references.nika`, the three new tests in `tests/compiler/tests/` (`a_matched_parameter_stays_lent`, `a_matched_payload_moved_out_keeps`, `a_library_text_view_is_lent`), and 42 regenerated lowered files that do not compile. The tree needs restoring or finishing by name.
   - No report was written.
   - Step (4) is not built: the `check_references.nika` edit has never been compiled or run (the build fails before), and `examples/escaping` still has its `ref`.
5. **One change that would have saved the most:** have the brief say that a change to a keep column runs one `lower-std` and counts the changed lowered signatures before any build. Over about 5 files, stop and report the error classes with probes as `partial`, then split the brief: first the method-parameter defect (`p5`), then the matched-subject view, then the keeps rule.
