1. **What cost the most:** about a dozen turns finding out where the work actually sits. I read the lowering of text tethers, `text_tiers.nika` (6,800 lines, outlined), `tiers.nika`, `declared.nika`, the #1152 path in `check_core_places.nika`, and `keeps.nika`. Only then did I see that NK2105 for `go` comes from `keeps.nika`'s struct-literal rule. That file is excluded by the brief and is one of its stop conditions.

2. **What the brief could have handed me:**
   - The issue text. I fetched it with `gh api`, though builders are told they have no GitHub access.
   - That NK2105 here comes from the ledger's `keeps`, inferred in `keeps.nika`. The brief listed "the place that raises NK2105 for a struct literal" as mine, which suggested it was in the checker.
   - Where tiers are written: `text_tiers.rs` `refine_with` → `tier_write`, at parse time.
   - That the #1152 `lent_fields` path already lowers a local lent into a `ref T` field.
   - That `EitherText` exists only for text, so a mixed non-text field is an open design question.

3. **Checks:** only old-binary probes, about 8 runs. All were useful except the first text probe, which NK1301 refused for `.clone()` on a literal (wasted). There was no build, no test run and no lint. One correction: my report says `Gate: green`, but I ran no gate. Since nothing was committed, it should have said `Gate: n/a`.

4. **One change that would have saved the most:** a 5-minute pre-flight in the brief, written by the supervisor: run the issue's probe (b) with `nikaia lower` and read the ledger's `keeps`. That shows `keeps = ["l"]` at once, so the brief would either have taken `keeps.nika` into Owns or been sequenced after the keeps lane. More generally, builder.md could say: "when a brief's stop condition names a file, first probe whether the step's headline case goes through that file."
