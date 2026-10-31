1. **Most turns lost:**
   - About five greps to find the signature of `proven::read`. It lives in `crates/unsafe/proven-index`, not in `nikaia-std/src`.
   - Three wasted probes of a map case. I wrote `Map[..]`, then `HashMap[..]`, then hit `= {}` being refused. The map case was outside the brief anyway.
   - A few turns finding where the `*` is decided: the `deref` flag of `emit_core_index_read`, passed from `emit_core_index` in `emit_core_places.nika`.

2. **What the brief could have handed me:**
   - The brief's locations were partly off. `emit_host.nika:158` and `a_copy_by_value`/`value_of_a_copy` in `check_types.nika` are not on this path.
   - The real decision point is the `deref` parameter of `emit_core_index_read` in `emit_core_reads.nika`. The checker side is the `Expr::Index` arm in `check_core_expr.nika`.
   - The precedent was #1008's `hands_on_kept` in `emit_core_args.nika`, and the fact pattern was `list_indices` in `check_facts.nika`.
   - I also looked up the Part I heading to cite, and where the crate behind `proven` lives.

3. **Checks:**
   - Useful: the probe matrix on the old binary (`let`, push, keep, loop, run-only argument, variable copy, field copy), one build cycle, the probes on a copy of the binary, then the gate once (clippy, the guard set, `tests/compiler` at both settings, `pre-push-check`).
   - Wasted: the separate fixpoint rebuild. The cycle's own second `lower-std` had already shown it; the build took 0.08 s and changed nothing. The map probes were wasted too.

4. **One change:** a brief for a lowering bug should name the emitter function and parameter that writes the wrong text (here the `deref` flag of `emit_core_index_read`), plus the closest earlier fix (#1008 `hands_on_kept`). It should also say whether sibling shapes are in scope; here those were copying from a variable and from a field. I met both and could only list them under `Found:`. Separately, `builder.md` could say where the `nikaia_std::proven` crate lives.
