1. **What cost the most.** Getting the rules block and the context took the most: I read y27-load.md and y29 in full, two long briefs whose Rules and Report sections are identical. Closing out cost the next most. The push was rejected, and then I amended my commit twice to fit a `Pushed:` line into the report. That was three commits for one line, and the sha in the first try went stale as soon as I amended.

2. **What the brief could have handed me.**
   - The *To build* text of #928 (I fetched it with `gh` and pulled out the section).
   - D6 of ADR-yaml-1.
   - The name of `load`'s key-comparison helper (`key_of` in eef3a2e7).
   - Above all, where the lent-parameter inference of ADR-277 D17 lives. I never found it, so the y30 brief only gives a low-confidence `rg` hint. One line in builder-facts would have done it, something like "lent-parameter inference: file:function".

3. **Checks.**
   - Useful: running the #1092 probe on `target/fast/nikaia`, both the `match` form (E0507) and the `??` form (prints 3). That confirmed the defect and gave a row that must keep working.
   - Wasted: grepping builder-facts for "lent" (nothing relevant), and the broad grep across tools that listed nine files without narrowing anything.
   - Not run: rows d and e of #1092. I left them to the builder to probe.

4. **One change that would save the most.** Have the driver write two things into `architect-input.txt`: whether `unpushed.txt` is non-empty, which means a push will be rejected, and the path of the current rules block as a file (`docs/briefs/yaml/rules.part`). The architect could then write `Pushed: rejected (unpushed batch)` before the one commit and paste the rules with a `sed`. That saves reading whole briefs and amending.
