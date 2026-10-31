1. **Most turns and time.** Finding the driver's input and then the real cause.
   - The brief named `docs/briefs/qa-input.txt` and `docs/briefs/bugs/qa-input.txt`. Neither existed. The file was in `docs/briefs/prover/`, a third lane the brief doesn't mention.
   - The step line only said `rs2nika/convert` red, with no failure detail in `suite.log`. I had to rerun the package myself.
   - My first run printed about 1,500 lines of unrelated warnings, so I wasted a round grepping noise before redirecting to a file.

2. **What I looked up that the brief could have handed me.**
   - The lane directory, so the brief should have named the exact `qa-input.txt` path.
   - What the failing test does: it runs `nikaia test --both-settings --locked --project <pkg>`. I read `examples.rs` to find that out.
   - The expected toolchain, which includes the `rust-analyzer` component. The rs2nika converter needs it, and no document I read said so.
   - That an earlier QA run's report already existed in git history (deleted in the working tree). It turned out to be about a different failure.

3. **Checks run and wasted.**
   - Useful: the `--locked` package run, the `rustup component add rust-analyzer` fix, the rerun that went green, and `cargo test --profile fast` on the gate test.
   - Wasted: the first package run piped to `tail`, and a second identical run just to capture output to a file. Those were two full runs of about 140 tests where one redirected run would have done.
   - Not run: `pre-push-check.sh`, clippy and fmt. I changed no files, so skipping them was deliberate.

4. **One change.**
   - The driver should write the failing test's own output tail into `qa-input.txt`, not only the step line. The gate would then show "rust-analyzer: Unknown binary" immediately, and I would have gone straight to the environment.
   - The brief should also say to look in `docs/briefs/*/qa-input.txt` when the two named paths are missing.
   - builder.md or `qa.md` could add a line: "if the failure is the environment, say so and commit nothing."
