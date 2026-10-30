Result: partial
Files: crates/nikaia-std/src/tools/emit_core_args.nika crates/nikaia-std/src/tools/emit_exprs.nika crates/nikaia-std/src/tools/lowered/emit_core_args.rs crates/nikaia-std/src/tools/lowered/emit_exprs.rs tests/compiler/tests/a_kept_callback_is_handed_on.nika tests/compiler/tests/a_kept_callback_is_handed_on.stdout
Commit: emit: a kept fn value handed to code that keeps it is cloned (#1008)
Body: Defect 2 of #1008: an argument naming a function value (not a declared function) handed to a parameter its callee keeps (the method the receiver resolves to, else every candidate of the name) is written `.clone()`, beside ADR-312 D1's handle duplicate; a `Kept` is an `Arc`. Fixes the moved value twice, in a loop, and in a recursive call, also through a trait bound (`host.ask(known)`). The test uses free functions: the brief's trait rows still need defect 1, which is decided by the contract pass, not the emitter. No tools file hands a kept fn value on, so the lower-std fixpoint changes only the two edited files.
Part: #1008
Pushed: rejected da282119 (committed locally on develop, not rebased)
Tests: new tests/compiler a_kept_callback_is_handed_on (6, 6, 12, 9) failed on the unfixed compiler (rustc "use of moved value: known" at lines 15, 22, 32, 38); control rows (free ask_once twice / down) print 6 and 9; trait probe: the two moved-value errors are gone, E0049 and the direct-call mismatch remain (defect 1); lower-std fixpoint clean
Gate: green
Deviation: step 1's trait half (impl takes the trait's keeps) and the brief's test a_trait_method_callback_is_handed_on not built: stop condition, the decision is outside tools/emit_*.nika (see Found)
Found: defect 1 of #1008 is decided by crates/nikaia/src/contracts/keeps.rs `infer`: a trait method's graph entry keeps its by-value parameters (the `Item::Trait` arm), but an impl's method gets `keeps` from its own body only (`uses_of`), so `Plain::ask` keeps nothing; that one contract column drives the emitter's signature (emit_function.nika ~565, `impl Fn`) and the checker's call-site facts (check_core_args.nika `check_core_args_runs` -> `kept_args`: `p.ask(known)` lowers `&&*known`). Fix: the impl method of a trait unions the trait method's keeps (by parameter position) in `infer`; then the emitter needs no change for it. Probe: the brief's program; after this commit only E0049 at the impl's `fn ask` and the mismatch at `p.ask(known)` remain. Also seen: `fn f(h: ref Holder) { let g = h.f; g("a") }` with a `fn` field passes the checker and rustc refuses "cannot move out of h.f behind a shared reference" (calling `h.f("a")` works)
Needs: owner decision (or a brief allowing the edit of contracts/keeps.rs for defect 1)
Changelog: a function value handed more than once to code that keeps it (twice, in a loop, recursively) now compiles
Owner: none
Disk freed: 0.02 GB (scratch binary copy); test cache 3.8G, disk 4.1G free of 252G, no new entry over 100 MB
Rs2nika: none
