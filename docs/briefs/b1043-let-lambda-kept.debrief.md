1. **What cost the most.** Three build cycles that each found one more layer of the same problem, so the checker fix needed three passes:
   - the #997 look-ahead refused callees that keep their argument;
   - rustc would not unsize `Kept<closure>` without a cast;
   - the parameter types were unknown (i32 against i64).

   On top of that, another session ran `git pull --rebase --autostash` during my build. That build silently lacked my edits, so one probe round was wasted, and it left `UU` and foreign files staged.
2. **What I looked up that the brief could have handed me:**
   - the path from #997's look-ahead to `run_code` and `lambda_params_by_use`, and that `check_core_stmt_code_parameter` skips keeping callees;
   - that a kept lambda's emitted form is `Kept(Arc::new(..))` and needs a `dyn Fn` cast when the `let` has no type;
   - that row d (a run-only callee) already fails on develop.

   One probe line per row, with the unfixed output, would have saved two cycles.
3. **Checks:**
   - **Useful:** the per-row probes, the lower-std fixpoint, the guard set plus `selfhosting_defects::` (43 passed), clippy, pre-push-check, and `tests/compiler --both-settings` (434 passed; the one failure is environmental).
   - **Wasted:** the probe round on the build that lost my edits to the autostash, and one `^error` grep that `--message-format short` can hide.
   - **Not possible:** a stash-based unfixed run, since stash is forbidden here. Early probes on the old binary stood in for it.
4. **One change.** Give builder.md, or the brief, a "run the rows on the current binary and paste their outputs" step before any edit. It should also say that a rejected push only needs the supervisor's rebase.

   Separately, the rules forbid `--autostash` for every session, but another session still ran it during my build. That hazard cost more than anything in the brief.
