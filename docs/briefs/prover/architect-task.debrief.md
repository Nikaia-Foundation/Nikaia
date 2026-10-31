1. **Most turns and time:** two things. Finding the D15 carrying path in the prover took about four reads of `prover_state.nika` and `prover_passes.nika` before the cause was clear. Writing p9's staging rule for `sysroot.rs` took two rewrites, because I first missed that `scripts/commit.nika` adds whole files.
2. **What I had to look up that the brief could have handed me:**
   - That p7 and p8 were already pushed (the old `state.md` said they were unpushed).
   - That the driver had cut the batch on p10's `Found:`, so p9 never ran (I had to read `drive.nika` to see what `CUT` means).
   - That `RED`'s timestamp was new while its contents were old.
   - That bug-lane b17 also edits `sysroot.rs`.
   - Where `count_checked_calls` and `PreClaim` live (`scripts/where.nika` would have given them).
   - That `var` does not parse; it is `let mut`.
3. **Checks run:**
   - Useful: the defect reproduction (check and run), the twin with a proved literal and two-step chain, the overlap check against the other lanes' batches, and the figure at both settings (about 90 s).
   - Wasted: my first reproduction failed on `var`, and a probe that removed `mut` only confirmed a misleading warning.
4. **One change to save the most:** the driver should write a short "since the last architect run" header into `architect-input.txt`. It would list the pushed/unpushed state of each lane commit, which briefs ran and which were cut, and the files the other lanes' current briefs touch. That would have replaced about eight of my lookups.
