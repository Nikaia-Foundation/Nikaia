1. **Most turns and time: running probes.** The first probe batch was wasted because I wrote typed lambda parameters (`fn(x: i64)`), and the parser does not accept those. One background run was killed after about a minute even though I had set a 900 s timeout, so I ran it again in the foreground. Each `nikaia run` costs a cold rustc build, so each round of probes took minutes.
2. **What I looked up that could have been handed to me:**
   - Lambda parameters are written untyped (`fn(x) { .. }`). That is a language fact for `docs/builder-facts.md`.
   - Where a lambda counts as "kept": `check_core_a_kept_lambda` and `kept_functions`.
   - Where keep plans are built: `buffers.nika` `made_arguments`. I found it through `emit_keeps` and a grep.
   - Which files the bug lane's in-progress briefs touch. I opened their briefs and found no file list.
   - The tail of `docs/architect.md` was too long to read in one call, so I had to page through it.
3. **Checks:**
   - Useful:
     - Probes a/b/g: the typed `let` works, so the `Variable` branch is the cause.
     - Probe f against f2 (the same struct field, inline lambda against let-bound).
     - Re-running #1036's probe.
     - The #1010 probe, which reproduces.
   - Wasted:
     - The typed-lambda batch.
     - Probe h, which hit the #1042 parser refusal I already knew about.
     - The killed background run.
     - Reading `emit_core_args`/`emit_keeps`, which only pointed onward to `buffers.nika`.
4. **The one change that would save the most:** a `where.nika` lookup by fact name (who writes `kept_lambdas` or `plan.calls`, and who reads them), plus each lane's `batch.txt` listing the files its briefs may change. Tracing a cause and checking for overlap with other lanes would then take one call each instead of 4–6 greps. Second choice: one line in `builder-facts.md` saying lambda parameters take no types.
