1. **Most turns and time.** The failure line only named the test, not the offending call. I had to add temporary debug prints and then bisect `sysroot_package.nika` through about five 55-second release test runs. A debug build also failed on a recursion limit, which cost one detour before I switched to release.

2. **What the brief could have handed me.**
   - The test's output holds only a count, so the useful hint was "bisect by truncating the suspect file, because `unanswered` keeps no names."
   - The file named in the log was the file the new commit touched, so "check the sysroot_package file first" would have been enough.
   - I had to look up that `lower-std` must be run after a `.nika` change, which binary to use, and that `nikaia run` refuses files inside a project.
   - I also looked up that CI builds release, and that a debug build fails locally.
   - I did not know the shared checkout holds other agents' uncommitted `emit_core_*` edits. I had to check they would not collide with my `lower-std` run.

3. **Checks.**
   - Useful: the release ceiling test, the 549-test `tests/compiler` run, the sysroot tests and `pre-push`.
   - Wasted: the debug-build attempt, and the second compiler test I wrote. It could not compile in Rust (mixed types in an or-pattern), so I deleted it.
   - Weak: the kept test passes without the fix, so only the ceiling test guards the change.
   - Skipped: clippy, because only generated Rust changed.

4. **One change.** Make the ceiling test print which file and which method call is unanswered when it fails, or add a note to `qa.md` saying "bisect by truncating the file and rerun with `--release`." That would have saved most of the turns.
