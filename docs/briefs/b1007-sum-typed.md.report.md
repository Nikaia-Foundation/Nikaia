Result: blocked
Files: none
Commit: none
Fixes: none
Pushed: none
Tests: none run; nothing built. Read emit_core_calls.nika:1175-1185, check_core_method.nika:555-580 (check_core_method_called), check_facts.nika fact maps.
Gate: green
Deviation: none
Found: the emitter has no fact carrying a call's checked result type. `value` (Ty) is known in check_core_method_called after check_core_call_on, but only `std_receivers` (ledger type of the receiver, Bytes/numbers only), `open_items` (open literals only, #931) and `collected_into` reach emit; none gives `i64` for `v.iter().sum()` with `v: Vec[i64]`. Fix needs a new fact in check_facts.nika (e.g. `summed: BTreeMap[i64, String]`, filled in check_core_method_called when method == "sum" and value is a plain number type) and emit_core_calls.nika pushing `::<T>` after `.sum` when the call is not declared (`collected_into`-style check for `let t: i64`). Edits to check_facts.nika, check_core_method.nika and their lowered files are outside the brief's permitted set.
Needs: brief widened to allow check_facts.nika + check_core_method.nika (+ lowered) edits; then one agent builds both sides.
Changelog: none
Owner: none
Disk freed: 0 GB (no clone, no build)
Rs2nika: none
