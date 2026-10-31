Result: done
Files: crates/nikaia/src/parser/mod.rs tests/compiler/tests/a_shift_in_a_guard_or_a_head.nika tests/compiler/tests/a_shift_in_a_guard_or_a_head.stdout tests/compiler/tests/a_bit_operation_in_a_head_wants_parentheses.nika tests/compiler/tests/a_bit_operation_in_a_head_wants_parentheses.stderr
Commit: parser: a guard and an if, while or match head read shifts and bit operations (#939)
Body: The head chain (head_cmp -> head_add) skipped the four bit and shift levels; head_bit_or, head_bit_xor, head_bit_and and head_shift now sit between head_cmp and head_add, mirroring bit_or_expr..shift_expr with the same not() guards and calling only head rules beneath them, so a struct literal's brace stays refused. NK1197 needed no change. The brief expected `eight`/`yes` for `m == 1 << 3` and `m >> 1 == 4`, but a `let` refuses both with NK1197 (ADR-285 D9 covers the shifts too), so a head now does the same: those cases are in the refusal test, whose .stderr is the real output. The positive test uses a shift and a bit operation in a match head, where no comparison stands beside them, plus parenthesised forms in a guard and a while.
Fixes: #939
Tests: both new tests failed on the unfixed compiler (parse errors "Didn't expect `>`" and "Expected `=>`, found `<`") and pass now; whole it binary 2886 passed 0 failed; tests/compiler 429 passed 1 failed: a_group_that_throws_a_called_error_keeps_its_channel (NK2302 on main.nika's `places`), which passes when run alone with the fixed and the unfixed binary and has no head with a bit operator, so it is not this change; clippy clean; guards green (sequences ok)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A guard and an `if`, `while` or `match` head read `<<`, `>>`, `&`, `|` and `^` as a body does (#939); beside a comparison they get NK1197, as in a `let`, instead of a parse error.
Owner: none
Disk freed: 0 GB
Rs2nika: none
Pushed: b6c7c3b78c76589e9e358cdd92fe37d36489aa33
