1. **Time sink.** The failure only showed on a cold cache. My first by-hand `nikaia build` and `nikaia test` passed, and a warm rerun of the test passed too. I lost about six calls before I deleted `examples/*/target` and `target/nikaia-example-tests` and got the failure back. The driver's `qa-input.txt` held no error text for the two red tests. It had the step names and 200 lines of unrelated warnings, so I had to rerun the tests to see the message.

2. **Lookups the brief could have handed me.**
   - Where the gate's per-test output is. `/home/user/nikaia-suite/target/gate-tests-*.log` only covered the compiler and rs2nika tests, not the `it` tests that failed.
   - That `qa-input.txt` is in `docs/briefs/prover/` for this lane.
   - That `nikaia.proofs` is the regenerated ledger file.
   - The commit script's report format (`Files:`, `Commit:`, `Changelog:`, `Body:`). I read `commit.nika` and a builder's report to find it.

3. **Checks.**
   - Useful: rerunning the two failing tests, then rerunning them cold. Regenerating the proofs and checking `git diff` was also useful.
   - Wasted: the first `grep` that cut off the c_library error, and `nikaia test` in tally and with a fresh cache dir, which only showed a warm state.
   - Skipped: I did not run clippy or `cargo fmt`, since no Rust changed.
   - Not checked: I did not look into why the three new entries are `refuted` rather than `proved`. I assumed they were the correct answers to p12's new goals.
   - `scripts/pre-push-check.sh` printed nothing, and I assumed that meant green without checking its exit code.

4. **Change.**
   - Add one line to `builder.md`: a builder who changes what the prover asks must regenerate `nikaia.proofs` in every example and package from a clean target (`rm -rf examples/*/target`), and rerun `examples` and `c_library` cold.
   - Add one line to the driver: put the failing test's own error lines, not just its name, into `qa-input.txt`.
   - The first would have prevented the cause. The second would have saved my reproduction time.
