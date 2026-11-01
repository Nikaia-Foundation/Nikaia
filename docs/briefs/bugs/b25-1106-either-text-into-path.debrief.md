1. **Cost:** The one-test project run, which had to build the project and lower the test program. I also built `target/fast/nikaia` and ran clippy over all targets. There was no search. The brief named the cause and the fix, so I made one edit, one build and one test run.

2. **Lookups the brief could have handed me:** Nothing material. The brief gave the file, the impl to copy and the probe. I used the scratch-project recipe from the facts section. I did not look up any ADR.

3. **Checks:**
   - **Fails-before:** I ran the old binary from the suite worktree, and the test failed in rustc as the brief expected.
   - **Passes-after:** I ran the new binary with `--both-settings`. It printed `abc1 afalse`.
   - **Fmt, clippy, `pre-push-check.sh`:** these were clean.
   - **Not run:** the brief's `either|mixed_arms|path` set, because `nikaia test` has no filter. I said so under `Tests:`.
   - **Wasted:** nothing outright. Clippy over all targets was probably more than a one-impl change in a std file needed, but the brief required it.

4. **Brief change:** Give the exact command for "the tests named `either|mixed_arms|path`", or list the test files. As written, the step can't be done with the available tooling, so I skipped it and had to record that in the report. A line such as "run these N files through a scratch project" would remove the ambiguity.
