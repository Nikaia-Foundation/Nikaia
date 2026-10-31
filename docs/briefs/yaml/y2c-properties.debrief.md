1. **What cost the most:** isolating defect 1 took about 8 runs of probes and bisects.
   - The NK1185 points at `is_float`, which the draft did not change. I first had to confirm that develop's own code checks.
   - Removing guards one at a time in the package copy gave the same refusal every time (variants b, c and the four "keep one guard" runs). Only a standalone probe with a second caller showed the cause.
   - Finding defect 2 meant one full package test run on a scratch copy. It was useful, but it was the longest single step.
2. **What I looked up:** almost nothing beyond what the brief named: the #1026 probe in the y8 brief, and the y2 report and debrief. I needed no ADR, converter or file locations. My first probe also hit #1001 (a group like `("-" | digit)`). builder-facts lists that defect, but I didn't think of it while writing the probe.
3. **Checks:**
   - Useful: the pre-flight probe, `nikaia check` on the draft, the minimal probes p4 and p6, and `nikaia lower` to read the generated line 560.
   - Wasted:
     - probe p2, which ran into #1001;
     - bisect variants b and c, which repeated what the "keep one guard" loop showed;
     - the package test run as a whole. A single `nikaia lower` plus `nikaia run` on one test would have found defect 2 faster.
4. **One change:** before writing a "start from the draft" brief, the architect applies the draft to develop and runs `nikaia check` once, plus one package test. That takes seconds to minutes, and it would have shown NK1185 and the `s` scope error. Both compiler fixes could then have been scheduled ahead of this brief, as the y2 debrief already asked for #1026. Second: builder-facts could say that a call inside a grammar guard is a separate case for the text-tier pass and the guard-scope lowering, so probe it alone.
