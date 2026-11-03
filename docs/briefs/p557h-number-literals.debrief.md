1. **What cost the most turns:**
   - The three older `parser_grammar_*` tests broke once the grammar called `number_at`, `float_end` and `ledger.nika`'s `slice`. I found this only in the full `tests/compiler` run, then fixed all three and re-ran them, which took another cycle.
   - `float_end` as a lowercase rule skipped whitespace before its first element, so `1.5 G` was refused. That cost a debugging round.
   - The `(digit | "_")*` probe hit the known defect #1001 before I wrote it with alternatives.

2. **What the brief could have handed me:**
   - Tools files share one namespace, so a test that copies `parser_grammar.nika` also needs `parse_numbers.nika` and `slice`.
   - The API of `parse_numbers.nika` (the `Number` enum, `number_at`, `float_end`).
   - An action that throws gets normal priority (`Diagnostics::external`), while the Rust refusal used `PRIO_STRUCTURAL`.
   - The `it` filter `errors::` is what runs `tests/errors`.
   - The test runner's `root()` only works when run from the repository directory.

3. **Checks I ran:**
   - The bound-peek probe, then `lower-std` and its fixpoint.
   - `it` filtered on parse, grammar, span, range_spelling and shipped_ledger (163 passed), and `errors::` (38 passed).
   - `tests/compiler --both-settings` (591 passed, 3 failed), then the 4 parser tests re-run singly.
   - clippy, `cargo fmt` and pre-push.
   - Partly wasted: the full `tests/compiler` run. Grepping for the tests that copy `parser_grammar.nika` would have found the 3 breakages in seconds.

4. **Found in the tooling:** `commit.nika` writes `Fixes:` into the message but dropped `Part: #557`. I put it back with `git commit --amend`, which changed only the message of my local, unpushed commit. I also corrected one wrong phrase in the body ("wrong character" for "offending character").

5. **One change that would save the most:** the brief should list the files that copy or consume the file being changed (here the three older `parser_grammar_*` tests) and the new cross-file dependencies (`parse_numbers.nika` and `slice`). Alternatively, builder.md could say: before the suite, `grep -l` the tests that read the file you change, and run them first.
