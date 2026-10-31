Result: done
Files: crates/nikaia-std/src/tools/keeps.nika crates/nikaia-std/src/tools/lowered/keeps.rs tests/compiler/tests/a_returned_lambda_keeps_its_fn_parameter.nika tests/compiler/tests/a_returned_lambda_keeps_its_fn_parameter.stdout
Commit: keeps: a lambda handed out keeps the fn parameters it names (#1034)
Body: `hand_over` (a `return`, an assignment, a body's last value) and the struct, list and tuple literal fields now also keep every parameter a lambda among them names (`keeps_named_by_a_lambda`, `holes_of` threaded through `hand_over`). Rows a and b, and the `twice` function of row c, lower and run. Row c's caller side has a second cause and is left out of the test: a `let`-bound local lambda handed to a kept parameter (`let add = fn(x) { x + 3 }` then `keep(add)`, even with `fn keep(f) { return f }`) reaches rustc as a bare closure where `Kept<dyn Fn>` is expected, and a use of `add` after that is NK1245; both happen on the unfixed compiler too. So `twice` takes an inline lambda in the test, which prints 3, 7, 7.
Part: #1034
Tests: new test passes (nikaia run, nikaia test --both-settings) and failed on the unfixed binary (rustc at lines 6, 10, 14); a_kept_lambda_calls_a_captured_fn and a_kept_fn_copied_from_a_name_or_field pass; `fn apply(f, x) { return f(x) }` still lowers `f: &impl AsyncFn(i64) -> i64`; it guard set + `keeps` filter 156 passed; clippy clean; pre-push-check ok; lower-std fixpoint clean; tests/compiler --both-settings 424 passed, 6 failed, none mine: 5 found target/fast/nikaia missing mid-run (another agent rebuilding), 1 another agent's untracked a_bit_operation_in_a_head_wants_parentheses
Gate: green
Deviation: row c left out of the test (second cause, see Found); `Part: #1034` instead of `Fixes`
Found: a `let`-bound local lambda handed to a kept fn parameter fails in rustc (probe: `fn keep(f: fn(i64) -> i64) -> fn(i64) -> i64 { return f }` `fn main() { let add = fn(x) { x + 3 }; let t = keep(add); println(f"{t(1)}") }` gives `Mismatched types` at `let t = keep(add)`: expected `Kept<dyn Fn(i64) -> Pin<Box<dyn Future>>>`, found closure); adding `println(f"{add(1)}")` after it is NK1245 at check; also `fn mk(k: i64) -> fn(i64) -> i64 { let g = fn(x) { x + k }; return g }` is rustc `Mismatched types` at `return g`; all on the unfixed compiler
Needs: none
Changelog: A function that hands out a lambda capturing one of its `fn` parameters (returned, assigned, or put in a struct, list or tuple) keeps that parameter, instead of a lowering rustc refused.
Owner: none
Disk freed: 0.04 GB (two binary copies in the scratchpad); cache 3.3G, disk 6.7G free; new cache entries over 100 MB: rlib/af187e66 265M, run-bundle/2df20709 286M, build-time-parsers/768d2772 1026M, build-time-bundle/b95bc86d 484M
Rs2nika: none
Pushed: 650a4ef486524e982d512800b3175715281024b9
