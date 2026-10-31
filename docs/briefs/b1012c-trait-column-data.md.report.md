Result: blocked
Files: none
Commit: none
Fixes: none
Pushed: none
Tests: built with the seed removed (keeps.rs, diff kept at scratchpad b1012c/keeps.diff): ledger column of `Measure::size(ref self, text: String)` became empty, yet lower-std changed no lowered file and the probe test (scratchpad b1012c/test.nika) still fails NK2105/NK1102 exactly as on the unfixed compiler; change undone, binary rebuilt from develop
Gate: none
Deviation: step 1 not built (premise wrong: the seed in keeps.rs is not what keeps a trait method's data parameter)
Found: no method lends a by-value data parameter whatever its `keeps`: `lends_with` in tools/lends.nika:51 returns lent only for `!signature.takes_a_receiver() || (own && a_function_value(ty))`, and `emit_arg_only_read` in tools/emit_core_args.nika:192 returns false for any receiver; probe `struct Count{base:i64} impl Count{fn size(ref self, text: String)->i64 sync{return self.base+text.len()}} fn main(){let c=Count{base:1}; let own=f"{c.base}x"; println(c.size(own)); println(own)}` refuses NK2105 (inherent method, empty keeps); same for a trait method under a bound
Needs: owner decision: does D17's "every by-value parameter" lift the receiver exclusion for trait methods and their impls only (FnContract has no trait flag; checker, emitter and the Rust host impls follow) or for every method (ADR-094 D2 reads the body of any callee; widens to all inherent methods of tools/*.nika and their Rust callers)
Changelog: none
Owner: lift the method exclusion of lends_with for trait methods only, or for all methods?
Disk freed: 0 GB (target/nikaia-project-tests 1.8G, / 4.7G free; no cache entries created)
Rs2nika: none
