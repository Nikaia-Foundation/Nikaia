1. **What cost the most:** the full `tests/compiler --both-settings` run, about 10+ minutes of waiting. The edit itself was quick because the brief named both places to change. I also spent two turns on row b. The brief said it prints `1` today, but it failed in rustc on the suite binary. I re-probed it in a second shape to rule out a mistake in my own probe.
2. **What I looked up:**
   - `nikaia test --help`, to find out whether it filters by name. It doesn't, yet the brief says to run tests "by name".
   - That `views_the_call_s` has a third caller, `names_borrowing` in `views.nika:463`. The brief named only two.
   - The `emit_boxed_future_in` helper and how the `crossing` suffix is written in `emit_core_types`.

   None of these was expensive.
3. **Checks run:**
   - Rows a, b, c and the Vec variant before and after the fix, on both binaries.
   - The cycle (two `lower-std` runs and the build), plus a second build to confirm the fixpoint.
   - `cargo fmt --check`, clippy at 1.97, clippy at 1.99.1 on `nikaia-std` with `--features compiler`, `pre-push-check.sh`.
   - All of `tests/compiler`.

   Partly wasted: clippy at 1.99.1, because only lowered files changed and their lints are allowed. The full `tests/compiler` run was unavoidable without a name filter, and it ran about 472 tests to check about 45.
4. **One change:** in the brief template, write "the tests are run by running all of `tests/compiler` once" instead of "run by name". Or better, add a name filter to `nikaia test` (`--filter SUBSTR`), which would cut most of the 10-minute wait for a brief like this. Also, check the claimed "today" result of each row before filing it: row b was stated wrongly.
