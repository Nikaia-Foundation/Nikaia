1. **Time and turns.** The long waits were the `lower-std` and build cycle, plus the clippy and `it` runs. In turns, I lost two to a scratchpad that did not exist yet. My first `cp` and `git show > $TMPDIR/...` failed quietly and I only saw it when `git apply` said the patch was missing. A third turn went on a blocked `rm /x.diff`, caused by the empty `$TMPDIR`.

2. **Lookups the brief could have handed me.**
   - The brief did not say that `type_rules.nika` and `type_rules.rs` were unchanged on `develop`, so I ran `git diff --stat` to find out. With that fact the patch could have been applied directly.
   - It did not say that the lowered code trips `clippy::extend_with_drain`. I found out when clippy failed, then looked up the allow list in `lib.rs`.
   - It did not say that the badge recipe needs the files staged first. `checkout-index` reads the index, so I had to stage and then regenerate.
   - The `it` module names were not given. I grepped for them, and my filter matched only 16 tests.

3. **Checks run.**
   - **Useful:** `lower-std` (twice), the build, `cargo test --lib type_rules`, the `it` filter run, the six `tests/compiler` programs with *rule* in the name, clippy, and `pre-push`.
   - **Wasted or weak:**
     - `cargo test --lib type_rules` matched 0 tests.
     - My first `checkout-index` ran before staging, so it did nothing useful.
     - `pre-push` still showed `version red`, which is the supervisor's line, so I left it.
   - **Not done:** I did not confirm that the `it` filter covered the type_rules tests, and I did not re-run `lower-std` after the `lib.rs` change. I also never probed the fails-before case, since this is a move and not a fix.

4. **One change.** Add a "Preflight" line to the brief template: create `<scratchpad>/<brief>/` first, and say whether the touched files changed on `develop`. For a move onto `develop`, also list the known clippy allows and the exact `it` module names.
