1. **Most turns:** finding out that the checker sees the literal tier as a view. The tier pass (`crates/nikaia/src/text_tiers.rs` `mark`) rewrites the parsed types before the checker runs, so `a_map_read_kept` returned early on `read.is_a_view()`. I spent about 8 turns reading `check_core_leaves` (the `lending` path), `check_values`, `check_items` and `views` before I grepped `lasting` in the Rust code. Second: I found late that `return m[k]` failed the same way, which cost one more build cycle.
2. **Lookups the brief could have handed me:**
   - That the tier pass runs at parse time and turns a literal-tier `String` into `is_view && lasting`, so the checker's `Ty` has `view: true` and cannot tell a lasting view from a borrowed one.
   - That `map_read_sites` is set in `check_core_expr.nika:88`, with the raw read type at hand there.
   - That the bracket form `m[k]` fails too. The brief said only `m.get(k)`.
3. **Checks run:**
   - Lowering probes with the old binary, which were useful.
   - Three build plus `lower-std` cycles: two for the fix, plus one fixpoint round because `fold_bound.rs` changed.
   - `nikaia run` on 7 probes, then the gate (clippy, guard set, `tests/compiler` at both settings), then `pre-push-check`.

   Wasted: the first fix was method-only, so one whole cycle went before I extended it to `m[k]`. Running the `m[k]` probe at the start would have saved it. The numbers probe (p7) answered nothing the brief needed.
4. **One change:** builder-facts should say: "the text tier pass runs before the checker. The literal tier is `is_view + lasting` on the parsed type, so a checker `Ty` sees it as a view." A brief about a text-tier defect should also list the bracket form `m[k]` among the cases to probe. One probe matrix with every spelling (`get`, `[]`, `let`, typed `let`, argument) written before the first build would have saved a cycle.
