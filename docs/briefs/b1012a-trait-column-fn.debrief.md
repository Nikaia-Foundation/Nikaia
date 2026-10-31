1. **What cost the most:** working out what state I was starting from. The brief says every file modified at the start is not mine, yet `keeps.rs` and the test files for exactly this brief were already in the tree. It took about 6 turns of reading the driver log, the file times, the earlier run's scratchpad and the diffs to see that a killed run of this same brief had written them. The longest wait was the 10-minute `tests/compiler` run. Finding an unfixed binary took 3 more turns: there was no `target/release/nikaia`, and the toolchain tests fall back to a relative `target/fast/nikaia` under `process::run(dir: tmp)`.
2. **What I looked up that the brief could have given me:**
   - Whether `TraitMethod` has a default body (it has none, so "the default body's evidence" never applies).
   - How to run the toolchain tests (f) and (i) against an older binary.
   - That this was a resumed run. The driver knew it ("killed by time limit during b1012a") but the brief didn't say.
3. **Checks run and checks wasted:**
   - Run: the `lower-std` fixpoint cycle, `tests/compiler` with `--both-settings`, the six plain programs and (f)/(i) on the old binary, the `contracts` filter plus the guard set, clippy, `--lib contracts::`, `pre-push-check` and the badge.
   - Wasted:
     - Two failed tries of (f)/(i) on the old binary (relative toolchain path).
     - The `tail --pid` wait helper, which ran into its time limit after the test run had already finished.
     - One `--lib` link that failed on stale incremental files, which `CARGO_INCREMENTAL=0` fixed.
   - Nothing ran twice in full.
4. **The one change:** when the driver restarts a killed brief, it should add a line to the brief: "resumed: the previous run's files X, Y are yours, binary built at T", and keep a pre-change binary in the scratchpad for the unfixed-compiler runs. Without that line, the "never stage files dirty at start" rule contradicts the task. I staged those files on my own judgement, and the supervisor should confirm that was right.

The push was rejected, so 0df2eefc still needs your rebase and push.
