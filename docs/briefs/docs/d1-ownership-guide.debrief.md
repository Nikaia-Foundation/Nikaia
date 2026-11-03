1. **What cost the most.** Probing which program shapes actually compile. Literal text acts differently from text the program builds: `.clone()` on a literal is refused (NK1301), and `let mut y = "short"` hits a wrong NK2105. A struct with an `int` field was refused with NK2302. Narrowing these to the right examples took about 10 of roughly 45 turns. Top-level snippets not running (the tour's blocks are fragments) cost one more round.
2. **What the brief could have handed me.**
   - Which spellings work today: `ref mut self` works but `mut self` does not parse, `self` takes the object over, and there are no top-level statements.
   - That example strings should come from `to_uppercase()` or an f-string, not a literal.
   - The CHANGELOG range to compare: `syntax.md` was last changed today, so "since the last change" was empty, and I had to choose 0.0.1228 myself.
   - That `command-line.md` has no *Where next* list.
   - The spec's NK2301/NK2305 conflict.
   - The anchors for Part III §13.5, §15.1 and C.3.
3. **Checks.**
   - Useful: a small script that pulls out every `nika` block, runs it, and gives me the real output to paste; a link and anchor checker; `pre-push` before each commit; running the old and new spelling once to confirm the new one is refused.
   - Wasted: rerunning the loop and NK1102 examples that the syntax tour had already checked, and a few runs to find a list form the compiler accepts (the `split` and `best!` variants). I skipped `cargo fmt`, clippy and builds on purpose, since no Rust changed.
4. **The change that would save the most.** A short *Facts for docs briefs* section in `builder-facts.md`. It would list the receiver, literal and top-level traps; give a ready extract-and-run script for checking guide blocks (with `main.nika` names, so pasted errors match); and give the link checker. That would have saved about a third of the turns.
