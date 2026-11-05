Result: done
Files: crates/nikaia/src/parser/mod.rs crates/nikaia-std/src/tools/project_check.nika tests/compiler/tests/a_minus_that_begins_a_line_begins_an_expression_after_catch.nika tests/compiler/tests/a_minus_that_begins_a_line_begins_an_expression_after_catch.stdout tests/compiler/tests/a_minus_that_begins_a_line_begins_an_expression_in_a_block.nika tests/compiler/tests/a_minus_that_begins_a_line_begins_an_expression_in_a_block.stdout tests/compiler/tests/a_minus_that_begins_a_line_subtraction_broken_after_minus.nika tests/compiler/tests/a_minus_that_begins_a_line_subtraction_broken_after_minus.stdout
Commit: parser: a `-` that begins a line begins an expression (ADR-390)
Body: The binary `-` of add_op (shared by add_tail and head_add_tail) is now the rule subtract_minus = same_line "-", as `[` and a call's `(` already are; the `??` fallback's not("-") and its error arm use the same rule, so `a ?? 0` followed by a line `-1` is two statements, while `a ?? 0 - 1` is still refused. `-=` unchanged. The leading-`-` chain in project_check.nika now ends each line with `-`; its lowered file is unchanged (lower-std wrote nothing). No other program in crates/nikaia-std, packages, examples, tools or scripts begins a line with a subtracting `-` (only unary `-1` values). Rust added: 4 grammar lines in parser/mod.rs (#557 / #125).
Fixes: #1247
Pushed: local 4919b0bf7d77141da2f1eec77a3c42100873daec
Tests: new tests/compiler a_minus_that_begins_a_line_* 3 pass --both-settings; the two begins_an_expression tests failed on the unfixed compiler (rustc "Cannot subtract {integer} from ()", NK1103), the broken-after-minus one passed there too (guard); it coalesce_fallback:: coalescing:: jumps:: 31 ok; tests/compiler --filter coalesce 7 ok; lower-std clean, fixpoint (no lowered change); nikaia check packages/json, packages/yaml ok; clippy, fmt clean; disk: cache target/nikaia-project-tests 3.6G, / 4.2G free, new cache entry over 100 MB run-bundle/c1ccf85af4afe566 309 MB
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: Parser: a `-` that begins a line begins an expression (ADR-390): a block ending in a line `-1` has the value `-1` instead of subtracting from the line above; a subtraction is broken after its `-`.
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
