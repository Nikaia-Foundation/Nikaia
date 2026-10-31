1. **What cost the most.** Tracing where a function's `keeps` list gets computed for #1034 step 2. `where.nika FnContract` found the struct but not where its field is filled, so I grepped `keeps.nika` in about six rounds before I reached `hand_over`, `reached_of` and `keeps_named_by_a_lambda`. The second cost was the side issue `same(inc)(1)`: I searched Part I for a grammar rule that would allow or refuse it, found none, then tried GitHub search to check for a duplicate and found it isn't available. In all, about eight turns.
2. **What the brief could have handed me.**
   - That the batch had just finished, and the three report paths. I found them with `git log` and `ls`.
   - That the bug lane holds the `wip` issues; I had to work out that #999 is the yaml lane's from b1035's `Needs:` line.
   - That issue search isn't available headless, so duplicates can't be checked.
   - That Part I has no grammar section for postfix calls, so there was nothing to find.
3. **Checks run.**
   - Useful: the status line (CI in progress, batch done), re-running #1034's step-2 shape and its variants, the model probe (`return f` and `return Box2 { run: f }` do run, which pointed at `keeps.nika`), `check` on the combined test, and re-probing b938b's rows.
   - Partly wasted: the two `same(...)(1)` variants that hit the parse error. They did produce #1042, but they weren't this batch's work. The GitHub search calls and the Part II grep were wasted.
   - Also wasted: appending `common.part` and then a second Attribution block, which needed a repair turn. `common.part` already ends with Attribution, and nothing says so.
4. **The change that would save the most.** In `docs/architect.md` *Briefs and reports*, state that `common.part` already ends with the Attribution block. In the architect-task brief, list the finished batch's report paths and the issue numbers each brief left open, so the architect starts from those instead of rebuilding them with git and `ls`. Teaching `where.nika` to find where a struct field is written (`NAME.field =` / `field:` in literals) would have saved the `keeps` hunt.
