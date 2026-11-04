Result: done
Files: tests/compiler/tests/async_parse_an_unknown_option.nika tests/compiler/tests/async_parse_an_unknown_option.stderr tests/compiler/tests/async_parse_detached_without_a_lambda.nika tests/compiler/tests/async_parse_detached_without_a_lambda.stderr tests/compiler/tests/async_parse_fmt_formats_every_form.nika tests/compiler/tests/async_parse_fmt_formats_every_form.stdout 
Commit: tests: `nikaia fmt` formats every `async` form and is idempotent; parse tests for `async(detached)` without a lambda and an unknown option (ADR-async-5)
Body: Steps 5 and 6 of #984. `tools/fmt.nika` already formats every form a1b parses (`async f()`, `async { … }`, `async(detached) fn { … }`, `async` in a call's arguments) and a second run changes nothing, so this step adds tests only: a program that writes a badly spaced file of all forms, runs `nikaia fmt` on it twice and prints the text and that both runs agree; and refusal tests for the two forms a1 left without one (`async(detached)` before something that is not a lambda, an option other than `detached`). Step 3 is 836268c1; a1 and a1b built steps 1, 2 and 4.
Fixes: #984
Pushed: local 694f75a14d0d24bb2cf6922b39b695771056dff0
Tests: the 3 new programs and the 6 of step 1 pass with nikaia test --both-settings on a scratch project; whole it binary 2887 passed 0 failed (sequences guard included); clippy clean; pre-push green; lower-std fixpoint holds; tests/compiler full run and tests/errors left to the driver
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: Two new refusals for async (#984): `NK2115` for `async` before a `let`, a loop or a function's declaration (whether a function pauses is inferred), `NK2120` for an `async` form inside a loop's body, whose help names `xs.par_iter().for_each fn(x) { … }`.
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
