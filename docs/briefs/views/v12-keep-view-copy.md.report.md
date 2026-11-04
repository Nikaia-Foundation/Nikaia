Result: partial
Files: crates/nikaia-std/src/tools/check_core_ops.nika crates/nikaia-std/src/tools/lowered/check_core_ops.rs tests/compiler/tests/views_arithmetic_on_a_lent_copy_is_owned.nika tests/compiler/tests/views_arithmetic_on_a_lent_copy_is_owned.stdout tests/specification/EXPECTED.txt tests/specification/COMPILES.txt
Commit: views: an operator on a lent value that copies gives a value of its own
Body: Step 2 only (#1240). check_core_ops_outcome marks a lent operand of a copying non-number type (a lent `self` of time::Duration) in compared_views, so the emitter writes `*self * 2`, and types an arithmetic result unviewed (unviewed_type), so `return self * 2` is no NK1104. New helper check_core_ops_a_lent_copy (Nikaia). No hand-written Rust function added or changed; only the lowered check_core_ops.rs. Spec block 10-nikaia-light #62 now lowers and compiles; EXPECTED.txt and COMPILES.txt regenerated.
Fixes: #1240
Tests: target/fast/nikaia test --both-settings --project tests/compiler --filter views_arithmetic_on_a_lent_copy (1 passed; refused NK1104 on the unfixed compiler); covers the probe and an i64 field read through a lent self (that case already worked before); cargo test -p nikaia --test it -- specification:: 5 passed; lower-std fixpoint clean; fmt, clippy clean
Gate: green
Deviation: step 1 (#1239) not built: returning a whole parameter as a lent view needs a non-text view result (`fn hand<T>(x: &T) -> &T`) the lowering lacks; only flipping the inference makes x `&T` with a `T` result, which rustc refuses
Found: (1) #1239 is a missing form: no non-text result can be a view of a parameter (D8/D12); probe in a file without keep: `fn first(x: Vec[Vec[i64]]) -> Vec[i64] { return x[0] }` reaches rustc as "cannot move out of a shared reference" (lowered `fn first(x: &[Vec<i64>]) -> Vec<i64> { *get(&x,0) }`); fixing it needs the checker to type the call's result as a view, the emitter to write `&'a T`, callers owning it to clone or be refused. (2) the #1240 probe's `time::Duration::from_millis(3)` is NK1245 and `.as_millis()` NK1171 (std has `3.millis()`, `.in_millis()`); the test uses those
Needs: supervisor decision on non-text view results for #1239 (D8/D12 lowering)
Changelog: An arithmetic operator on a lent value of a copying type (`self * 2` in a method of `time::Duration`) gives a value of its own and lowers with the value read (#1240).
Owner: none
Disk freed: none
Rs2nika: none

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GFeMqxRbtUMiGVyDMxsvEq
Pushed: local e1d4dee097264f121fb9b2dda7a2c9a026af2979
