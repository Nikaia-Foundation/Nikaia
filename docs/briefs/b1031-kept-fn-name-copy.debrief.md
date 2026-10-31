1. **Most turns:** tracing where a function value is a `Kept` and where it is only a reference. That took about 15 read turns over `emit_expr`, `emit_core_args`, `CheckLocal`, `run_code`, `check_core_free_call` and the assignment target. The key facts were: a parameter that only runs its function is in `state.run_code` (and lowers to `&impl AsyncFn`), a name callee is never passed to `check_core_expr`, and an assignment target is.
2. **What the brief could have handed me:**
   - `state.run_code` as the test for "this name holds a `Kept`".
   - The argument code calls `emit_expr` on its argument, so an emitter-wide `.clone()` also lands on lent arguments, run-only arguments and the closure-wrapper paths.
   - The struct-literal shorthand `H { f }` takes its own branch (`kept_byte_fields` is the model).
   - `EmitWalk` is built in one place, `EmitWalk::fresh()`.

   I needed no ADRs or converter.
3. **Checks run:**
   - Three probe programs on the old binary using `lower` to see the Rust it writes, then `run` with the new one.
   - One build and two `lower-std` runs (the fixpoint check).
   - `tests/compiler` at both settings, the guard set, clippy and `pre-push-check`.
   - Nothing was clearly wasted. The extra `cargo build` I ran to check the fixpoint only recompiled once; an md5 of the `lowered/` diff before and after `lower-std` was cheaper and enough.
   - Two probe rounds hit unrelated refusals (NK2605 on a field call, NK1131 on `return h.f`) and cost a rerun each.
4. **One change:** the brief's *Locations* should name the positions where the fact must be taken out again or must not apply (callee, assignment target, lent or run-only argument, `run_code` parameter, shorthand field). The emitter sites alone are not enough, and finding these exceptions was most of the reading. builder.md could also say how to show "fails on the unfixed compiler" without a stash: `nikaia lower` with the old binary before the first build.
