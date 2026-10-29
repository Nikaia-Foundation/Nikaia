1. **What cost the most:** designing the M3b host traits. I read about 255 lines of `modules.rs` and grepped `emit/mod.rs`, `contracts/mod.rs` and `ty.nika` to learn which types are already Nikaia's (`Ledger`, `Provenance`) and what `emit_module_body_at` takes. That was about 8 of my roughly 25 turns. Second: my first Python edit script failed because I had guessed the exact `CLAUDE.md` line wrong, which cost a re-grep and a second run.
2. **What I looked up that the task could have handed me:**
   - The latest *State* entry. It sits under 30 older entries in `architect.md`, and the 25k read cap cut off the rest of the file.
   - That `crate::contracts::{Ledger, Provenance}` are re-exports of `ty.nika`'s.
   - Whether #933, #934 and #921 were still open.
   - The current line ranges in `modules.rs` after M3a.
   - That `batch.nika` cuts only on `Found:` and `Deviation:`, not on `Needs:`.
3. **Checks:**
   - Useful: the M3a report and debrief, the reflog (it confirmed the reset), `git log` on `manifest.rs` (it confirmed MF's range still holds), the issue states through `gh api`, and `pre-push-check` before the push.
   - Wasted: reading the 30 older *State* entries; one grep over `it` tests for callers, which the brief only lists because the signatures stay the same.
4. **Change that would have saved the most:** `architect-input.txt` should carry the newest *State* paragraph and the cut report's path and debrief. Alternatively, `architect.md` should move older *State* entries to a history file and keep only the last three, so one read covers the state.
