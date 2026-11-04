Result: partial
Cause: none found. Only red: arm64 job, `dsl_grammar::text_the_grammar_refuses_is_refused_at_the_block` (2886 passed, 1 failed; run 41746282250). `findings` came back `[]` where NK1178 was expected.
Fixed: nothing; no commit (nothing half-made).
Tests: that test passes here at HEAD, alone and with the whole `dsl_grammar` module on a cold `NIKAIA_CACHE_DIR`; it passed on arm64 in the two develop runs before (41733893156, 41733702585). `bootstrap_compiler` and `changes` were green on this run.
Ruled out: the 9 unpushed commits do not touch the path. Their diff is parser `async` forms, NK2115 in `parse_findings.nika`, NK2120 in `check_places`/`check_core_expr` (`Expr::Spawn` only), tests and briefs. The path itself is `check_core_a_block_over_a_grammar` -> `check_core_comptime_binding` -> `Host::run_grammar` (`crates/nikaia/src/build_time.rs`) -> `Workshop::run`.
Suspect: an arm64 CI flake in the grammar workshop, a shared build-time-parsers dir under load. I did not confirm it. A refused run should give NK1178 in any case, so empty findings suggest a silent path (e.g. `bt_not_folded` when `ran.found` is false, or `comptime_binding` returning early).
Probe: rerun the arm64 job. If it fails again, run `cargo test -p nikaia --test it dsl_grammar` there with `RUST_BACKTRACE=1` and print `found` and `wall` in `Host::run_grammar`.
Found: possible flaky silent skip of a grammar run (above), not reproduced; not from these commits. Note: my own cold-cache run of dsl/grammar tests filled the disk (2.3G free, 95%) and caused unrelated "No space left" failures; I deleted the scratch cache.
Files: none
Commit: none
