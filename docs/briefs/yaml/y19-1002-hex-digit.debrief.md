1. **Most turns:** finding out which yaml test failed. I ran the package suite three times: once with `tail`, which hid the failing test's name, once with a grep, and once more to see the full output. Running a test file on its own (`nikaia run tests/X.nika`) is refused in a project, which cost one more turn. Next came deciding whether the shifted error positions were a defect or the right result. I spent a turn looking in the spec for failure-position rules and found nothing useful.
2. **What the brief could have handed me:**
   - The brief points to `is_hex` "near line 1086", but `Hex` was its only user.
   - It doesn't say that `errors_carry_line_and_column` checks error offsets of bad escapes, which a guard-to-built-in change can move.
   - It doesn't say that `nikaia.contracts`, `nikaia.derived` and `nikaia.lock` under `packages/yaml` are tracked and change with the source.
3. **Checks:**
   - Useful: the package suite, `fmt --check`, `fmt::every_nika_file`, `pre-push-check.sh`.
   - Wasted: two package-suite runs, which one run written to a file and grepped would have replaced. builder-facts already says to do that, and I didn't.
   - I never read builder-facts whole, so that wasn't wasted, but I could have missed a trap that was in it.
4. **Change that would save the most:** the stop condition "escape tests change" was ambiguous. The brief should say whether error *positions* count as escape-test output. For example: "error offsets may move to the offending character; update the `.stdout` and say so in `Body:`". Or the opposite, if the architect wants a stop. Either way I wouldn't have had to choose between stopping and deviating.
