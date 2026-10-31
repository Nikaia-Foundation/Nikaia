1. **Most turns and time.**
   - The first test run used the debug profile. It hit the 120s timeout and then a winnow recursion-limit compile error, which cost about three calls and several minutes before I switched to `--profile fast`.
   - Finding the emitter took about ten turns. I started in `crates/nikaia/src/emit`, and the grammar emitter is actually in `crates/nikaia-std/src/tools/emit_pattern.nika`.

2. **What I looked up that the brief could have handed me.**
   - The brief said `docs/briefs/qa-input.txt` or `docs/briefs/bugs/qa-input.txt`. The real file was in `docs/briefs/prover/`, a third lane, so I had to run `find`.
   - Tests must run with `--profile fast`. The gate's `cargo test` command is in `gate.nika`, and the brief could have quoted it.
   - Grammar emission is written in Nikaia under `crates/nikaia-std/src/tools/*.nika`, not in the Rust `emit/` directory.
   - `scripts/commit.nika` allows one report per commit, so two causes meant rewriting the same report file between commits.

3. **Checks.**
   - Run: the failing test, `badges.py --check`, `cargo fmt --check` and `pre-push-check.sh`.
   - Wasted: the debug-profile `cargo test` and `cargo build`, plus about six greps for the emitter in the wrong places.
   - I did not run clippy. That is fine, because I changed only a test file and badge output, but I could have said so in the report.
   - The seven `rs2nika` lock and proofs files were already modified when I started. I assumed the suite run produced them and left them alone. I never confirmed that.

4. **One change.**
   - Have the driver put the input directory, the exact gate commands (`cargo test --profile fast ...`) and each failing test's file path into the task text, or into the top of `qa-input.txt`.
   - Add one line to `qa.md`: "red on the `fast` profile only; the debug profile does not build".
   - That would have saved most of the lost turns.
