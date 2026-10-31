Result: blocked
Files: none
Commit: none
Fixes: none
Pushed: none
Tests: preconditions on develop (b1012a 1c72e8b9); run3.patch applied; lowered trait now takes `&impl Fn` (no Kept), as b1012a meant; `lower-std` clean, but `cargo check -p nikaia-std --features compiler` fails on lowered/comptime_tree.rs (defects 1 and 2 below), and defect 3 would fail at the first build of `nikaia`. Run 4's draft (run3 plus the inner `known` lambda left untyped, so b997 types it from where it is used) saved as docs/briefs/k3a2-comptime-compute.run4.patch (untracked, `git apply --check` ok). My files put back by name, nothing committed
Gate: none
Deviation: none
Found: three lowering defects in `tools/*.nika` / Nikaia code that hands on `fn` parameters: (1) a let-bound lambda with a declared type (`let not_here: fn(ref String) -> CheckComputed sync = fn(why) {..}`, no captures) lowers to `Kept<dyn Fn(&str) -> CheckComputed + Send + Sync>`, and inside `tools` the name `Sync` resolves to `ty.nika`'s `pub enum Sync`: E0404 "expected trait, found enum `Sync`" (lowered/comptime_tree.rs:473; the emitter should write `std::marker::Sync`). (2) An untyped let lambda that shadows the `fn` parameter it reads (`let known = fn(name) { known(name) ?? ahead.get(name)?.clone() }`, then `return comptime_computed(host, .., known, typed)`) is handed on as `known`, not `&known`: E0308 "expected `&_`, found closure" (lowered :513); a probe without the shadowing writes `&more` correctly. (3) A recursive function whose `fn` parameter lowers as `&impl Fn` and that hands itself a new lambda recurses without end at monomorphisation. Probe (`nikaia run`): `fn alt(name: String) -> i64? sync { return name.len() }` / `fn deep(n: i64, known: fn(ref String) -> i64? sync) -> i64 sync { if n == 0 { return known("a") ?? 0 }; let more = fn(name) { known(name) ?? alt(name) }; return deep(n - 1, more) }` / `fn main() { println(f"{deep(2, fn(name) { null })}") }` gives "Reached the recursion limit while instantiating `deep::<{closure}>`". The Rust being moved uses `&dyn Fn` for this reason; `comptime_computed` and `comptime_worked_out_ahead` need it (run-shape `fn` parameter lowered `&dyn Fn`, at least where the function calls itself)
Needs: fix of defects 1-3 (lowering, outside this brief's files); then the next run starts from run4.patch
Changelog: none
Owner: none
Disk freed: 0 GB; cache target/nikaia-project-tests 4.0G, disk 3.7G free; no new cache entries over 100 MB
Rs2nika: none
