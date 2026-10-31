1. **What cost the most time:** the test runs. The `it` filters took about 4 minutes, `tests/compiler --both-settings` about 10, and the single `it` re-run another 3. Following up the one `it` failure took three turns: my first run used the wrong path (`tests/build_time` instead of `tests/build-time`), then I ran the package on my binary copy, then the test alone. The cause was another agent rebuilding the binary mid-run, not my change.
2. **What I looked up that the brief could have given me:**
   - `nikaia test` has no name filter. Step 4's "by name" can't be done, so the brief could just say "run the whole project".
   - The package directory is `tests/build-time`, with a hyphen.
   - The brief did not say other agents might change `.nika` files during the run. Their lowered files then show up in `git status` and look like the drift that step 2 says should stop the brief.
3. **Checks run:**
   - Run and useful: the build before applying the patch, `lower-std` twice plus an md5 comparison to show the lowered files were stable, `cargo check nikaia-std --features compiler`, fmt, clippy on 1.97 (all targets) and 1.99.1 (`-p nikaia` only), the `it` filters, the whole `tests/compiler`, and `pre-push-check`.
   - Partly wasted: the single `it` re-run (3 minutes). Running `tests/build-time` on my binary copy had already shown the failure was not mine.
   - One correction: I wrote `Gate: green` in the report, but I never ran the gate. That field only reflects the cheap checks, so it should be read as "not run".
4. **One change that would save the most:** builder.md should say to snapshot the lowered files' md5 right after the fixpoint, and to compare later drift against that snapshot. Drift that appears after it, beside `.nika` files another agent has modified, is theirs: leave it unstaged and do not stop. The brief's step 4 should name the real commands (the whole `tests/compiler`, `tests/build-time`) instead of name filters that don't exist.
