1. **Cost.** The biggest cost was the test program. I ran `nikaia run FILE` inside `tests/compiler` and was refused, because a test file isn't an entry point. Then I got the closure's return type wrong twice: first `1`, then `Some(1)`. Only `null` in an `if` expression works. The lowering cycle (lower-std, build, lower-std, build again for the fixpoint) was the other main cost, but it was necessary.

2. **Lookups the brief could have given.**
   - How to run a single test program: put it in a scratch project with its own `nikaia.toml` and `src/main.nika`, then `nikaia run`.
   - `nikaia test` has no name filter.
   - A `filter_map` closure returns `T?` and the empty case is `null`. `project_check.nika:442` shows this, but I only found it by grep.
   - The exact fix location was given. I would still have liked to know whether the file was meant to be formatted or kept as the long line.

3. **Checks.**
   - Ran: the `fmt::` filter (40 tests, including the scan), the guard set, clippy beside the tests, the new program run by hand, and fmt idempotence on copies of both files. All were useful.
   - Wasted: the first `nikaia run` calls, and the attempt to filter `nikaia test`.
   - Skipped: running the new test against the unfixed formatter (needs the binary rebuilt without the fix) and `nikaia test --both-settings --project tests/compiler`. I also did not look at `pre-push-check.sh`'s exit status.

4. **Brief and builder.md change.** Add one line to `builder-facts.md`: "To try a test program in seconds, copy it to `<scratch>/src/main.nika` with a minimal `nikaia.toml` and `nikaia run`; `nikaia test` takes no filter." Also state how to get a failing-before result without a rebuild, for example keep the pre-fix binary as `$S/nikaia.old` and run the new test with it.
