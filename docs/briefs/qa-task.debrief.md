1. **Time sink.** The `cargo build --profile fast` ran past the 120 s tool timeout and went to the background. I then wrote a wait loop with `pgrep -f "cargo build"`, which matched its own shell and hung for 10 minutes. The `pkill` I used to clear it killed my own shell as well. That cost about 3 calls and several minutes. The fix itself was trivial.

2. **Looked up.**
   - The brief doesn't say how to run `scripts/commit.nika`. It isn't executable, and I had to find out that it runs as `target/fast/nikaia run scripts/commit.nika -- REPORT`.
   - I had to work out that the stale lowered file was already modified in the working tree. A previous gate run in the same checkout had regenerated it, so the "fix" was just committing it.
   - I read one builder report (b949) only to see the report format.
   - `qa-input.txt` named the drifted file but not the commit that caused it. I never confirmed 316e0721 as the cause. I inferred it from `git log -S TrustedMap`, so the Cause line in my report is unverified.

3. **Checks.**
   - Useful: `git diff` of the lowered file, `lower-std` rerun to confirm it changes nothing more, the build, and `pre-push-check.sh`.
   - Redundant: the first build, because the binary at `target/fast/nikaia` was already built and the drift was visible in the diff. The `cargo fmt` I ran was also pointless, since only a generated file changed.
   - Skipped: clippy, because no hand-written Rust changed. I didn't rerun any `nikaia test` programs, because the failing step wasn't a test.

4. **Suggested change.** Add a "known recipes" block to `qa.md`:
   - "Lowered drift: run `target/fast/nikaia lower-std`, check the file diff, commit it (no rebuild needed if the binary is current)."
   - "Run scripts with `target/fast/nikaia run scripts/X.nika`."
   - "Never wait on a build with `pgrep -f`."

   `qa-input.txt` should also name the last commit that touched the drifted file's `.nika` source or the lowering.
