1. **What cost the most.** The run was short, about 20 turns. The biggest cost was one wasted cycle. The cycle line's first `lower-std` hid the rustc errors, and only `cargo check -p nikaia-std --features compiler` showed the two clashes. One clash was the CheckWall duplicate: GR re-landed between the brief and my run, and the brief still told me to write my own copy. The other was the EitherText→PathBuf error. A second smaller cost was working out that `emit_core_leaves.rs` drifted because of another agent's uncommitted `.nika`, not because of anything I changed.

2. **What the brief could have handed me.**
   - That GR had re-landed (`grammar_run.nika` :28 already has `impl Error for CheckWall`).
   - Which Rust callers of `rustc`/`toolchain`/`target_libdir` remain, which settles whether they are forwards or stay as they are. The brief's "Still to build: forwards `rustc`, `toolchain`, `target_libdir`" contradicted its own *Stays Rust* list.
   - The fact that 13177b7c was rebased to 03c31a6a, so its pre-flight hash check fails.

3. **Checks I ran.**
   - Useful: the pre-flight greps, `lower-std`, `cargo check` on both crates, and a plain `nikaia run` probe that reproduced the defect.
   - Wasted: the `cargo build` in the cycle. It could only fail once `nikaia-std` failed to compile, and running `cargo check --features compiler` first would have skipped it.
   - I also built the wip commit through a separate index. That kept the shared staging area clean, but neither the brief nor builder.md covers it, so the supervisor should confirm it is acceptable.

4. **The one change that would have saved the most.** Re-run briefs should require a step right after the cherry-pick: run `lower-std`, then `cargo check -p nikaia-std --features compiler` on the current HEAD. Add an explicit line saying whether the builder may fix a std or compiler defect of a few lines as its own commit. Here the fix was six verified lines, and stopping costs a whole extra round.
