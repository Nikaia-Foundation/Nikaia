1. **Most turns and time:** designing the new Rust host for K3a part 1 (`ComptimeLowerHost`). I read about 480 lines of `comptime_run.rs`, `comptime_tree.nika`'s API, `Parsed::keeping`, `may_not_run`, the M3b host pattern and P5b's step for the converter rows (`[forms]`/`[hosts]`), about 8 tool calls. The second biggest cost was reading `docs/architect.md` whole: about 29k tokens, three quarters of it the *State* diary, which I needed only for the newest two entries.
2. **What the brief could have handed me:**
   - The cut report's `Deviation:` text with the governing ADR next to it (D10), so I didn't have to reason out whether the cut was false.
   - The current *State* entry alone, instead of a pointer to the whole file.
   - The plan row's `Cut:`/`Convert:` line for the next move, with ranges checked against today's tree. I had to re-check that K1 landed after a4b33a0 and re-grep the function lines.
3. **Checks:**
   - Useful: `git log` on `project.rs` (p5e's ranges still hold), the grep of callers (`grammar_run.rs`, and none in `it/`), and the `--tethers` flag existing before I wrote a rule naming it.
   - Wasted: none really. The CI query only showed runs in progress, so it decided nothing.
   - Not run, so the k3a1 brief carries unverified claims:
     - that the `it` filters `comptime grammar_at_build_time` match any tests;
     - that `String.len()` counts bytes (I handed that check to the builder);
     - that P5b's convert command still works.
4. **One change:** split `docs/architect.md` so that *State* keeps only the last 3 entries and the older ones move to `docs/architect-history.md`. Also, `drive.nika` should write the cut brief's report and debrief paths into `architect-input.txt`. That would have saved about 25k tokens and two calls.
