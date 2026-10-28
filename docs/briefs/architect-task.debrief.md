1. **What cost the most:** working out what the MF move should actually be. The plan row says "100 %, no prerequisite", but `manifest.rs` is mostly `toml::Value` glue. I had to read the whole file and count its callers (one grep loop over 14 names). Only then could I set the shape: a Nikaia `ManifestFile` behind Rust wrappers, with `rooted` and the Rust tests staying in Rust. Reading the 375-line `architect.md` whole, including about 45 old *State* entries, was the second largest cost.
2. **What I looked up that the task could have handed me:**
   - That `RED` with no job list means an empty status file, not a red CI. One `gh api` call showed CI green.
   - The lanes and which files each one owns (`bootstrap-week.md` section 3).
   - That the search API is blocked; only repository endpoints work.
   - That `project.rs` lines had shifted after P5a (`rooted` moved from 637 to 538).
   - That `crates/nikaia-compiler` does not exist yet (R0 has not run).
   - Where NK2605 is raised. `where.nika` found no definition for `check_crossings`, so I fell back to grep.
3. **Checks run:**
   - Useful: the CI runs list, the plan rows, the `git log` of `modules.rs` (to confirm M3a's line ranges still hold), and the caller counts.
   - Wasted: two GitHub search calls (blocked) and the `check_crossings` part of the `where.nika` query, which found nothing.
4. **The change that would save the most:**
   - The driver should write `RED <run id> <failing jobs>`, or nothing, into `architect-input.txt`, never a bare `RED`.
   - `architect.md`'s *State* should keep only the last three entries and move older ones to an archive file. That alone would have cut my reading by about a third.
   - For MF itself: plan rows should say whether a range is decision logic or glue around an external crate, so the "100 %" draft share is not misleading.
