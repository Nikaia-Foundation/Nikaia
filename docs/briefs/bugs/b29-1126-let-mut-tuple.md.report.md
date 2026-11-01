Result: done
Files: crates/nikaia-std/src/tools/emit_stmt.nika crates/nikaia-std/src/tools/lowered/emit_stmt.rs tests/compiler/tests/a_let_mut_tuple_marks_every_name.nika tests/compiler/tests/a_let_mut_tuple_marks_every_name.stdout
Commit: parser: let mut (a, b) marks every name mutable
Body: The parser and checker already accepted `let mut (a, b)`. Both messages came from rustc, on the lowered Rust: `emit_stmt_let` wrote `let mut (a, b) = …`, which rustc refuses, and `for mut (a, b)` became `for (mut a, mut b)`, so rustc gave unused_mut for the `b` the body never assigns. Now the tuple `let` writes `#[allow(unused_mut)] let (mut a, mut b) = …`, and a `for`/pausing `while let` with a `mut` head of two or more names gets the same allow. A single `let mut x` / `for mut x` the body never changes still warns. The emitter has no per-name assignment facts, so a tuple head where no name is assigned also stays silent.
Fixes: #1126
Pushed: local 40944720e2250a324be84c1679be9d83e2dbb143
Tests: the new a_let_mut_tuple_marks_every_name failed on the unfixed binary (rustc: "`mut` must be attached to each individual binding") and passes now with --both-settings, in a scratch project with a_let_tuple_with_one_mut_name_is_not_supported_yet (still ok, NK1307). Hand probes: `for mut (a, b)` with the new binary prints `3` and gives no warning; `let mut x` never changed still warns. lower-std plus a fixpoint rebuild left only emit_stmt.rs changed. cargo fmt, clippy --all-targets and pre-push-check.sh are clean. Grepped `it` for `let mut (` / `for (mut `: nothing found.
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: Fixed: `let mut (a, b) = …` compiles and marks every name mutable, as NK1307's help says; a `mut` tuple head no longer warns about a name the body leaves alone.
Owner: none
Disk freed: none
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
