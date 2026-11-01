1. **Time sink.** The two `it` and `nikaia_test` runs took about 6 minutes each and cost the most time. The first `it` run had no error text, so I had to reproduce a failure by hand. The `nikaia_test` package was run twice, once through cargo and once directly. The second run was partly wasted, because the same two programs failed and the cause was the machine's low disk.

2. **Lookups the brief could have handed me.**
   - Two input files existed (`docs/briefs/qa-input.txt` and `docs/briefs/bugs/qa-input.txt`). I had to compare timestamps to see that the bug lane's was the live one.
   - The log's "failed:" lines were junk from an awk filter. It never showed the real compiler error, which only appeared after I reran one test.
   - I had to work out how a `let mut` local appears in the checker (`immutable == null`).
   - I had to find the `lower-std` step in `docs/builder-facts.md`.
   - `nikaia test` has no `--filter`, and `nikaia run` refuses a file inside a test project. That cost two failed commands.

3. **Checks.**
   - Useful: the one-test cargo reproduction, the 21 targeted tests, `accepted_compiles`, `examples`, fmt, clippy and pre-push.
   - Wasted: the `nikaia test --filter` attempts, which failed on an unknown flag. The second whole-package run and the agent_script investigation added little, since low disk explains both failures.
   - I did not identify the single program that was red on CI in `nikaia_test`.

4. **One change.** Make the driver's `qa-input.txt` carry the first compiler error for each failed test. That means the panic text, or the `error[E…]` block plus the test's name. It should also name the unpushed commit that touched the failing area. That would have saved about half the investigation. Failing that, put in the brief or builder.md one line saying the log's `failed:` lines are noise, and that the fast way is `cargo test --release -p nikaia --test it -- <module>::<test>` and reading the `error[` lines.
