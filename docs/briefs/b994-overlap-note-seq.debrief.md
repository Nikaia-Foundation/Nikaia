1. **Most time:** the `tests/compiler` run, which was the whole 379-test suite. I ran it twice, once to get the expected `.stderr` and once to confirm it passed. I also lost a few turns when `nikaia check` in `tests/compiler` checked the project instead of my file. That wrote another test's error into my `.stderr`.

2. **What the brief could have handed me:**
   - How to get a `.stderr`. `--bless` only writes `.stdout`, so the brief should say to copy the refusal block from the test run's failure output, with the path form `tests/NAME.nika:6:9`.
   - That `nikaia test` has no filter and always runs the whole project.
   - That one `lower-std` / build / `lower-std` cycle gives the lowered diff directly.

3. **Checks:**
   - **Useful:** the cycle, the probe with `nikaia check`, the project test run, and the guard set.
   - **Wasted:** the first `.stderr` attempt, and the project-wide test run used just to read one test's output. I did not run the new test against the unfixed compiler. I judged it failing by construction, which is a shortcut the rule did not allow, and the report says so.
   - **Unconfirmed:** clippy and `pre-push-check.sh` printed nothing, and I treated that as passing without seeing a positive result.

4. **One change:** add a recipe to `builder.md` for a new refusal test: write an empty `.stderr`, run the project once, copy the refusal block from the failure into it, then re-run. It should also say that the check against the unfixed compiler is done by running the test with the old binary copy. Separately, a push rejected for a non-fast-forward should tell the builder what to do, for example whether to fetch, since `commit.nika` only reports it.
