Result: done
Files: crates/nikaia-std/src/tools/project_check.nika crates/nikaia-std/src/tools/lowered/project_check.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/lib.rs crates/nikaia/src/project.rs crates/rs2nika/convert/rs2nika.names docs/selfhosting-plan.md
Commit: project: the check's tally and the explanations into Nikaia (P5d)
Body: `project_check.nika` takes the check's NK1238 retain, the order, the three filters, the printing loop and the tally (`project_check_check` over `ProjectCheckHost`, said as `ProjectCheckSaid`), the throws each callee gained and the `keeps` that moved (`project_check_gained`, `project_check_keeps_that_moved`, `ProjectCheckKept`; `arguments()` is `params` after a `self`), and the order of `--explain`'s reports (`project_check_explain` over `ProjectCheckExplainHost`, one `explain_report` arm per kind). Stays Rust: `Around`, `Explain`/`asked`, the walks, renderer and reports behind the two hosts (`CheckHost`, `ExplainHost` in project.rs, `RefCell` for the facts and the violations), `write_proofs`, `write_ledger`, `changed_lines`, `foreign_library`; `Project::report` goes to P5c. The tools module allows `clippy::map_clone` (a `.map(fn(x) { x.clone() })` lowers as written). rs2nika.names gets `[forms project_check_]` and `[hosts project_check_]`. Self-hosting share 82.2 %.
Part: #125
Tests: it filters project check newly keeps explain asserts sharing overlap tether bounds trust compile_time_reads prove build_time sequences (765 passed); --lib project:: (10 passed); nikaia test --both-settings --project tests/compiler (245 passed); clippy clean; pre-push-check ok; lower-std fixpoint (only lowered/project_check.rs and the index of mine)
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: about 2.2 GB (target/nikaia-project-tests removed under 3 GB free; now 5.5 GB free, no cache entry over 100 MB left)
Rs2nika: kept 12 of 79 lines; costliest: the [forms project_check_]/[hosts] rows changed nothing on re-convert (even the global `{x}.span.at()` row is not applied by this binary, probe `fn f(a: &X) -> i64 { let c = x.span.at(); 3 }` under --file and the tree engine); signatures kept Rust's arguments (parsed, own, Around, facts_out) instead of the host; map loops over `inferred.functions.iter().filter_map` with `?` in the closure (maps have no `.iter()` in Nikaia: an outer `for (name, contract)` loop, the inner chain kept)
Pushed: 0f9a525aab78434beea986dc40b0187df38f9e30
