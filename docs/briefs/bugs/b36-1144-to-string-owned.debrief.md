1. **Most turns: reproducing the bug.** The brief says the probe is "verbatim in #1144" but doesn't include it. My first probe was rebuilt from the prose and used `??`, and that form works. Getting there also took three tries to learn the `collections::BTreeSet()` spelling. In the end I found the real probes (`s3_gap3*.nika`) in another session's scratchpad via the S3 debrief, about 8 turns in. The second big cost was learning that `.to_string()` of a view stays a view on purpose (ADR-282 D8). The brief calls owned `to_string` "specified behaviour", so I had to stop and check the ADR and the checker's existing NK1102 refusal before choosing a direction.

2. **What the brief could have handed me:**
   - The probe text itself (8 lines).
   - ADR-282 D8, plus the fact that `mk(n).trim().to_string()` is already refused with "Write `.clone()`".
   - The suspected location: `text_tiers.nika`, the `Expr::Match` arm binders (`bind_whole_name`, `bind_some_payload`), the `to_string` arm of the method tiering, and the `Stmt::For` binding.
   - That an unbound local counts as owned (`kinds_of_local`).
   - That `--tethers` shows "borrowed `<result>`" with no source.

3. **Checks:**
   - Useful: old-binary probes (a, c, e, f variants), `--tethers`, two lower-std/build cycles with the fixpoint, both new tests blessed and run `--both-settings` in a scratch project, the `it` text/view modules (203 passed), clippy on 1.97 and 1.99.1, `pre-push`.
   - Wasted: my own first probe (wrong shape), and tracing `put_viewed` into `emit_stmt` (the keep path wasn't where the fix went).
   - The second build cycle was needed only because my first `rooted_in_a_parameter` followed every method call. I could have caught that before the first build.

4. **The one change:** a bug brief should paste the issue's probe verbatim, its exact output, and its file path. It should also check the requested behaviour against the governing ADR, here by quoting ADR-282 D8. Then the supervisor decides the D8 conflict before the run, not after it. That would have saved about 15 turns and the deviation.
