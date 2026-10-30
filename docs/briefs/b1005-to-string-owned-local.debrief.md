1. **Most time:** my first fix keyed on "a local that is not a parameter". It broke `to_string_of_a_view_fills_a_string`: a loop variable over a `ref Vec[String]` has owned kinds but is a view. I only found out from the 10-minute `tests/compiler` run, so that run, clippy, the `it` set and the pre-push check all ran twice.
2. **Looked up:**
   - ADR-282 D8's location: three greps under `docs/adr` and `docs/history` before I found `docs/specification/adr/adr-282.md`.
   - The `Kinds` and `TierLocal` structs.
   - That numbers already get owned kinds in the tiers (`Binary` and `Cast` give `owned_kinds()`, as do the scalar payloads in `bind_a_payload`).
   - How a `for` loop binds its locals.

   The brief could have given the ADR path and said "number values are owned kinds already, the fault is `viewed()` in the `to_string` rule". It had that rule's line but sent me to "where the local's kinds are formed", which was not where the fault was.
3. **Checks:**
   - Useful: the probe of all rows on the old and new binary (seconds), the `lower-std` fixpoint, `tests/compiler` at both settings, the `it` guard set with `text_tiers::` and `views_kept::`, clippy and the pre-push check.
   - Wasted: the whole first round of clippy, `it` and pre-push on the wrong fix. Also an extra fixpoint cycle, one `nikaia run` on a test file inside the project (refused), and a `cat` that lost `$S` because I backgrounded an `S=… &&` chain, the trap builder-facts already names.
4. **One change:** the brief should list the existing tests that mark the boundary (here `to_string_of_a_view_fills_a_string`, which it mentioned only as a note). builder.md should say to copy those tests into the scratchpad and `nikaia run` them right after the first build, beside the new probes. That takes seconds, where waiting for the full `tests/compiler` run took 10 minutes.
