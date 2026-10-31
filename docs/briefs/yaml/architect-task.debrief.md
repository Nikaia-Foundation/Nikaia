1. **What cost the most:** finding where the fault probably sits for #1089, about 8 turns. A broad grep for `match &` hit 48 files and told me nothing. After that I followed `lent_scrutinees` to `check_core_flow_through_a_view`, then `a_lent_let`, then the callers of `check_core_stmt_lent_let`. The report's probe also cost a run: it lacked `impl Error for E` and used `HashMap::new()`, so its first run stopped on NK1161 and NK1149.

2. **What I looked up that the brief could have given:**
   - #928's current *To build* text. I fetched it whole just to edit one sentence of step 4.
   - Which brief to copy the rules block from (y28, the newest one with an `Attribution:` block).
   - That y27's work sat on local branch `wip/y27-load` (eef3a2e7). The report said so, but I checked the branch to be sure.
   - Whether #1057 (b13) already covered this defect. I read that issue and the b13 brief to find out.

3. **Checks:**
   - Useful: the probe plus seven variants (map and list, `??` and `match`, index, `ref` parameter, clone in the arm). They gave #1089 its rows a–h and bounded the defect. Reading the lowered Rust of three variants side by side showed the missing `&` straight away.
   - Wasted: the broad grep, and the first probe run.
   - The push: I tried it, and it was rejected. I could have seen beforehand that local `develop` was ahead of `origin/develop` with the batch's unpushed commits.

4. **One change that would have saved the most:** two lines in the architect task.
   - Say that local `develop` holds the batch's unpushed commits, so the architect commits and writes `Pushed: rejected` without trying to push.
   - Say that a report's `Found:` probe must be a complete program that runs as pasted (error `impl`, constructors written out). The same rule belongs in builder.md, so the next architect can re-run a probe without fixing it first.
