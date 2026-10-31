Result: done
Files: crates/nikaia-std/src/tools/views.nika crates/nikaia-std/src/tools/emit_core_types.nika crates/nikaia-std/src/tools/lowered/views.rs crates/nikaia-std/src/tools/lowered/emit_core_types.rs tests/compiler/tests/a_pausing_two_view_field.nika tests/compiler/tests/a_pausing_two_view_field.stdout
Commit: emit: a struct field holding pausing code with two views in and a view out names the call's lifetime
Body: views_the_call_s no longer asks for `sync`: a function type handed two views or more that gives one back hands back the call's own view whether it may pause or not, so holds_view and the two names_borrowing walks give a struct no `'a` for such a field. The kept branch of emit_core_types writes it `Kept<dyn for<'call> Fn(&'call str, &'call str) -> BoxedIn<'call, &'call str>>` (the `lent > 1` block, beside the sync form). Row b (struct built, never called) also failed in rustc before the fix, contrary to the brief; the new test holds both rows.
Fixes: #1061
Tests: before (suite binary): row a rustc Lifetime may not live long enough; row b same error (did not print 1); row c printed 4; ref Vec[i64] variant fails differently (args not borrowed, also with sync), so not in the test. After: a 4, b 1, c 4. New test a_pausing_two_view_field fails on the unfixed binary (two Lifetime errors), passes; tests/compiler --both-settings: 472 passed, 0 failed (all of function_type, two_view, code_param, kept, struct_field, lambda among them); lower-std fixpoint clean; build, fmt, clippy 1.97 and 1.99.1 (nikaia-std tools), pre-push-check green
Gate: green
Deviation: none
Found: a field holding code over `ref Vec[i64]` (`struct H { f: fn(ref Vec[i64], ref Vec[i64]) -> ref Vec[i64] }`, `h.f(x, y)` with `let x = [1, 2, 3]`) reaches rustc with *Arguments to this function are incorrect* (Expected `&[i64]`, found `Vec`), also when the field is `sync`: the call through a field passes the lists unborrowed
Needs: none
Changelog: A struct field holding code that may pause, handed two views and giving one back, compiles: its result is the call's view (#1061).
Owner: none
Rs2nika: none
Disk freed: clippy-1.99 target dir removed; project-tests cache 3.4G, disk 9.7G free, no new entry over 100 MB
Pushed: local ccfbc205cd1b4b52fdc0d9bd61a0fc6060fe8361
