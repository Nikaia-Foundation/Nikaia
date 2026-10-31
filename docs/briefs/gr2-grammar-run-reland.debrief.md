1. **What cost the most:** the build plus two `lower-std` cycle, a few minutes. The first `lower-std` already showed `project_members.rs` drifting, and I could have stopped there. The A/B `lower-std` (with `grammar_run.nika` set aside, then put back) cost one more call. It was worth it, because it showed the brief's premise was wrong. The whole run took about 15 turns.
2. **What I looked up:** nothing for the brief's own steps. After stopping I grepped `TrustedMap`, `trusted_input` and `contracts/trust.rs` to give the architect a lead. The brief could not have given me that, since it assumed the drift was gone. Two small mismatches: the brief said `project_members.rs` was already modified when I started, but it was not. And `lower-std` also rewrites `runtime/lowered.rs`, which then picks up other agents' uncommitted `sysroot.rs` edits. Neither the brief nor the facts mention that.
3. **Checks I ran:**
   - Pre-flight build: needed.
   - Revert and conflict resolution: needed.
   - First `lower-std`: needed, and decisive.
   - `cargo build` plus the second `lower-std`: wasted, since the drift was already visible.
   - A/B `lower-std`: useful, because it pinned the cause.
   - No tests ran, so nothing wasted there.
4. **The change that would have saved the most:** turn the brief's claim into a pre-flight check before any revert or merge. Drop `grammar_run.nika` (from `git show b070adb9:...`) into `tools/`, run `lower-std` with the HEAD binary, check `git status` of `lowered/`, then remove the file. That is one call, and it would have shown in a single turn that b15's finding does not hold today. More generally, builder.md could say: when a brief rests on another run's finding, re-check that finding with the cheapest probe first. And a run that stops should also restore every lowered file its own `lower-std` touched, `runtime/lowered.rs` included.
