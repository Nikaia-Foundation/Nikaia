1. **What cost the most:** finding the cause of y9's Found. I searched for where `-> String` turns into `&'a str` with three greps that found nothing (`'a str`, `view_rule`, `literal` in `check_grammar.nika`). I only found it by reading `reference_type`, then `pattern_type`, and seeing there was no `Pattern::Literal` case. I also first read `RED` in the `ls` output as the driver's input; it is an empty file. And I read most of `docs/architect.md` (about 36k tokens) when only *Briefs and reports* mattered.
2. **What the brief could have handed me:**
   - That a label's type in a grammar comes from `CheckState::pattern_type` in `check_grammar.nika`.
   - That `nikaia lower FILE` shows each rule's lowered type.
   - That Part II 10.8's table is around line 716 of `20-nikaia-advance.md`.
   - That #1004 (a temporary `Bytes(..)` argument) muddies any probe over bytes, so the input should be bound with `let` first.
3. **Checks:**
   - Useful: probes a–h (text vs bytes, `text(..)` vs literal, choice, optional), diffing the lowered `c.rs` against `f.rs`, grepping y2c's draft for literal labels (none), and checking `fixed-issues.py`'s pattern against my commit subject.
   - Wasted: probe b (it hit #1004, not the bug), the three empty greps, and grepping ADR-366 for "literal".
4. **The change that would save the most:** the architect task, or a lane line in architect.md *State*, should name the grammar pipeline in one line: checker `check_grammar.nika` (`pattern_type`), then emitter `emit_grammar.nika` / `emit_pattern.nika`, then `nikaia lower` to see the result, then `winnow-grammar` (pinned). The task should also say: "read architect.md's *Briefs and reports* only, not the whole file."
