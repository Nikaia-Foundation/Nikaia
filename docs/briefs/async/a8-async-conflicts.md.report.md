Result: done
Files: crates/nikaia-std/src/tools/check_core_expr.nika crates/nikaia-std/src/tools/check_core_ops.nika crates/nikaia-std/src/tools/check_lambdas.nika crates/nikaia-std/src/tools/check_takes.nika crates/nikaia-std/src/tools/lowered/check_core_expr.rs crates/nikaia-std/src/tools/lowered/check_core_ops.rs crates/nikaia-std/src/tools/lowered/check_lambdas.rs crates/nikaia-std/src/tools/lowered/check_takes.rs tests/compiler/tests/async_detached_takes_what_it_uses.nika tests/compiler/tests/async_detached_takes_what_it_uses.stderr
Commit: check: NK2101 names async(detached) for a detached task
Body: The Spawn arm passes written_async down to a_task_takes_these, which now takes a `who`. For async(detached) the message says "a detached task", and the note "A task started with `async(detached)`"; spawn keeps its wording. No .stderr changed: no test showed this note before; the new test shows both wordings. Step 1 is fc6eedcc (NK2107/NK2104 between the tasks of one group). Rust changed in this lane: only hollow() in crates/nikaia/src/check/core.rs (one field initialised, in fc6eedcc).
Fixes: #1227
Tests: the new test ran on the old binary with the spawn note for async(detached). All 46 async_/task_/spawn_/overlap_ programs pass --both-settings. Step 1's three refusals ran accepted on the old binary.
Gate: green
Deviation: none
Found: the lowering writes `fn f(state: ref CheckState, block: ref Block, mut out: Vec[ref Expr])` without a named lifetime, so rustc refuses `out.push(each)` ("lifetime may not live long enough", lowered/check_walks.rs); it should be refused in Nikaia or lowered with a lifetime. The step-1 collector builds owned records instead.
Needs: supervisor decision: ADR-async-5 D7's last sentence ("A task that changes a name bound outside it is NK2107") reads stricter than #1227 (refused only when another task of the group reads or changes it); built per the issue
Changelog: The tasks of one `async` group are checked against each other: two that change one name are NK2107, and two that write stdout or another resource are NK2104. NK2101 names `async(detached)` for a task started that way.
Owner: none
Disk freed: 0
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local ceabe4ac475f8f60cf13a8ed0a2f59b12cf0b1c5
