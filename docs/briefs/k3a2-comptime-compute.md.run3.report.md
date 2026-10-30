Result: blocked
Files: none
Commit: none
Fixes: none
Pushed: none
Tests: preconditions on develop (#1008 d1ff2ec3/a5bfee04); run2.patch applied, then typed `not_here: fn(ref String) -> CheckComputed sync` and `let known: fn(ref String) -> BuildValue? sync`, and marked `comptime_computed`/`comptime_worked_out_ahead` `sync` (they lowered `async` with `Box::pin`); lower-std clean, `cargo check -p nikaia-std` clean. Saved as docs/briefs/k3a2-comptime-compute.run3.patch (untracked, `git apply --check` ok). My files restored by name, nothing committed
Gate: none
Deviation: none
Found: none
Needs: owner decision: a body-less trait method's `fn` parameter is always kept (#1008, ADR-277 D11). So `ComptimeComputeHost::sub_program`, `unit_without_its_constants` and `folded` lower to `nikaia_std::func::Kept<dyn Fn(&str) -> Option<BuildValue> + Send + Sync>`, which is `'static`, and `comptime_computed` takes `known`/`typed` the same way. `compute` gets `&dyn Fn` closures that borrow the callers' maps (check/host.rs `compute_comptime`: `|name| known.get(name).cloned()` over `&BTreeMap`), and Rust cannot turn those into a `'static` `Arc`. Probe: `trait H { fn ask(ref self, known: fn(ref String) -> i64? sync) -> i64? sync }` lowers to `fn ask(&self, known: Kept<dyn Fn(&str) -> Option<i64> + Send + Sync>)`; a Rust impl called from `fn f(k: &dyn Fn(&str) -> Option<i64>)` gets "borrowed data escapes". No ADR covers lending a callback through a trait method, and `ref fn(..)` does not parse (run 2). Two ways out for the owner: (a) a way for a trait method to lend a `fn` parameter (run shape, `&dyn Fn`); (b) let this move edit the two callers (check/host.rs:276, contracts/mod.rs:1277) so they hand `Kept` closures over owned copies of the maps, with `compute` taking `Kept`. Either way the next run starts from run3.patch
Changelog: none
Owner: may the move lend a callback through a trait method (new form, a) or should the callers hand owned `Kept` closures (b, edits outside comptime_run.rs)?
Disk freed: 0 GB; cache 4.0G, disk 4.0G free; run-bundle/b2db8b4e959f1cfc (347 MB) is newer than the brief but not from this run (I ran no tests)
Rs2nika: none
