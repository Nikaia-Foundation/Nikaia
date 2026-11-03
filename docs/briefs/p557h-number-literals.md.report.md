Result: done
Files: crates/nikaia-std/src/tools/parser_grammar.nika crates/nikaia-std/src/tools/lowered/parser_grammar.rs crates/nikaia-std/src/lib.rs tests/compiler/tests/grammar_bound_peek.nika tests/compiler/tests/grammar_bound_peek.stdout tests/compiler/tests/parser_grammar_numbers.nika tests/compiler/tests/parser_grammar_numbers.stdout tests/compiler/tests/parser_grammar_literals.nika tests/compiler/tests/parser_grammar_trivia.nika tests/compiler/tests/parser_grammar_words.nika
Commit: Parser grammar in Nikaia: the number rules FLOAT, FLOAT_END, number_lit, negative_number_lit
Body: Translates by hand the Rust rules `FLOAT`, `float_end`, `number_lit` and `negative_number_lit` (with `number_prefix` and `number_at`) into `tools/parser_grammar.nika`; the actions call `parse_numbers.nika` as the Rust ones do. `NUMBER_PREFIX` is Rust's `number_prefix` (`-` in front, `E+`/`E-`, ASCII letters, digits, `_`, `.`); `NUMBER_READ` binds it with a bound `peek` (#1201, winnow-grammar 7235ce46) and hands it to `number_at`; `NUMBER_AT` refuses `Number::Not` with a guard named `digits` and consumes `consumed` bytes with `any{n}`, so a refusal's caret stands on the offending character, as in Rust, and the action throws `parse_numbers`' words. `float_end` is `FLOAT_END` here: a lowercase rule skips trivia before its first element, so `1.5 G` came out refused as a float with a scale; in capitals it reads only what touches the digits, as the Rust extern rule does. One difference stays: an action's failure carries winnow-grammar's normal priority (`Diagnostics::external`), the Rust rules gave a refused number `PRIO_STRUCTURAL`; it only matters where another alternative fails at the very same offset, and `fail(..)` takes no computed message. The lowered file adds two clippy lints to the tools allow list in `crates/nikaia-std/src/lib.rs` (`let_unit_value` for a unit action that throws, `identity_op` for the bound `any{0 + n}`). `parser_grammar_numbers` runs the rules over ADR-285 D16's and ADR-322's literals and prints each refusal beside what `nikaia check` (the Rust grammar) says of the same literal: words and caret equal (the check's column is 12 to the right, after `let x = `). `grammar_bound_peek` is #1201's probe. The three older `parser_grammar_*` tests now copy `parse_numbers.nika` and `ledger.nika`'s `slice` beside the grammar file, which calls them; their output is unchanged.
Fixes: #1201
Part: #557
Pushed: local 8a23fc6dc2a87593a5ee40efdfe7f7b8538f7440
Tests: grammar_bound_peek and parser_grammar_numbers new (the bound peek is the dependency bump e265d24f, so not run against the old winnow-grammar; the number rules are new); it filtered parse/grammar/span/range_spelling/shipped_ledger: 163 passed; lower-std fixpoint holds (only parser_grammar.rs changed); tests/compiler --both-settings: 591 passed + the 3 older parser_grammar tests, which now also copy parse_numbers.nika and slice, re-run singly ok with unchanged stdout (594 total); tests/errors (it errors::) 38 passed; clippy clean; disk: project-tests cache 1.8G, 4.2G free, new cache entry rlib/af187e66 267 MB
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
