1. **What cost the most:** designing P5d's and P5b's hosts. That meant reading about 900 lines of `project.rs` (the check and explain block, `lower_reading`, `report`), then checking the Nikaia side one item at a time: `CheckFinding`, `Newly`/`Kept`, `Published`, the ledger parser, `changed_contracts`, whether `eprint` exists. Close behind was finding that host calls already have a mechanism (`rs2nika.names` `[forms X_]`/`[hosts X_]`, R244/R295). I nearly wrote a converter brief for "wanted pattern (3)" when it was already learned.
2. **What the brief could have handed me:**
   - Current line ranges. The plan's `Cut:` lines were stale (at a4b33a0), so I re-grepped every range.
   - That `[forms X_]` already covers the "named Rust call becomes a host method" pattern. `docs/architect.md` lists it as *wanted*, which misled me.
   - How free disk space compares with `agent.nika`'s 3 GB limit.
3. **Checks run:**
   - **Useful:** the converter's draft of P5d (it showed P5d is almost all host calls), `where.nika` for callers, the open-issue list, and `git status` for other lanes' files.
   - **Wasted:** `rs2nika --score` with one argument; I got the usage wrong.
   - **Not done:**
     - I did not check that `[forms]` can match a call with seven arguments, nor the `it` filter names in either brief.
     - The #943 issue body says `Fixes #N` instead of `#943`.
     - The CI query showed every run of today's pushes still `in_progress`, so I checked nothing about their result.
4. **One change that would have saved the most:** have `architect-input.txt` (or `drive.nika`) carry the current line ranges of the next plan rows, from a `scripts/plan_rows.nika` (still owed). Also fix the *Schematic patterns* paragraph in `docs/architect.md` to say host calls are learned through `[forms X_]` (P4b). Together that would have saved about a third of my turns.
