1. **Most turns:** about eight greps and reads before writing code, all to answer one question. The brief said "add a field to `CheckState`", but `CheckState` is also built in Rust (`hollow()` in `crates/nikaia/src/check/core.rs`), which is outside my file list. I looked for a way around that: `CheckFlow` (also off the list), marking each binding as a parameter (lambda parameters carry the same mark), and finally scanning the frames, which needs to know the scope starts empty. Time otherwise went to one test run of about 60–70 minutes, which was needed.
2. **Lookups the brief could have handed me:**
   - that `CheckState` is also constructed in `core.rs:64`, so a new field touches a file the brief forbids;
   - that `frame_holding` already exists (`check_scope.nika:35`);
   - that `check_core_block` pushes its own frame, so a top-level `let` lies deeper than the parameter frame;
   - that the scope starts empty when a function is walked.
3. **Checks:** one build cycle (clean), the probe on the old and the new binary, `cargo check -p nikaia-std --features compiler` (clean), then clippy, the guard set and `tests/compiler --both-settings` in one background run, then `scripts/pre-push-check.sh`. Nothing was run twice. The only waste was my no-op `echo` call "to wait", which was redundant. I never ran `scripts/where.nika`, which might have found `frame_holding` sooner.
4. **The change that would save the most:** before a brief offers "add a field to struct X", it should check where X is constructed, with `where.nika X`, and include those files in the allowed list. Otherwise the builder has to choose between the stop rule and the brief's own decision.

Two errors in my report: "Disk freed: 0.3 GB" is an estimate, not measured. And the report was edited after the commit, which is fine only because the report itself is not in `Files:`.
