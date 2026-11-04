Result: done
Files: tests/compiler/tests/async_binding_refusal_stands_at_its_let.nika tests/compiler/tests/async_binding_refusal_stands_at_its_let.stderr
Commit: async: a binding's refusal stands at its let
Body: Step 1 (05caf996) gave the walk of a binding's `async e` the `let`'s span (`emit_async_binding`: `flow.at(span.start)`), and so ended the refusal at position -1 that panicked in render_refusal. A pausing task no longer meets that refusal at all. The tuple refusal is the one left that stood nowhere. This test shows it at its `let`. Rust changed in this run, all in step 1: task.rs (`Scope::new`, `Scope::start_paused`, `Scope::spawn_paused_here`, `caught_paused`, `group_paused`, `group_paused_plain`) and task/group.rs (`ChildHandle::held`, `held_task`, `ChildHandle::join`).
Fixes: #1226
Tests: on scratch copies (--both-settings), the 44 async_*/group_scope/overlap/task:: programs plus the 4 new ones pass. parser_grammar_words fails there only because it reads a repo path. There are no task_* tests. The new .stderr test fails on the old binary (it gives no site) and the async_pausing_* tests fail there too. Both #1226 probes print the right values. nikaia-std --lib task:: passes 7. clippy is clean.
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: async: a task whose call pauses builds and runs. It starts in place, and where it is waited for it is polled to its end (ADR-async-5 D20, #1226).
Owner: none
Disk freed: 0
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local 1a410df3fbb50b2561c300dfa103b17852502799
