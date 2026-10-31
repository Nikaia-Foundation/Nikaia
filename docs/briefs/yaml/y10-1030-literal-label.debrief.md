1. **Most cost:** the `tests/compiler --both-settings` run took over 10 minutes of wall time. In turns, the costliest part was finding that the brief's cause was only half the fix. After the build, the checker's arm alone turned the rustc error into NK1104/NK1106. It took a grep and two reads to find the second place, `grammar_reference_kinds` in `text_tiers.nika`, which decides whether a rule's result is a view. That meant a second build cycle.
2. **Looked up:**
   - How a test builds `Bytes` from a literal (`Bytes("ab".bytes().collect())`). The brief's probe implied `Bytes("ab")`, which is NK1102.
   - The `Pattern::Literal(_)` variant's shape.
   - That `text_tiers.nika` mirrors `pattern_type` for result tiers.

   The brief could have said: "the text(..) capture is typed in two places, `check_grammar.nika` `reference_type` and `text_tiers.nika` `grammar_reference_kinds`; add the arm to both."
3. **Checks run:**
   - The probe on the old binary: useful, and it doubles as the run against the unfixed compiler.
   - Two lower/build cycles: the first could have been saved.
   - The gate guards with the `grammar` it filter, clippy in parallel, and pre-push: all needed.
   - `tests/compiler` with `--both-settings`: required because `check_` changed.

   Nothing else was wasted. A slip in backgrounding (`&&` before `&`) lost `$S` in the foreground shell, but it cost nothing.
4. **One change:** in builder-facts' grammar trap line, add: "a label's *view/owned tier* comes from `text_tiers.nika` `grammar_pattern_kinds`; a change to `pattern_type` needs the same arm there". The brief's file list should then include `text_tiers.nika`, so the change is no deviation. Also, `builder-facts.md`'s *Red on develop now* line about `sequences` is stale: it passed here.
