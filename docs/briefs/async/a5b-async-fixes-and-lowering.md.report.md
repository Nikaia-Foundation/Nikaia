Result: done
Files: crates/nikaia-std/src/tools/throws.nika crates/nikaia-std/src/tools/lowered/throws.rs tests/compiler/tests/async_task_crash_is_in_the_throw_set.nika tests/compiler/tests/async_task_crash_is_in_the_throw_set.stdout tests/compiler/tests/async_group_raises_an_unread_failure.nika tests/compiler/tests/async_group_raises_an_unread_failure.stdout
Commit: async: a function that starts an async task throws `task::Crashed` in its inferred throw set
Body: `thrown_in` (throws.nika) adds `task::Crashed` to the direct set of every `Expr::Async` outside a `catch`, as std.contracts does for `task::scope` (Part II 12.7, ADR-async-5 D10), so the generated `__NikaiaThrows_…` sum gets `From<Crashed>` + `Joined` and is a task's channel. A function with no async task keeps its throw set. async_task_crash_is_in_the_throw_set (an inferred throws with a task that fails, caught by the caller) and async_group_raises_an_unread_failure (a5's last test, a failure nobody read arrives at the group's end) both failed on the binary before (a panic in render_refusal, and rustc `From<Crashed>` not satisfied). Commit 380e720f (before this one) carries a5's lowering and the fix for defect (b). No Rust function added or changed in this commit.
Part: #988
Tests: async_* --both-settings 32 passed (async_parse_fmt_formats_every_form fails only in a scratch copy, by the relative target/fast/nikaia path); tests/compiler --both-settings 645 passed, 7 failed only on "no such file or directory: target/fast/nikaia" (the scratch copy); it `task overlap spawn scope async` 74 passed; nikaia-std --lib task 14 passed; clippy clean; both defect tests failed on the binary before; no task::scope/spawn/overlap output changed
Gate: green
Deviation: defect (b) was in the lowering (emit_stmt_expr), not the parser, and went into commit 380e720f with a5's lowering; the order is reversed (a5 first, then fix 1) because fix 1's tests need a5's lowering
Found: crates/nikaia/src/emit/host.rs renders `EmitRefused.at` as `at as usize`, so a refusal with at = -1 panics in Span::new (PosOverflow); a5's async refusals now pass null; probe: any emit refusal built with `at: -1`
Needs: none
Changelog: Async tasks throw `task::Crashed`: a function that starts `async` tasks has `task::Crashed` in its inferred throw set, and `async e`, its binding and `async { … }` run on the group runtime, joined where the checker waits
Owner: none
Disk freed: 0 (test cache 3.3G, 8.2G free, no new entry over 100 MB)
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 6500f9e030b29ecaba30f304af01ea93cec08c53
