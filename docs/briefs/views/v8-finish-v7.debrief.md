1. **Most turns:** working out why a text literal became `&String::from(..)`. It took about 15 turns to trace from the emitter (`emit_core_args`: `lent_args` and `owned_texts`) back to the checker, past several `owned_texts.insert` sites that were not the cause. The real cause: a method call walks its arguments against the parameter type (ADR-288), so the literal becomes owned text before the call knows the position is lent. The NK2302 refusal in `views.nika` (`rsrc_report` ignored a written `from`) cost one more build cycle, about 10 minutes.
2. **Lookups the brief could have handed me:**
   - `nikaia test` has no `--filter` (the brief's fact was wrong). I had to build a scratch project per test set, and the untracked `src` files had to be left out.
   - The 2026-11-02 release binary cannot parse `from`, so it cannot show the test failing before.
   - Where NK2302 is raised for trait impls (`views.nika`, `rsrc_report`), and that `check_result_from` already checks a written `from` against the body.
   - That `owned_texts` decides whether a literal is emitted as `String::from`.
3. **Checks:**
   - Useful: lowering one-file probes with `nikaia lower` to read the emitted Rust (this found the call-site defect quickly), and comparing free functions with methods.
   - Useful: the lane tests in one scratch project, 128 plus 111 tests in two runs.
   - Wasted: the release-binary run (it fails to parse `from`), the first scratch project that copied untracked `src` files, and an early guess at the call path through `check_core_call_on` and `check_core_method`.
4. **Change that would have saved the most:** in `builder.md`, a short "how to run one test" recipe, i.e. a scratch project with the tracked `src` and the named tests, run with `nikaia test --both-settings --project`, and a check that a fact like `--filter` exists before the brief states it. Second: a note that a newly lent method parameter shows up in the checker (literal ownership, trait-view rules) as well as in the lowering.
