Result: done
Files: tests/compiler/tests/fn_type_string_param_takes_a_named_function.nika tests/compiler/tests/fn_type_string_param_takes_a_named_function.stdout
Commit: tests: a named function with a String parameter fits a function type of its signature
Body: Coverage for #1231, no fix: its probe (`fn tag(s: String, n: i64) -> String`; `let g: fn(String, i64) -> String = tag`; `g("ab", 2)`) prints `abab` on the current binary and on the release binary of 2026-11-02, so it does not reproduce on either. The commit for #1230 is 6256975d; no Rust function added or changed by hand in either commit.
Fixes: #1231
Tests: fn_type_string_param_takes_a_named_function ok (both settings); does not reproduce before (v7 probe on the 2026-11-02 release binary prints abab), coverage only; #1230 commit 6256975d: views_trait_from_lifetime ok and failed before (NK2302, then rustc E0716), 128 views/fn_type/trait/lambda/keep/lend/from/method and 111 text/literal/string/call tests ok, lower-std fixpoint, clippy clean
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: A trait method whose result says `from a` takes `a: String` as lent and lowers with one lifetime shared by its sources (#1230); a named function with a `String` parameter is covered as a function value (#1231).
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 7c7be0d24cf0527ce1fa0da4790913fb1eedbfb3
