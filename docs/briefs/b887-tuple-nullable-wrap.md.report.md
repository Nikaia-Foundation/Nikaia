Result: done
Files: crates/nikaia-std/src/tools/check_overflow.nika crates/nikaia-std/src/tools/check_texts.nika crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered/check_overflow.rs crates/nikaia-std/src/tools/lowered/check_texts.rs tests/compiler/tests/a_plain_value_in_a_tuple_at_a_nullable_part_is_wrapped.nika tests/compiler/tests/a_plain_value_in_a_tuple_at_a_nullable_part_is_wrapped.stdout
Commit: check: a plain value in a tuple literal at a T? part is wrapped
Fixes: #887
Tests: new program test a_plain_value_in_a_tuple_at_a_nullable_part_is_wrapped (argument, let, return, local, null, already T?, nested tuple) failed on the unfixed compiler (8 rustc mismatches), passes after; nikaia test --both-settings --project tests/compiler 172 passed; nikaia-std --lib check_ ok
Gate: green
Deviation: none
Found: `fn f(p: (String?, i64)) -> String { p.0 ?? "none" }` passes the checker, rustc refuses "cannot move out of p.0 which is behind a shared reference" (a `??` on a part of a lent tuple parameter moves it); scripts/commit.nika looks for the brief at <report minus .report.md>.md, so a report named <brief>.md.report.md (as this brief asks) loses the Attribution block (worked around with Model:)
Needs: none
Changelog: Fixed: a plain value in a tuple literal at a `T?` part (argument, `let`, `return`, nested tuple) is wrapped as `Some(..)`, as it is when passed to a `T?` parameter (#887).
Owner: none
Model: Opus 5.5
Disk freed: 0
Pushed: 6e7816cdba9c75331c8eb6715ab2632ea0df493c
