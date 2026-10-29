1. **Most turns and time: finding where the fault was.** The brief pointed at `views.nika` (`rsrc_*`) and `text_tiers.nika`. The cause was in `buffers.nika` (`made_here`), with a second part in `views.nika`. I spent about 15 turns reading `rsrc_method`, `rsrc_callee`, the tier code and `tether.nika` before a `--tethers` probe showed the free-call case (`make`) already uses the caller's keep and the method case does not. On time, the `tests/compiler` runs cost most: three full runs of about 10 minutes each. One was spent only to find the new test's line, because `nikaia test` has no name filter. Another was a rerun after four other-session tests landed mid-run and one failed against my stale binary.
2. **What the brief could have handed me:**
   - The diagnostic flags: `nikaia check --tethers` and the `nikaia.contracts` file written beside a lowered probe. These localised the bug in two calls.
   - That ADR-353 D10 already lowers "a view of a local owned `String` returned" through `__keep`. The brief's expected `-> String` conflicts with that, and I had to decide alone (I reported it under `Deviation:`).
   - `buffers.nika`/`tether.nika` (`made_here`, `makes_a_buffer`, `a_call_buffer`) as the place to look.
3. **Checks run:**
   - The probe on the old binary, which reproduced the bug.
   - A small probe matrix: free call, method with/without args, local receiver, a parameter view, a view-of-`self` method.
   - Two cycles, plus one manual extra `lower-std` to check the fixpoint.
   - The `it` filters and guard set (209 passed), clippy, `pre-push-check`, and `tests/compiler` at both settings, three times.
   - **Wasted:** the second `tests/compiler` run, only to grep the new test's name, because the first output had been cut by `tail`. Also `--lib contracts::keep` (0 tests).
   - **Gap:** I did not rerun the `it` gate on the final rebased tree.
4. **One change:** give builder.md a line on diagnosis: "for a wrong lowering, run `nikaia check --tethers F.nika` and read the `nikaia.contracts` beside the probe first". Also have briefs state the *expected lowering* only after checking the ADR that governs it, or say "any lowering rustc accepts". Smaller: `nikaia test` needs a name filter, or builder.md should say to grep the full output rather than `tail` it.
