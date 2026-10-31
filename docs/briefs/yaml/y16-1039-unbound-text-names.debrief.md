1. **What cost the most.** Three `lower-std` round trips, each over a refusal about borrowing that I could have avoided by writing the code more plainly first:
   - `for bound in kept` pushed a `ref CheckLocal` into a `Vec`.
   - `let below = match … => pat.node` was refused with NK2106.
   - A helper returning `args[0]` (or `args[0].node`) as `ref Spanned[Pattern]` was refused with NK1104, but returning a match binding works. I worked around it by writing two arms. This may be a defect, and my report should have listed it under `Found:` with a probe; I left it out.
   
   After those, the gate ran once, long but in one call.
2. **What I looked up that the brief could have handed me:**
   - `bindings_of` has one caller (`check_function.nika:324`).
   - `check_defaults_folds_in` does not depend on the action's frame, so fold steps were unaffected.
   - `text_tiers.nika` `bind_grammar_pattern` already over-binds and needs no change.
   - The action prelude's line in `emit_grammar.nika` (~408).
   - `Span` holds `start: u32`.
   - How to get a `.stderr`: `nikaia check` inside `tests/compiler` checks the whole project and showed another test's error. I needed a scratch project with `tests/NAME.nika`.
3. **Checks I ran:**
   - Useful: probes a–e on the unfixed binary, then after the build, plus one extra `Bytes` probe; `packages/yaml` tests; the gate, whose test step covered the whole `tests/compiler` package at both settings.
   - Partly wasted: the first `nikaia check` for the `.stderr`; the `grep` for `text(` uses across examples (no hits in grammars); the gate `--dry-run` to confirm the test step's scope. That scope is in builder.md already.
4. **One change that would have saved the most.** In builder-facts, *Traps*, a note on writing tools: "a function cannot hand back `xs[i]` as `ref`; a `let` cannot keep a field of a borrowed value; iterating a local `Vec` yields `ref`. Write the loop inside each match arm." Add the `.stderr` recipe too: a scratch dir with `tests/NAME.nika`, then `nikaia check tests/NAME.nika`.
