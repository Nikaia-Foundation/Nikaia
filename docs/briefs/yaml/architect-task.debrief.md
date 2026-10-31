1. **Most turns:** getting the probes to run. Rebuilding the y2 report's probe took four tries: `('a'..'z')+` doesn't parse, a rule without an action needs `-> { }`, and `alpha` isn't a defined rule. The control probe through a function call took four more: NK1261 for `ref String`, String has no `slice`, NK1214 for cutting at a made-up number, NK2105 for a moved parameter. It only ran once I switched to literals.

2. **Looked up that could have been handed to me:**
   - The y2 report gave its probe as fragments, without the `Word` rule, so I had to rebuild it.
   - Which grammar primitives exist (`digit`, `any`; no `alpha`, no character ranges).
   - That the text-tier walk lives in `crates/nikaia-std/src/tools/text_tiers.nika`, not in `tools/`. Finding `bind_grammar_pattern` and `called_struct` took about five greps.
   - That `wip/y2-properties` sits on the pre-rebase commit 9fe7210f, not on 9d92e474.

3. **Checks:**
   - Useful: the failing probe and the same probe with one alternative removed; the control probe showing the function path (#803) works; ADR-282 D12–D15; `git apply --check` of the draft onto `develop`; reading #928's *To build*.
   - Partly wasted: the four failed control probes; reading the end of y2b's rules block; one find for a `tools/` directory that doesn't exist.

4. **Biggest saving:** `builder.md` should require a `Found:` probe to be a complete file that runs as written, saved beside the report (`<brief>.probe.nika`). `docs/builder-facts.md` should also get a short list of grammar primitives and the smallest working grammar program. Together these would have saved about eight turns.
