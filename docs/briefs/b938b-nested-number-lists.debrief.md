1. **Most cost:** the one `tests/compiler --both-settings` run, which was long, plus one build cycle. It ended with 7 failures that I could not cheaply isolate. Another agent's uncommitted `text_tiers.nika`/`.rs` was in the shared tree and so in my binary. I put the failures down to that file but never proved it. That left the gate verdict shaky, and that was the real cost. The code itself took few turns, because the brief's *Locations* were accurate.

2. **Looked up that the brief could have handed me:**
   - how the `for` builds its frame (`lent`, `CheckLocal.id`, `bound = view_of_type(..)`, so `r`'s type is a view and needs `.unseen()`);
   - the function signatures of `check_core_stmt_for` and the `let`;
   - `number_list_name` and `open_items_of`, which only accept a `Variable`;
   - that the flow has one constructor, at `check_flow.nika:~405`, and no Rust one.

   The brief said 5 `CheckOpenNumber {` literals; there are 3. I also guessed that the bound type of `[[1,2],[3]]` is `Vec[Vec[?]]`; I never looked it up, but the probes confirmed it.

3. **Checks I ran:**
   - the probe matrix on the old binary (useful: it doubled as the "fails unfixed" proof);
   - the old-binary `lower-std`, the build, then `lower-std` again;
   - the probes on the new binary, plus extra shapes;
   - `fmt` on the test;
   - the touched `it` modules plus the guard set, with clippy beside them;
   - `pre-push-check`, run twice: the first run piped through `tail` showed nothing, so a second was needed to see the exit code (wasted);
   - `tests/compiler` at both settings.

   Also wasted: one probe written with `i16`, which Nikaia does not have.

4. **One change:** builder.md should say what to do when `git status` shows another agent's uncommitted `tools/*.nika` or `lowered/*.rs` at the start. That file gets compiled into your binary and makes the `tests/compiler` verdict impossible to attribute. Either the brief lists the failures already known on that tree, or builder.md gives a cheap way to check one: rerun the failing test with a binary that does not contain the foreign file.
