1. **What cost the most:** about 8 turns of tracing. The brief placed defect 1 in the emitter (`emit_function.nika` ~564). In fact the impl's `keeps` column comes from `crates/nikaia/src/contracts/keeps.rs::infer`. That same column drives the checker's `kept_args` fact (the `&&*known` at the direct call). After that came about 3 probe turns on an unrelated false acceptance (`let f = h.f` through a `ref`) before I found a test shape that avoided it.
2. **What the brief could have handed me:**
   - Where `keeps` is computed: `keeps.rs` `infer`, with an `Item::Trait` arm and impl methods through `uses_of`.
   - That call sites are decided by checker facts (`kept_args` in `check_core_args.nika`), not only by the emitter.
   - That method calls carry the resolved key in `walk.method_callee`; a bare `callee` is the method name, which matches several candidates.
   - That `takes_a_handle`'s `duplicate` (`.clone()` after the argument) already exists as the pattern to copy.
3. **Checks:**
   - Ran: the `lower-std`/build cycle twice, then `nikaia run` probes (mine, the trait probe, the free-function control rows), the guard set, `tests/compiler` at both settings, clippy and the pre-push check (all green).
   - Wasted: the first build cycle, because it keyed on `callee` and missed method calls; two probe runs on the field-move issue.
   - Running the gate in the background with a 600 s limit was risky: `tests/compiler` alone can take about 10 minutes.
   - The push was rejected and I did not rebase, per the rules.
4. **One change:** before writing a brief that confines a fix to `tools/emit_*.nika`, the architect should check where the deciding fact comes from: the contract column (`keeps.rs`) and the checker fact (`kept_args`). Here that check would have produced a brief allowing a ~10-line union in `infer`, so the whole issue could have closed in one run instead of `partial`. For builder.md: a trap line saying "fn-parameter run/keep is the contract's `keeps` (`contracts/keeps.rs`), read by both the checker and the emitter."
