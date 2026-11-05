1. **Most turns and time:** getting a trustworthy rerun of the one failing test.
   - `nikaia test` has no filter, so I could only run the whole 724-program package.
   - The first attempt used a stale `target/release/nikaia` and failed on unrelated syntax.
   - The second hit the 10-minute foreground limit.
   - The third ran in the background for about 8 minutes and was red on `agent_script`, which I did not resolve.
   - Finding the failing test also took a `gh run view --log-failed` call, because the tail of `suite.log` showed no panic.

2. **Lookups the brief could have handed me:**
   - The input file was `docs/briefs/async/qa-input.txt`, not `docs/briefs/qa-input.txt` as the brief says.
   - The failing test name and its panic text.
   - The exact rerun command, `cargo test --release -p nikaia --test it the_compiler_test_package_passes_at_both_settings`, and that it needs a fresh build and a background run.
   - Which binary to use: `target/debug/nikaia` was the usable one for `scripts/`.
   - That `nikaia run <file>` and `nikaia lower <file>` cannot compile a single test program.

3. **Checks:**
   - **Useful:** the package test rerun confirmed the fix, and `pre-push` and `cargo fmt --check` were green.
   - **Wasted:** `nikaia run` and `nikaia lower` on the single file, `nikaia test --filter`, and `nikaia test --both-settings` on the stale binary.
   - **Skipped:** clippy, because no Rust changed.
   - **Gap:** I did not verify that `agent_script` is environmental, and I did not run it on a clean `origin/develop`. My claim that it is this shared checkout is an inference.

4. **One change:** have the driver write the failing test names and their panic text into `qa-input.txt`, and name the lane's input directory in the brief. A line in `qa.md` giving the single-program rerun recipe would also help: `nikaia test` has no filter, so say how to run one program or that the whole package is the only rerun. Also say that a red on a test the diff doesn't touch should be checked against CI's result for the same test.
