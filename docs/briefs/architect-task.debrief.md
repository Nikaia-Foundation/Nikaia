1. **What cost the most turns:** probing C1b. I wrote my probes as plain programs and kept running into problems that came from my probe, not from the compiler. Plain programs need `use std::collections`, `ref String` is refused outside `extern`, and I left out `mut`. That took about 8 iterations before the real defect (#1107) showed up. A second cost was the K3b probe: run as a plain program, the builder's shape gives NK1102. I had to find a shape that reaches `EitherText` (ADR-282 D13's mixed-arms local) before the rustc error appeared.

2. **What the brief could have handed me:**
   - That the remaining plan rows (S1, P5e, K3b, C1b) all had a blocker or a prerequisite. I had to rediscover that from `selfhosting-plan.md`.
   - How plain programs write views: `ref T` locals, `Vec[ref T]` results, map `m[k]` against `.get(k)`.
   - That ADR-222 is archived; `check-adr-refs` caught that only after I had written the brief.
   - That `LedgerOps::candidates` is dead code.

3. **Checks:**
   - Useful: the #1088 pre-flight probe, the conflict check of the P5c branch against develop, the `EitherText` probe, the rows for #1107, and `check-adr-refs`.
   - Partly wasted: the early C1b probes (my own mistakes), and the first K3b probe run as a plain program, which could not reach the defect.
   - Not done: no S1 brief, so that research only went into *State*.
   - The push: I pushed C1a (5bd603f3), which had only been committed locally, before the driver's suite had run on it. The task told me to push, but it conflicts with the rule that the driver runs the suite and pushes.

4. **One change:** a fixed probe header in `docs/builder-facts.md`, *Traps already met*, with the plain-program idioms (`use std::collections`; `ref T` only on non-text types; result views written without `ref` for text; how to reach `EitherText` in a plain program). Next to it, a one-line rule for `architect-task.md`: "push only your own commits; if HEAD carries unpushed builder commits, leave pushing to the driver." The first would have saved about 10 turns. The second removes the conflict over pushing untested commits.
