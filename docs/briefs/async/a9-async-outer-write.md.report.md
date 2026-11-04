Result: done
Files: docs/specification/30-nikaia-tooling.md
Commit: spec: Part III's NK2107 row names the async task (ADR-async-5 D7)
Body: The NK2107 row says, beside the par_iter() lambda, that an async task changing a name bound outside it is the same refusal whatever runs beside it, and adds a task that produces a value (`let n = async count()`) to the ways out. Step 1 is 521bfb09 (the checker's rule; Rust changed there: only hollow() in crates/nikaia/src/check/core.rs, one field line removed, for #557 / #125).
Fixes: #1229
Pushed: local bac28f2f45d55f9113b35cb89b6641fd2340a1bd
Tests: async_outer_write_a_total/_a_push accepted (printed) by the binary before 521bfb09, refused now; _only_reading_runs and _shared_mut_runs run; async_* and overlap_* 50 of 50 at --both-settings (no task_*/spawn_* tests exist); clippy, pre-push green; std, packages, examples and guide have no task that changes an outer name
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: An `async` task that changes a name bound outside it is refused with NK2107, whatever runs beside it (ADR-async-5 D7); reading an outer name and changing a `SharedMut` through `update` stay accepted.
Owner: none
Disk freed: none (cache 2.9G, / 5.6G free, no new entry over 100 MB)
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
