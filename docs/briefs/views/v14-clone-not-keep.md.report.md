Result: done
Files: crates/nikaia-std/src/tools/keeps.nika crates/nikaia-std/src/tools/lowered/keeps.rs tests/compiler/tests/a_cloned_generic_parameter_is_not_kept.nika tests/compiler/tests/a_cloned_generic_parameter_is_not_kept.stderr tests/compiler/tests/a_clone_of_a_lent_field_is_handed_back.nika tests/compiler/tests/a_clone_of_a_lent_field_is_handed_back.stdout
Commit: A cloned parameter is not a keep
Body: The keep inference (keeps.nika keep_receiver) counted a receiver as moved when the body's method calls did not resolve or no entry described the method; `x.clone()` of an unbounded `T` does not resolve (NK1126), so `id` was said to keep `x` (NK2302). `.clone()` reads its receiver and makes a copy whose result reaches nothing of it (ADR-views-3 D8, D12), so for `clone` that fail-closed fallback no longer applies; an entry that does take its receiver by value still counts. No Rust changed. The brief's `T: Clone` probe cannot be accepted yet: Nikaia has no `Clone` trait (NK1135, NK1126 says bounds are not available yet), a missing form for the supervisor; a user trait bound (`T: Twin` with `fn clone(ref self) -> Self`) shows a second defect, `x.clone()` typed `Self`, not `T` (NK1104).
Fixes: #1243
Tests: tests/compiler --both-settings filters clone (6), keep (43), hand_back/handed_back (10), is_a_view (6, v13's), still_keeps (1), generic (4) all ok; the unbounded test fails on the old binary (extra NK2302), the lent-field test passed there already (concrete types were not affected); it keeps:: views_kept:: tethers:: result_source:: 133 ok; clippy, fmt green
Gate: green
Deviation: the brief's test with `T: Clone` accepted (prints 3) is not built: no `Clone` trait exists (NK1135)
Found: in a file with `trait Twin { fn clone(ref self) -> Self }`, `fn id[T: Twin](x: T) -> T { return x.clone() }` is NK1104 "returning `Self`, but declared to return `T`": `Self` of a bound's method is not substituted with `T`
Needs: supervisor decision: a `Clone` trait (or built-in bound) usable as `T: Clone`, so `fn id[T: Clone](x: T) -> T { return x.clone() }` type-checks
Changelog: A cloned parameter is not a keep: a body that returns `x.clone()` of a generic parameter no longer gives NK2302 "keeps `x`"; a clone is a fresh value (ADR-views-3 D8, D12, #1243).
Owner: none
Disk freed: 0 (cache 2.8G, 3.6G free on /, new cache entry run-bundle/c1ccf85af4afe566 212 MB)
Rs2nika: none
Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local ee2da0dc0680953a78733ddf714fbcbe1f5591d5
