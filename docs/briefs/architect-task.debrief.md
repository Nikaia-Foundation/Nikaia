1. **What cost the most:** deciding #1044's rule. To settle "clone or refuse?" I probed a map, a `ref` field, `Vec[String]` and an owned local, and grepped Part I and Part III for the copy rule and `NK1131`. That was about 15 of roughly 35 turns. Wrong probe syntax cost three more: `var`, `HashMap` without its module, `HashMap::new`, and `.to_string()` on a literal.
2. **What the brief could have handed me:**
   - That `docs/briefs/b859c-type-argument.md` belongs to another session. It sits in my folder untagged; I learned whose it was from `git log`.
   - That #1047 already has its options written out. My first *State* line said it had none, which took a second commit to fix.
   - That `architect.md` is over the read cap, so *State* has to be read by line offset.
   - Probe syntax: map construction and `use std::collections`. This belongs in builder-facts as a ready-to-paste block of correct forms.
3. **Checks:**
   - Useful: the `nikaia run` probes of rows a–g, `nikaia lower` to see the lowered `&[Sym]`, the grep for `view_of` assignments, and the `ref Box2` probe, which turned up the `NK1131` model.
   - Partly wasted: the full open `ready` issue list (long output, I used three entries), the `where.nika NK1104` call, and the oversized `NK1131` spec grep that spilled to a file.
4. **The one change:** have `drive.nika` write a "lane owner" line for every brief in `docs/briefs/` that is not in my `batch.txt` (or a `owner:` key in brief frontmatter). Also have the task include the issues filed since the last architect run, with label and whether a decision paper exists. That would have saved the b859c digging and the #1047 correction. Second choice: a builder-facts block of correct probe snippets (map, set, owned list, `?.clone()`).
