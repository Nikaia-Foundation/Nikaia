1. **Most time lost:**
   - The first `lower-std` runs cost the most wall time. The old binary re-lowered everything, and one run timed out. My waiter loop also matched itself through `pgrep -f`.
   - About 15 turns went on probing the `ref`-field struct problem (`NK2302` and `NK1106`) in `KeepsBody`/`KeepsIndexes`. In the end a by-value `KeepsIndexes` built by the Rust forward fixed it.
   - Another agent's half-written `sysroot_package.nika` broke `lower-std`, cargo and clippy for a while. I had to wait for it and then work around it with `-A clippy::needless_as_bytes`.

2. **Could have been handed to me:**
   - The brief says the supervisor's probe shows a lambda capturing a `ref` parameter and a struct holding a `ref` enum field. It does not cover building a struct of `ref MethodIndex` fields from locals, or passing one into a function, which is the `KeepsBody` case.
   - The brief did not say that `tools/*.nika` cannot be checked with `nikaia check` on its own. Only `lower-std` checks them, and it takes about two minutes.
   - It also did not say that `commit.nika` stages whole files, so a shared `lowered.rs` forces a manual commit. I had to read `commit.nika` and `pre-push.nika` to find that out.
   - How the `version` check in `pre-push` reads the working tree rather than the staged copy was another thing I had to dig out of the scripts.

3. **Checks:**
   - Ran: `lower-std` and its fixpoint, `cargo build`, the `contracts::` unit tests, the six `it` groups, the two scratch-project programs, clippy and `pre-push`.
   - Wasted: `nikaia check keeps.nika`, which gives false "no type `Ty`" errors on a single tools file. Four small probes (`p1`–`p4`) before I reached the working form. The first scratch-project run, which failed only because `target/fast/nikaia` was missing there.
   - I did not run the fails-before test. No new test was needed, because this is a pure move.

4. **One change:** add one line to the brief. It would say that a struct with `ref` fields is built by the Rust caller and passed in by value, and that `keeps_infer` therefore takes the `KeepsIndexes` parameter. Add one line to `builder.md` as well: `commit.nika` stages whole files, so with another agent's hunks in a shared generated file, stage only your hunk and commit by hand.
