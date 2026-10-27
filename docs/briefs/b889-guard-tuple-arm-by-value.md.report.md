Result: done
Files: tests/compiler/tests/a_guard_proved_name_beside_null_given_back.nika tests/compiler/tests/a_guard_proved_name_beside_null_given_back.stdout
Commit: tests: a guard-proved name beside `null` or a literal in a tuple arm is given up (#889)
Body: The emit fix was already on develop: 30798b2 (`return` walks its value as a function's tail does, so a proved name in an arm of `return match` is given up) fixes #889's probe, the `(x, 2)` form and a struct alike; the issue predates it. This commit adds the brief's regression test. Verified: with 30798b2's one line reverted (`a_narrowed_read_given_back(returned)` at check_core_stmt.nika:1385) the test fails three times with Mismatched types in rustc; restored, lower-std leaves lowered/ unchanged. Also probed by hand, all pass: the same arms under `let r = match`, `r = match`, `v.push(match ...)` and a statement `match` with `return` in each arm.
Fixes: #889
Pushed: ea62f5bc787ed42637ff312ce97c421d67918bba
Tests: new test failed on the unfixed compiler (30798b2's line reverted: 3x Mismatched types), passes now; nikaia test --both-settings --project tests/compiler 189 passed; it handed_over text_tiers typecheck narrow + guard set 219 passed; clippy clean; pre-push-check ok
Gate: green
Deviation: To build step 1 (emit) not built: already fixed by 30798b2, only the test is added
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0
Rs2nika: none
