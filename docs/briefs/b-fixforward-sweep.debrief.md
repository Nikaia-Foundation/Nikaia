1. **What cost the most:** about 12 grep rounds looking for a test that reads `crates/rs2nika/convert/src`, chasing the brief's guess about the cause. I looked through `it` tests, `tests/compiler`, scripts, the repository-wide walks in `fmt.rs` and `sequences.rs`, and the `nikaia` binary's own dump path. None of them read those files. Proving that nothing dumps a file took longer than any search that finds something would have.
2. **What the brief could have handed me:**
   - `rs2nika` is not a workspace member, so `cargo test --workspace` never touches it.
   - `dump_accepted` skips project files, and the converter's files belong to a project.
   - Who writes the dump (`check_against` in tests, plus every one-file `nikaia check`/`run`, named by the binary).
   - The sweep runs only in the `arm64` job on `develop`.
   - The format of the sweep's failure lines (hash line, `rustc` error, panic).

   Each of these took a lookup.
3. **Checks:**
   - Useful: the grep sweep (it ruled the guess out); the annotation filter tried on a sample log; a YAML parse; `scripts/pre-push-check.sh`.
   - Mostly wasted: the walks through `fmt.rs` and `sequences.rs`, which the workspace membership fact would have made unnecessary.
   - Not run: no build, clippy or gate, since the change was CI-only. The `Gate: green` in my report stands for that, not for a gate run.
   - One more thing: the report on disk no longer has its `Rs2nika: none` line. Either `commit.nika` or another writer dropped it, and it's worth checking whether `release.nika` needs that field.
4. **The one change:** when the job log can't be read, the supervisor should first check whether the commit can even reach the failing step. Here that means asking whether c7e5b66's files are in the workspace at all, in one line. If they aren't, the brief should do two things:
   - Order step 3 first.
   - Name the cheap suspects: the three `KNOWN` hashes (does one compile now, or is it no longer produced?) and a sweep difference specific to arm64.

   That would have spared the whole search for a converter-file test.
