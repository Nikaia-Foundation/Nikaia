Result: done
Files: crates/nikaia-std/src/task/group.rs crates/nikaia-std/src/task.rs crates/nikaia-std/src/tools/emit_core_args.nika crates/nikaia-std/src/tools/emit_stmt.nika crates/nikaia-std/src/tools/emit_function_body.nika crates/nikaia-std/src/tools/emit_out.nika crates/nikaia-std/src/tools/emit_state.nika crates/nikaia-std/src/tools/lowered/emit_core_args.rs crates/nikaia-std/src/tools/lowered/emit_stmt.rs crates/nikaia-std/src/tools/lowered/emit_function_body.rs crates/nikaia-std/src/tools/lowered/emit_out.rs crates/nikaia-std/src/tools/lowered/emit_state.rs tests/compiler/tests/async_handle_dropped_live.nika tests/compiler/tests/async_handle_dropped_live.stdout tests/compiler/tests/async_handle_dropped_live_failing.nika tests/compiler/tests/async_handle_dropped_live_failing.stdout
Commit: A live child handle dropped unjoined is waited for at its block's end
Body: A store at group level fails borrowck for tasks that borrow the body's locals, so the rest of a bound pausing task named after its let now lives in a per-handle cell task::Rest<'b>, declared by the emitted code just before the handle's let (`let __rest_N = nikaia_std::task::Rest::new();`, after what the task borrows) and finished (`__rest_N.finish().await`) at the end of that block, after its tail (a value tail is bound first). The handle refers to the cell as &'g dyn Polled; join/join_within poll it, cancel()/on_timeout: Cancel drop it at the pause; a failure of the finished rest reaches the group as an unread one through the handle's existing Drop and settle. No new heap: the Box of the rest moves from the handle to the cell. The rest is waited for at the end of the block that declared it, which is the group's end in a function body; in a nested block it is that block's end, since borrowck allows no later point. Leaving the block early (return, throw, break) still drops the rest at its pause. Rust added: group.rs Polled (trait), Rest (struct; new, finish, Default, Polled::poll_rest, Polled::cancel_rest), ChildHandle::kept, field ChildHandle::kept; changed ChildHandle::new, cancel_here, join, join_within; task.rs Scope::start_kept (new), re-export of Polled and Rest. Nikaia: emit_async_task (start_kept where walk.async_keeps), emit_stmt (declares the cells), emit_stmt_block_plain and emit_function_body (finish the cells), EmitOut::insert_at, EmitWalk async_keeps/async_kept/async_rests.
Part: #987
Pushed: local 95df9561f579780ce14448839af4086a0aa947d6
Tests: async_handle_dropped_live and async_handle_dropped_live_failing both failed with the emitter change switched off (one build) and pass with it; --both-settings filters async_/task_/spawn_/overlap_ 72 passed 0 failed; nikaia-std --lib task:: 7 passed; clippy and fmt clean
Gate: green
Deviation: none
Found: a line that opens with `-1` after a call statement is read as a subtraction from it: `let n = run() catch {\n println("x")\n -1\n }` gives rustc "Cannot subtract {integer} from ()" (the test uses 100 instead)
Needs: none
Changelog: A live `ChildHandle` dropped unjoined is waited for at the end of its block, so a task outliving a `join(within:)` time-out runs to its end and its failure arrives as an unread one (Part I 8.2)
Owner: none
Disk freed: 0 (cache 2.8G, 3.7G free on /; new entry over 100 MB: run-bundle/c1ccf85af4afe566 212 MB)
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
