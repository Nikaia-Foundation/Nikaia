1. **Most cost:** turning json's single NK1300 into four separate bugs took about 15 probe rounds. Several were lost to my own tooling: a shell function I defined didn't carry into the next call, and a `sed` placeholder substitution silently matched nothing (probes n1/n2). The `j_hex4` probe also led nowhere: an index `% ds.len()` I added myself made it fail, not json's shape.
2. **Looked up, could have been handed over:**
   - Whether the red came from this lane: 43312d30 is a yaml-lane commit, and the main lane had already queued a fix-forward for it. Finding that meant reading the main lane's `batch.txt` and brief, plus `architect.md`'s long *State* section.
   - ADR-359 D1 (summaries skip loops) and D3 (a function promises its result with an `assert`), and Part I's rule that only a `mut` parameter changes the caller's value.
   - Code locations: `prover_state` `an_assert` vs `precondition_call`, `bounds_body` `untracked_in`, `prover_let`, `prover_join`.
   - That NK1241 already exists, and where dfe7bac6 stands relative to develop.
3. **Checks:**
   - Useful: the CI run list plus job annotations (one failing test), the spike probes run against develop, the narrowing probes (q1-q7, m1-m9, r1-r11, s1), and `pre-push-check`.
   - Wasted: n1/n2 (broken `sed`), `j_hex4` (the `%` index), a 64 KB grep for "ensures" across the spec, and reading all of the *State* history when one entry was enough.
4. **One change:** when the driver writes `architect-input.txt` for a red gate or a red CI, it should add the failing test, the commit that first broke it, and whether some lane's `batch.txt` already holds a fix-forward for it. That alone would have saved about 8 turns of working out who owns the red. Second: the architect brief should say "write the probe generator to a script file first" (as `docs/tokensaverstrategy.md` asks), since my inline shell functions broke twice.
