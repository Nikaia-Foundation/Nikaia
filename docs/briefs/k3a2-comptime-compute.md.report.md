Result: done
Files: crates/nikaia-std/src/tools/comptime_tree.nika crates/nikaia-std/src/tools/lowered/comptime_tree.rs crates/nikaia-std/src/tools/lowered.rs crates/nikaia/src/comptime_run.rs docs/selfhosting-plan.md
Commit: comptime: computing a build-time constant moves into Nikaia (K3a part 2)
Body: `computed` and `worked_out_ahead` are `comptime_computed` and `comptime_worked_out_ahead` in comptime_tree.nika (from run4.patch, unchanged), over the trait `ComptimeComputeHost`; Rust's `ComputeHost` keeps the program's files, an arena of the units it made (named by index), the library, the workshop and the bounds, and `compute` hands it over inside the defaults adapter. `NOT_LOWERED`, `NOT_READ`, `NOT_CARRIED` are `COMPTIME_*` consts and `NOWHERE` a const in Nikaia; the Rust re-exports of `NOWHERE` and `NOT_COMPUTED_YET` went, since nothing in Rust reads them now (clippy flags an unused re-export). `lowered_units` takes `&[&Parsed]` so the host lowers units out of its arena, and `without_readers` is split so the host drops readers over the arena's units in their file order. The K3a plan row also carries part 1's record, which was in the working tree uncommitted.
Part: #125
Pushed: 829ce6354106baf7de078ef9e3100baf568f19f0
Tests: lower-std fixpoint changes only lowered/comptime_tree.rs and its lowered.rs index line; cargo check -p nikaia-std --features compiler clean; it `comptime grammar_at_build_time` 59 passed; tests/compiler --both-settings 398 passed 0 failed; guard set 24 passed (sequences ok); clippy clean; pre-push-check ok
Gate: green
Deviation: none
Found: none
Needs: none
Changelog: none
Owner: none
Disk freed: 0 GB (scratch binary copy removed); cache target/nikaia-project-tests 1.8G, disk 4.8G free; new entries over 100 MB: rlib df8bcfe6 265, rlib af187e66 265, run-bundle b2db8b4e 286, build-time-parsers 768d2772 519, build-time-bundle dc4d30d1 484 MB
Rs2nika: kept 16 of 87 lines; costliest: (1) the draft's direct calls of Rust helpers over `Parsed` (`sub_program`, `constants_of`, `by_package`, `emit_program`/`dumper`/`with_driver`) became `host.` calls by unit index, `driven` folding three steps into one enum; (2) the `let ... else` and `if let` on `Wall`/`Option` became `match` over `ComptimeRan`/`ComptimeDriven`; (3) `worked_out_ahead`'s walk of `Item::Comptime` became `host.comptimes_of(at)` with `folded`/`ty_of`
