Result: done
Files: crates/nikaia-std/src/tools/check_arguments.nika crates/nikaia-std/src/tools/check_arm_leaves.nika crates/nikaia-std/src/tools/check_core_expr.nika crates/nikaia-std/src/tools/check_core_stmt.nika crates/nikaia-std/src/tools/check_numbers4.nika crates/nikaia-std/src/tools/check_types.nika crates/nikaia-std/src/tools/check_values.nika crates/nikaia-std/src/tools/lowered/check_arguments.rs crates/nikaia-std/src/tools/lowered/check_arm_leaves.rs crates/nikaia-std/src/tools/lowered/check_core_expr.rs crates/nikaia-std/src/tools/lowered/check_core_stmt.rs crates/nikaia-std/src/tools/lowered/check_numbers4.rs crates/nikaia-std/src/tools/lowered/check_types.rs crates/nikaia-std/src/tools/lowered/check_values.rs tests/compiler/tests/a_duration_is_not_an_integer_millis.nika tests/compiler/tests/a_duration_is_not_an_integer_millis.stdout tests/compiler/tests/a_duration_is_not_an_integer_parameter.nika tests/compiler/tests/a_duration_is_not_an_integer_parameter.stderr tests/compiler/tests/a_duration_is_not_an_integer_within.nika tests/compiler/tests/a_duration_is_not_an_integer_within.stderr
Commit: checker: an integer is never a `time::Duration` (or any other `std` type)
Body: An open number (a literal, or `let n = 20`) was not claimed by `no_number_is` (check_values.nika) when its slot was a type of a ledger, so `f(20)` for `d: time::Duration` passed the checker and reached rustc. `no_number_is`/`no_integer_is` now take the world and claim every type `world.library` declares (none of them is a number): the argument, assign, arm and open-number paths refuse it with their own code (NK1102, NK1105, ...). `async(within: 20)` typed the open literal as `?` and passed `expect_type`; `check_core_async_within` now types a number-shaped deadline as `i64`, so NK1102 says so. `way_out_of_a_mismatch` gives a number in a `time::Duration` slot the help `20.seconds()` / `20.millis()`, the open-number refusal too. Also refused now: `20` as a `time::Instant`, `fs::File`, ...
Fixes: #1246
Pushed: local 7f77b71df040ba4305d35cb97f2d053973991ff0
Tests: a_duration_is_not_an_integer_{parameter,within,millis} pass (both settings); the two refused ones pass the old checker (fail before); async_*/task_*/time_* 81 of 82 pass, async_parse_fmt_formats_every_form fails on the old binary too (hard-coded target/fast/nikaia path, see Found); examples/ and packages/ .nika (50 files) checked with old and new binary: same error counts; lower-std clean; cargo clippy -D warnings clean; sequences corpus guard not run (untouched)
Gate: green
Deviation: none
Found: tests/compiler/tests/async_parse_fmt_formats_every_form.nika (694f75a1) line 12 returns the fixed path "target/fast/nikaia", which the test process does not find (fails with "no such file or directory: target/fast/nikaia" on the binary before this change too; CI has no target/fast either, builder-facts trap)
Needs: none
Changelog: The checker refuses an integer where a `time::Duration` (or another `std` type) is expected, NK1102 with the help `20.seconds()` / `20.millis()`; it reached rustc before (#1246).
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
