1. **What cost the most turns:** tracing the GR builder's trust question back to its source. That took about 6 reads: `architect-input`, the GR report, ADR-010, `trust.rs` `join`, the emitter's `map_name`, and the `std.contracts` entry for `process::run`. The answer was in ADR-010 D9 all along, which the builder could not see from where it stood.

2. **What I looked up that the brief could have handed me:**
   - The ADR that governs the area of a `Needs: architect decision`. Here that was ADR-010 D9, and its *Rejected* list answers per-file trust directly.
   - Each blocker's state for each blocked move. I worked out by hand that #1080 was done (d436703a) and that #1082 waits on s1082, which sits in the same batch.
   - The path to the builder's saved diff, which lives in another session's scratchpad.

3. **Checks I ran:**
   - Useful: `git apply --check` of the saved patch; the issue labels of #1080 and #1082; a grep of `project_members.rs` to confirm all the drifting maps are private to that file.
   - Not run, and they should have been: lowering the re-land to see whether the keyed `project_members.rs` compiles; a probe of my suspicion that `join` fails open.
   - Wasted: none, but the `grep` across `.ledger` files for provenance found nothing; `std.contracts` was the right file.

4. **One change:** the builder's report should keep a resolved draft in the repo, never in a scratchpad. `builder.md` should say: "A `blocked` report saves its resolved work as `docs/briefs/<brief>.wip.patch`, and on a decision question names the ADR it suspects." That saves the architect the copy and most of the trace. There is also an error of mine to fix: the brief's line on restoring `runtime/lowered.rs` is a conditional `git checkout` that a builder could misread. It should say "restore it by name, from HEAD".
